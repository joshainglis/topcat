//! PostgreSQL pg_dump file parser implementing ImportSource.
//!
//! This module provides the [`PgDumpParser`] struct that implements the
//! [`ImportSource`] trait, producing [`RawObject`] instances from pg_dump output.

use std::fs;
use std::path::PathBuf;

use regex::Regex;

use super::patterns::{
    ALTER_TABLE_PATTERN, DEFAULT_ACL_PATTERN, EVENT_TRIGGER_PATTERN, EXTENSION_PATTERN,
    FOREIGN_TABLE_PATTERN, FUNCTION_PATTERN, GRANT_PATTERN, INDEX_PATTERN, METADATA_PATTERN,
    OWNER_PATTERN, POLICY_PATTERN, REVOKE_PATTERN, RULE_PATTERN, SERVER_PATTERN,
    STATISTICS_PATTERN, SUBSCRIPTION_PATTERN, TABLE_PATTERN, TRIGGER_PATTERN, USER_MAPPING_PATTERN,
};
use super::{DEFAULT_SCHEMA_PATTERN, build_cast_pattern, build_operator_pattern};
use crate::commands::import::object_types::ObjectType;
use crate::commands::import::sources::{ImportSource, RawObject, SecurityStatement};
use topcat::exceptions::TopCatError;

/// Parser for PostgreSQL pg_dump output files.
///
/// Implements [`ImportSource`] to extract database objects and security
/// statements from pg_dump output. Populates `extracted_deps` on objects
/// during parsing using source-specific patterns.
///
/// # Example
///
/// ```ignore
/// let mut parser = PgDumpParser::from_file(path, None)?;
/// let objects = parser.extract_objects()?;
/// let security = parser.collect_security()?;
/// ```
pub struct PgDumpParser {
    /// Raw content of the dump file.
    content: String,

    /// Pattern for matching CAST statements.
    cast_pattern: Regex,

    /// Pattern for matching OPERATOR statements.
    operator_pattern: Regex,

    /// Cached security statements (populated lazily).
    cached_security: Option<Vec<SecurityStatement>>,
}

impl PgDumpParser {
    /// Create a parser from file path.
    pub fn from_file(
        dump_file: PathBuf,
        schema_pattern: Option<String>,
    ) -> Result<Self, TopCatError> {
        let content = fs::read_to_string(&dump_file)?;
        Ok(Self::from_content(content, schema_pattern))
    }

    /// Create a parser from string content.
    pub fn from_content(content: String, schema_pattern: Option<String>) -> Self {
        let pattern = schema_pattern.unwrap_or_else(|| DEFAULT_SCHEMA_PATTERN.to_string());
        let cast_pattern = build_cast_pattern(&pattern);
        let operator_pattern = build_operator_pattern(&pattern);

        Self {
            content,
            cast_pattern,
            operator_pattern,
            cached_security: None,
        }
    }

    /// Parse content and extract all objects.
    fn parse_objects(&self) -> Vec<RawObject> {
        let lines: Vec<&str> = self.content.lines().collect();
        let mut objects = Vec::new();

        let mut current_metadata: Option<ParsedMetadata> = None;
        let mut current_content: Vec<&str> = Vec::new();

        for line in &lines {
            // Check for metadata comments
            if let Some(caps) = METADATA_PATTERN.captures(line) {
                let type_str = caps.name("type").map(|m| m.as_str().trim()).unwrap_or("");
                let obj_type = ObjectType::from_pg_dump_type(type_str);

                // Skip COMMENT type entries in metadata - they're handled inline
                if obj_type == ObjectType::Comment {
                    continue;
                }

                // Save previous object if exists
                if let Some(meta) = current_metadata.take() {
                    let content_str = current_content.join("\n");
                    if let Some(obj) = self.create_raw_object(meta, content_str) {
                        objects.push(obj);
                    }
                }

                // Start new object
                let target_type = caps.name("tgt_type").map(|m| {
                    let t = m.as_str().trim();
                    if t == "COLUMN" { "TABLE" } else { t }.to_string()
                });

                current_metadata = Some(ParsedMetadata {
                    name: caps
                        .name("name")
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                    identity: caps
                        .name("identity")
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                    schema: caps
                        .name("schema")
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_default(),
                    obj_type,
                    pg_type_str: type_str.to_string(),
                    target_type,
                    owner: caps.name("owner").map(|m| m.as_str().trim().to_string()),
                });
                current_content.clear();
            } else if current_metadata.is_some()
                && !line.starts_with("--")
                && !line.trim().is_empty()
            {
                current_content.push(line);
            }
        }

        // Save last object
        if let Some(meta) = current_metadata {
            let content_str = current_content.join("\n");
            if let Some(obj) = self.create_raw_object(meta, content_str) {
                objects.push(obj);
            }
        }

        objects
    }

    /// Create a RawObject from parsed metadata and content.
    fn create_raw_object(&self, meta: ParsedMetadata, content: String) -> Option<RawObject> {
        // Handle schema-less objects
        let schema = if meta.schema == "-" {
            match meta.obj_type {
                ObjectType::Schema => Some(meta.name.clone()),
                ObjectType::Extension => {
                    // Extract schema from content
                    if let Some(caps) = EXTENSION_PATTERN.captures(&content) {
                        Some(
                            caps.name("ext_schema")
                                .map(|m| m.as_str().to_string())
                                .unwrap_or_else(|| "public".to_string()),
                        )
                    } else {
                        Some("public".to_string())
                    }
                }
                // Global objects
                ObjectType::ForeignDataWrapper
                | ObjectType::Server
                | ObjectType::Language
                | ObjectType::EventTrigger
                | ObjectType::Publication
                | ObjectType::Subscription
                | ObjectType::AccessMethod
                | ObjectType::Cast
                | ObjectType::OperatorClass
                | ObjectType::OperatorFamily
                | ObjectType::Operator => None,
                _ => Some("public".to_string()),
            }
        } else {
            Some(meta.schema.clone())
        };

        // Determine name (may be extracted from content for some types)
        let name = if meta.obj_type == ObjectType::Extension {
            if let Some(caps) = EXTENSION_PATTERN.captures(&content) {
                caps.name("ext_name")
                    .map(|m| m.as_str().to_string())
                    .unwrap_or(meta.name.clone())
            } else {
                meta.name.clone()
            }
        } else {
            meta.name.clone()
        };
        let name = if name.is_empty() {
            match meta.obj_type {
                ObjectType::Table => TABLE_PATTERN
                    .captures(&content)
                    .and_then(|caps| caps.name("name").map(|m| m.as_str().to_string()))
                    .unwrap_or(name),
                ObjectType::Function => FUNCTION_PATTERN
                    .captures(&content)
                    .and_then(|caps| caps.name("name").map(|m| m.as_str().to_string()))
                    .unwrap_or(name),
                _ => name,
            }
        } else {
            name
        };

        let mut obj = RawObject::new(meta.obj_type, schema, name, content.clone());

        // Set identity for overloaded objects
        if !meta.identity.is_empty() {
            obj = obj.with_identity(meta.identity);
        }

        // Set owner if present
        if let Some(owner) = meta.owner {
            obj = obj.with_owner(owner);
        }

        // Set target type for secondary objects
        if let Some(target_type) = meta.target_type {
            obj = obj.with_target_type(target_type);
        }

        // Store original pg_dump type string
        obj.set_metadata("pg_dump_type", meta.pg_type_str);

        // Extract dependencies based on object type
        self.populate_extracted_deps(&mut obj, &content);

        Some(obj)
    }

    /// Populate extracted_deps on an object based on its type and content.
    fn populate_extracted_deps(&self, obj: &mut RawObject, content: &str) {
        match obj.obj_type {
            // Trigger → Function (from EXECUTE FUNCTION clause)
            ObjectType::Trigger => {
                if let Some(caps) = TRIGGER_PATTERN.captures(content)
                    && let (Some(fn_schema), Some(fn_name)) =
                        (caps.name("fn_schema"), caps.name("fn_name"))
                {
                    obj.add_extracted_dep(
                        format!("{}.{}", fn_schema.as_str(), fn_name.as_str()),
                        ObjectType::Function,
                    );
                }
            }

            // ForeignTable → Server (from SERVER clause)
            ObjectType::ForeignTable => {
                if let Some(caps) = FOREIGN_TABLE_PATTERN.captures(content)
                    && let Some(server) = caps.name("server")
                {
                    obj.add_extracted_dep(server.as_str().to_string(), ObjectType::Server);
                }
            }

            // Subscription → Publication (from PUBLICATION clause)
            ObjectType::Subscription => {
                if let Some(caps) = SUBSCRIPTION_PATTERN.captures(content)
                    && let Some(pub_name) = caps.name("publication")
                {
                    obj.add_extracted_dep(pub_name.as_str().to_string(), ObjectType::Publication);
                }
            }

            // Server → ForeignDataWrapper (from FOREIGN DATA WRAPPER clause)
            ObjectType::Server => {
                if let Some(caps) = SERVER_PATTERN.captures(content)
                    && let Some(fdw) = caps.name("fdw")
                {
                    obj.add_extracted_dep(fdw.as_str().to_string(), ObjectType::ForeignDataWrapper);
                }
            }

            // UserMapping → Server (from SERVER clause)
            ObjectType::UserMapping => {
                if let Some(caps) = USER_MAPPING_PATTERN.captures(content)
                    && let Some(server) = caps.name("server")
                {
                    obj.add_extracted_dep(server.as_str().to_string(), ObjectType::Server);
                }
            }

            // EventTrigger → Function (from EXECUTE FUNCTION clause)
            ObjectType::EventTrigger => {
                if let Some(caps) = EVENT_TRIGGER_PATTERN.captures(content) {
                    let fn_name = caps.name("fn_name").map(|m| m.as_str());
                    let fn_schema = caps.name("fn_schema").map(|m| m.as_str());

                    if let Some(name) = fn_name {
                        let qualified = match fn_schema {
                            Some(schema) => format!("{schema}.{name}"),
                            None => name.to_string(),
                        };
                        obj.add_extracted_dep(qualified, ObjectType::Function);
                    }
                }
            }

            // Statistics → Table (from FROM table)
            ObjectType::Statistics => {
                if let Some(caps) = STATISTICS_PATTERN.captures(content) {
                    let tbl_name = caps.name("table_name").map(|m| m.as_str());
                    let tbl_schema = caps
                        .name("table_schema")
                        .map(|m| m.as_str())
                        .or(obj.schema.as_deref());

                    if let Some(name) = tbl_name {
                        let qualified = match tbl_schema {
                            Some(schema) => format!("{schema}.{name}"),
                            None => name.to_string(),
                        };
                        obj.add_extracted_dep(qualified, ObjectType::Table);
                    }
                }
            }

            // Rule → Table (from ON table)
            ObjectType::Rule => {
                if let Some(caps) = RULE_PATTERN.captures(content) {
                    let tbl_name = caps.name("table_name").map(|m| m.as_str());
                    let tbl_schema = caps
                        .name("schema")
                        .map(|m| m.as_str())
                        .or(obj.schema.as_deref());

                    if let Some(name) = tbl_name {
                        let qualified = match tbl_schema {
                            Some(schema) => format!("{schema}.{name}"),
                            None => name.to_string(),
                        };
                        obj.add_extracted_dep(qualified, ObjectType::Table);
                    }
                }
            }

            // Index → Table
            ObjectType::Index => {
                if let Some(caps) = INDEX_PATTERN.captures(content) {
                    let tbl_name = caps.name("table").map(|m| m.as_str());
                    let tbl_schema = caps
                        .name("schema")
                        .map(|m| m.as_str())
                        .or(obj.schema.as_deref());

                    if let Some(name) = tbl_name {
                        let qualified = match tbl_schema {
                            Some(schema) => format!("{schema}.{name}"),
                            None => name.to_string(),
                        };
                        obj.add_extracted_dep(qualified, ObjectType::Table);
                    }
                }
            }

            // Constraint/FK/Default/RowSecurity → Table
            ObjectType::Constraint
            | ObjectType::FkConstraint
            | ObjectType::Default
            | ObjectType::RowSecurity => {
                if let Some(caps) = ALTER_TABLE_PATTERN.captures(content)
                    && let (Some(tbl_schema), Some(tbl_name)) =
                        (caps.name("tbl_schema"), caps.name("tbl_name"))
                {
                    obj.add_extracted_dep(
                        format!("{}.{}", tbl_schema.as_str(), tbl_name.as_str()),
                        ObjectType::Table,
                    );
                }
            }

            // Policy → Table (from ON table)
            ObjectType::Policy => {
                if let Some(caps) = POLICY_PATTERN.captures(content) {
                    let tbl_name = caps.name("table").map(|m| m.as_str());
                    let tbl_schema = caps
                        .name("schema")
                        .map(|m| m.as_str())
                        .or(obj.schema.as_deref());

                    if let Some(name) = tbl_name {
                        let qualified = match tbl_schema {
                            Some(schema) => format!("{schema}.{name}"),
                            None => name.to_string(),
                        };
                        obj.add_extracted_dep(qualified, ObjectType::Table);
                    }
                }
            }

            // Cast → Type (try to attach to type)
            ObjectType::Cast => {
                if let Some(caps) = self.cast_pattern.captures(content) {
                    if let Some(from_name) = caps.name("from_name") {
                        let from_schema = caps.name("from_schema").map(|m| m.as_str());
                        let qualified = match from_schema {
                            Some(schema) => format!("{}.{}", schema, from_name.as_str()),
                            None => from_name.as_str().to_string(),
                        };
                        obj.add_extracted_dep(qualified, ObjectType::Type);
                    }

                    if let Some(to_name) = caps.name("to_name") {
                        let to_schema = caps.name("to_schema").map(|m| m.as_str());
                        let qualified = match to_schema {
                            Some(schema) => format!("{}.{}", schema, to_name.as_str()),
                            None => to_name.as_str().to_string(),
                        };
                        obj.add_extracted_dep(qualified, ObjectType::Type);
                    }
                }
            }

            // Operator → Function (from FUNCTION = clause)
            ObjectType::Operator => {
                if let Some(caps) = self.operator_pattern.captures(content)
                    && let Some(proc_name) = caps.name("procedure_name")
                {
                    obj.add_extracted_dep(proc_name.as_str().to_string(), ObjectType::Function);
                }
            }

            _ => {}
        }
    }

    /// Collect security statements from content.
    fn parse_security_statements(&self) -> Vec<SecurityStatement> {
        let mut statements = Vec::new();

        for line in self.content.lines() {
            // Check for GRANT statements
            if let Some(caps) = GRANT_PATTERN.captures(line) {
                let obj_type = caps.name("obj_type").and_then(|m| {
                    let parsed = ObjectType::from_pg_dump_type(&m.as_str().to_uppercase());
                    (parsed != ObjectType::Unknown).then_some(parsed)
                });

                statements.push(SecurityStatement::grant(
                    caps.name("obj_schema").map(|m| m.as_str().to_string()),
                    caps.name("obj_name")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    obj_type,
                    line.to_string(),
                ));
            }

            // Check for REVOKE statements
            if let Some(caps) = REVOKE_PATTERN.captures(line) {
                let obj_type = caps.name("obj_type").and_then(|m| {
                    let parsed = ObjectType::from_pg_dump_type(&m.as_str().to_uppercase());
                    (parsed != ObjectType::Unknown).then_some(parsed)
                });

                statements.push(SecurityStatement::revoke(
                    caps.name("obj_schema").map(|m| m.as_str().to_string()),
                    caps.name("obj_name")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    obj_type,
                    line.to_string(),
                ));
            }

            // Check for OWNER statements
            if let Some(caps) = OWNER_PATTERN.captures(line) {
                let obj_type = caps.name("obj_type").and_then(|m| {
                    let parsed = ObjectType::from_pg_dump_type(&m.as_str().to_uppercase());
                    (parsed != ObjectType::Unknown).then_some(parsed)
                });

                statements.push(SecurityStatement::owner(
                    caps.name("obj_schema").map(|m| m.as_str().to_string()),
                    caps.name("obj_name")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    obj_type,
                    line.to_string(),
                ));
            }
        }

        statements
    }

    /// Check if content contains DEFAULT ACL statements.
    pub fn has_default_acl(&self) -> bool {
        self.content
            .lines()
            .any(|line| DEFAULT_ACL_PATTERN.is_match(line))
    }

    /// Collect DEFAULT ACL statements.
    pub fn collect_default_acl(&self) -> Vec<String> {
        self.content
            .lines()
            .filter(|line| DEFAULT_ACL_PATTERN.is_match(line))
            .map(|s| s.to_string())
            .collect()
    }
}

impl ImportSource for PgDumpParser {
    fn extract_objects(&mut self) -> Result<Vec<RawObject>, TopCatError> {
        Ok(self.parse_objects())
    }

    fn collect_security(&mut self) -> Result<Vec<SecurityStatement>, TopCatError> {
        // Use cached version if available
        if self.cached_security.is_none() {
            self.cached_security = Some(self.parse_security_statements());
        }
        Ok(self.cached_security.clone().unwrap_or_default())
    }

    fn source_name(&self) -> &'static str {
        "pg_dump"
    }
}

/// Parsed metadata from a pg_dump metadata comment.
#[derive(Debug)]
struct ParsedMetadata {
    name: String,
    identity: String,
    schema: String,
    obj_type: ObjectType,
    pg_type_str: String,
    target_type: Option<String>,
    owner: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::import::sources::SecurityKind;

    #[test]
    fn test_parse_simple_table() {
        let content = r#"
-- Name: users; Type: TABLE; Schema: public; Owner: postgres

CREATE TABLE "public"."users" (
    id serial PRIMARY KEY,
    name text NOT NULL
);
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Table);
        assert_eq!(objects[0].schema, Some("public".to_string()));
        assert_eq!(objects[0].name, "users");
        assert_eq!(objects[0].owner, Some("postgres".to_string()));
    }

    #[test]
    fn test_parse_trigger_with_function_dep() {
        let content = r#"
-- Name: my_trigger; Type: TRIGGER; Schema: public;

CREATE TRIGGER my_trigger AFTER INSERT ON "public"."users" FOR EACH ROW EXECUTE FUNCTION "public"."notify_insert"()
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Trigger);

        // Check extracted dependency
        let deps = objects[0].extracted_deps();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "public.notify_insert");
        assert_eq!(deps[0].dep_type, ObjectType::Function);
    }

    #[test]
    fn test_parse_foreign_table_with_server_dep() {
        let content = r#"
-- Name: remote_data; Type: FOREIGN TABLE; Schema: public;

CREATE FOREIGN TABLE "public"."remote_data" (
    id integer
) SERVER "my_server"
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::ForeignTable);

        let deps = objects[0].extracted_deps();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "my_server");
        assert_eq!(deps[0].dep_type, ObjectType::Server);
    }

    #[test]
    fn test_parse_index_with_table_dep() {
        let content = r#"
-- Name: users_id_idx; Type: INDEX; Schema: public;

CREATE INDEX "users_id_idx" ON "public"."users" USING btree ("id");
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Index);

        let deps = objects[0].extracted_deps();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "public.users");
        assert_eq!(deps[0].dep_type, ObjectType::Table);
    }

    #[test]
    fn test_parse_policy_with_table_dep() {
        let content = r#"
-- Name: users_select_policy; Type: POLICY; Schema: public;

CREATE POLICY "users_select_policy" ON "public"."users"
    FOR SELECT USING (true);
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Policy);

        let deps = objects[0].extracted_deps();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "public.users");
        assert_eq!(deps[0].dep_type, ObjectType::Table);
    }

    #[test]
    fn test_parse_default_with_table_dep() {
        let content = r#"
-- Name: COLUMN id; Type: DEFAULT; Schema: public;

ALTER TABLE ONLY "public"."users" ALTER COLUMN "id" SET DEFAULT nextval('"users_id_seq"'::regclass);
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Default);

        let deps = objects[0].extracted_deps();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "public.users");
        assert_eq!(deps[0].dep_type, ObjectType::Table);
    }

    #[test]
    fn test_parse_cast_extracts_both_type_deps() {
        let content = r#"
-- Name: CAST (integer AS text); Type: CAST; Schema: -;

CREATE CAST (integer AS text) WITH FUNCTION int4_to_text(integer);
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Cast);

        let dep_names: Vec<_> = objects[0]
            .extracted_deps()
            .iter()
            .map(|dep| dep.name.as_str())
            .collect();
        assert!(dep_names.contains(&"integer"));
        assert!(dep_names.contains(&"text"));
    }

    #[test]
    fn test_parse_subscription_with_publication_dep() {
        let content = r#"
-- Name: my_sub; Type: SUBSCRIPTION; Schema: -;

CREATE SUBSCRIPTION "my_sub" CONNECTION 'host=primary' PUBLICATION "my_pub"
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Subscription);

        let deps = objects[0].extracted_deps();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "my_pub");
        assert_eq!(deps[0].dep_type, ObjectType::Publication);
    }

    #[test]
    fn test_parse_server_with_fdw_dep() {
        let content = r#"
-- Name: remote_server; Type: SERVER; Schema: -;

CREATE SERVER "remote_server" FOREIGN DATA WRAPPER "postgres_fdw"
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Server);
        assert!(objects[0].is_global());

        let deps = objects[0].extracted_deps();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "postgres_fdw");
        assert_eq!(deps[0].dep_type, ObjectType::ForeignDataWrapper);
    }

    #[test]
    fn test_collect_security_statements() {
        let content = r#"
GRANT SELECT ON TABLE "public"."users" TO "reader";
REVOKE ALL ON TABLE "public"."users" FROM PUBLIC;
ALTER TABLE "public"."users" OWNER TO "admin";
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let security = parser.collect_security().unwrap();

        assert_eq!(security.len(), 3);

        assert_eq!(security[0].kind, SecurityKind::Grant);
        assert_eq!(security[0].target_name, "users");

        assert_eq!(security[1].kind, SecurityKind::Revoke);
        assert_eq!(security[1].target_name, "users");

        assert_eq!(security[2].kind, SecurityKind::Owner);
        assert_eq!(security[2].target_name, "users");
    }

    #[test]
    fn test_global_objects_have_no_schema() {
        let content = r#"
-- Name: my_fdw; Type: FOREIGN DATA WRAPPER; Schema: -;

CREATE FOREIGN DATA WRAPPER "my_fdw" HANDLER pg_catalog.my_handler
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert!(objects[0].is_global());
        assert_eq!(objects[0].schema, None);
    }

    #[test]
    fn test_extension_schema_extraction() {
        let content = r#"
-- Name: pgcrypto; Type: EXTENSION; Schema: -;

CREATE EXTENSION IF NOT EXISTS "pgcrypto" WITH SCHEMA "crypto"
"#;

        let mut parser = PgDumpParser::from_content(content.to_string(), None);
        let objects = parser.extract_objects().unwrap();

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].obj_type, ObjectType::Extension);
        assert_eq!(objects[0].schema, Some("crypto".to_string()));
        assert_eq!(objects[0].name, "pgcrypto");
    }
}
