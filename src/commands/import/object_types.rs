//! PostgreSQL object type definitions and categorization.
//!
//! This module defines all PostgreSQL object types that can appear in pg_dump output
//! and provides categorization for output organization, layer assignment, and
//! dependency handling.

use std::fmt;

/// All PostgreSQL object types that can appear in pg_dump output.
///
/// This enum covers the complete set of database objects that pg_dump can export,
/// organized roughly by category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    // Schema-level structural objects
    Schema,
    Extension,
    Table,
    View,
    MaterializedView,
    ForeignTable,
    Sequence,

    // Types
    Type,
    Domain,
    Collation,

    // Routines
    Function,
    Procedure,
    Aggregate,

    // Table attachments
    Index,
    Constraint,
    FkConstraint,
    Trigger,
    Policy,
    RowSecurity,
    Default,
    Statistics,
    Rule,

    // Full Text Search
    TextSearchConfiguration,
    TextSearchDictionary,
    TextSearchParser,
    TextSearchTemplate,

    // Foreign Data Wrapper
    ForeignDataWrapper,
    Server,
    UserMapping,

    // Operators and Access Methods
    Operator,
    OperatorClass,
    OperatorFamily,
    AccessMethod,
    Cast,

    // Replication
    Publication,
    Subscription,

    // Database-level
    EventTrigger,
    Language,
    Transform,
    Conversion,

    // Security
    Acl,
    DefaultAcl,
    SecurityLabel,

    // Comments and other
    Comment,

    // Unknown or unsupported
    Unknown,
}

struct ObjectTypeMetadata {
    obj_type: ObjectType,
    canonical: &'static str,
    aliases: &'static [&'static str],
}

const fn object_type_metadata(
    obj_type: ObjectType,
    canonical: &'static str,
    aliases: &'static [&'static str],
) -> ObjectTypeMetadata {
    ObjectTypeMetadata {
        obj_type,
        canonical,
        aliases,
    }
}

const OBJECT_TYPE_METADATA: [ObjectTypeMetadata; 45] = [
    object_type_metadata(ObjectType::Schema, "SCHEMA", &["SCHEMA"]),
    object_type_metadata(ObjectType::Extension, "EXTENSION", &["EXTENSION"]),
    object_type_metadata(ObjectType::Table, "TABLE", &["TABLE"]),
    object_type_metadata(ObjectType::View, "VIEW", &["VIEW"]),
    object_type_metadata(
        ObjectType::MaterializedView,
        "MATERIALIZED VIEW",
        &["MATERIALIZED VIEW"],
    ),
    object_type_metadata(
        ObjectType::ForeignTable,
        "FOREIGN TABLE",
        &["FOREIGN TABLE"],
    ),
    object_type_metadata(
        ObjectType::Sequence,
        "SEQUENCE",
        &["SEQUENCE", "SEQUENCE OWNED BY"],
    ),
    object_type_metadata(ObjectType::Type, "TYPE", &["TYPE"]),
    object_type_metadata(ObjectType::Domain, "DOMAIN", &["DOMAIN"]),
    object_type_metadata(ObjectType::Collation, "COLLATION", &["COLLATION"]),
    object_type_metadata(ObjectType::Function, "FUNCTION", &["FUNCTION"]),
    object_type_metadata(ObjectType::Procedure, "PROCEDURE", &["PROCEDURE"]),
    object_type_metadata(ObjectType::Aggregate, "AGGREGATE", &["AGGREGATE"]),
    object_type_metadata(ObjectType::Index, "INDEX", &["INDEX"]),
    object_type_metadata(ObjectType::Constraint, "CONSTRAINT", &["CONSTRAINT"]),
    object_type_metadata(
        ObjectType::FkConstraint,
        "FK CONSTRAINT",
        &["FK CONSTRAINT"],
    ),
    object_type_metadata(ObjectType::Trigger, "TRIGGER", &["TRIGGER"]),
    object_type_metadata(ObjectType::Policy, "POLICY", &["POLICY"]),
    object_type_metadata(ObjectType::RowSecurity, "ROW SECURITY", &["ROW SECURITY"]),
    object_type_metadata(ObjectType::Default, "DEFAULT", &["DEFAULT"]),
    object_type_metadata(ObjectType::Statistics, "STATISTICS", &["STATISTICS"]),
    object_type_metadata(ObjectType::Rule, "RULE", &["RULE"]),
    object_type_metadata(
        ObjectType::TextSearchConfiguration,
        "TEXT SEARCH CONFIGURATION",
        &["TEXT SEARCH CONFIGURATION"],
    ),
    object_type_metadata(
        ObjectType::TextSearchDictionary,
        "TEXT SEARCH DICTIONARY",
        &["TEXT SEARCH DICTIONARY"],
    ),
    object_type_metadata(
        ObjectType::TextSearchParser,
        "TEXT SEARCH PARSER",
        &["TEXT SEARCH PARSER"],
    ),
    object_type_metadata(
        ObjectType::TextSearchTemplate,
        "TEXT SEARCH TEMPLATE",
        &["TEXT SEARCH TEMPLATE"],
    ),
    object_type_metadata(
        ObjectType::ForeignDataWrapper,
        "FOREIGN DATA WRAPPER",
        &["FOREIGN DATA WRAPPER"],
    ),
    object_type_metadata(ObjectType::Server, "SERVER", &["SERVER", "FOREIGN SERVER"]),
    object_type_metadata(ObjectType::UserMapping, "USER MAPPING", &["USER MAPPING"]),
    object_type_metadata(ObjectType::Operator, "OPERATOR", &["OPERATOR"]),
    object_type_metadata(
        ObjectType::OperatorClass,
        "OPERATOR CLASS",
        &["OPERATOR CLASS"],
    ),
    object_type_metadata(
        ObjectType::OperatorFamily,
        "OPERATOR FAMILY",
        &["OPERATOR FAMILY"],
    ),
    object_type_metadata(
        ObjectType::AccessMethod,
        "ACCESS METHOD",
        &["ACCESS METHOD"],
    ),
    object_type_metadata(ObjectType::Cast, "CAST", &["CAST"]),
    object_type_metadata(ObjectType::Publication, "PUBLICATION", &["PUBLICATION"]),
    object_type_metadata(ObjectType::Subscription, "SUBSCRIPTION", &["SUBSCRIPTION"]),
    object_type_metadata(
        ObjectType::EventTrigger,
        "EVENT TRIGGER",
        &["EVENT TRIGGER"],
    ),
    object_type_metadata(
        ObjectType::Language,
        "PROCEDURAL LANGUAGE",
        &["PROCEDURAL LANGUAGE", "LANGUAGE"],
    ),
    object_type_metadata(ObjectType::Transform, "TRANSFORM", &["TRANSFORM"]),
    object_type_metadata(ObjectType::Conversion, "CONVERSION", &["CONVERSION"]),
    object_type_metadata(ObjectType::Acl, "ACL", &["ACL"]),
    object_type_metadata(ObjectType::DefaultAcl, "DEFAULT ACL", &["DEFAULT ACL"]),
    object_type_metadata(
        ObjectType::SecurityLabel,
        "SECURITY LABEL",
        &["SECURITY LABEL"],
    ),
    object_type_metadata(ObjectType::Comment, "COMMENT", &["COMMENT"]),
    object_type_metadata(ObjectType::Unknown, "UNKNOWN", &["UNKNOWN"]),
];

impl ObjectType {
    /// Return all object types known to the import system.
    pub fn all() -> Vec<Self> {
        OBJECT_TYPE_METADATA
            .iter()
            .map(|metadata| metadata.obj_type)
            .collect()
    }

    /// Parse an object type from pg_dump metadata comment.
    ///
    /// The type string comes from the `Type:` field in pg_dump comments like:
    /// `-- Name: users; Type: TABLE; Schema: public;`
    pub fn from_pg_dump_type(type_str: &str) -> Self {
        let normalized = type_str.trim().to_uppercase();
        OBJECT_TYPE_METADATA
            .iter()
            .find(|metadata| metadata.aliases.contains(&normalized.as_str()))
            .map(|metadata| metadata.obj_type)
            .unwrap_or(Self::Unknown)
    }

    /// Get the pg_dump type string representation.
    pub fn as_pg_dump_type(&self) -> &'static str {
        OBJECT_TYPE_METADATA
            .iter()
            .find(|metadata| metadata.obj_type == *self)
            .map(|metadata| metadata.canonical)
            .unwrap_or("UNKNOWN")
    }

    /// Check if this is a primary object type (not an attachment).
    ///
    /// Primary objects are standalone and can have their own files.
    /// Attachment objects belong to a parent object.
    pub fn is_primary(&self) -> bool {
        matches!(
            self,
            Self::Schema
                | Self::Extension
                | Self::Table
                | Self::View
                | Self::MaterializedView
                | Self::ForeignTable
                | Self::Sequence
                | Self::Type
                | Self::Domain
                | Self::Collation
                | Self::Function
                | Self::Procedure
                | Self::Aggregate
                | Self::TextSearchConfiguration
                | Self::TextSearchDictionary
                | Self::TextSearchParser
                | Self::TextSearchTemplate
                | Self::ForeignDataWrapper
                | Self::Server
                | Self::UserMapping
                | Self::Operator
                | Self::OperatorClass
                | Self::OperatorFamily
                | Self::AccessMethod
                | Self::Cast
                | Self::Publication
                | Self::Subscription
                | Self::EventTrigger
                | Self::Language
                | Self::Transform
                | Self::Conversion
        )
    }

    /// Check if this is a table attachment (belongs to a table).
    pub fn is_table_attachment(&self) -> bool {
        matches!(
            self,
            Self::Index
                | Self::Constraint
                | Self::FkConstraint
                | Self::Trigger
                | Self::Policy
                | Self::RowSecurity
                | Self::Default
                | Self::Statistics
                | Self::Rule
        )
    }

    /// Check if this is a global object (not schema-qualified).
    pub fn is_global(&self) -> bool {
        matches!(
            self,
            Self::ForeignDataWrapper
                | Self::Server
                | Self::UserMapping
                | Self::AccessMethod
                | Self::Language
                | Self::EventTrigger
                | Self::Publication
                | Self::Subscription
                | Self::Transform
        )
    }

    /// Check if this is a security-related object.
    pub fn is_security(&self) -> bool {
        matches!(
            self,
            Self::Acl | Self::DefaultAcl | Self::SecurityLabel | Self::Policy | Self::RowSecurity
        )
    }

    /// Get the category for output directory organization.
    pub fn category(&self) -> ObjectCategory {
        match self {
            Self::Schema => ObjectCategory::Schema,
            Self::Extension => ObjectCategory::Extension,
            Self::Table => ObjectCategory::Table,
            Self::View => ObjectCategory::View,
            Self::MaterializedView => ObjectCategory::MaterializedView,
            Self::ForeignTable => ObjectCategory::ForeignTable,
            Self::Sequence => ObjectCategory::Sequence,
            Self::Type | Self::Domain => ObjectCategory::Type,
            Self::Collation => ObjectCategory::Collation,
            Self::Function | Self::Procedure | Self::Aggregate => ObjectCategory::Function,
            Self::Index
            | Self::Constraint
            | Self::FkConstraint
            | Self::Trigger
            | Self::Default
            | Self::Statistics
            | Self::Rule => ObjectCategory::TableAttachment,
            Self::Policy | Self::RowSecurity => ObjectCategory::Security,
            Self::TextSearchConfiguration
            | Self::TextSearchDictionary
            | Self::TextSearchParser
            | Self::TextSearchTemplate => ObjectCategory::TextSearch,
            Self::ForeignDataWrapper | Self::Server | Self::UserMapping => {
                ObjectCategory::ForeignDataWrapper
            }
            Self::Operator | Self::OperatorClass | Self::OperatorFamily => ObjectCategory::Operator,
            Self::AccessMethod => ObjectCategory::AccessMethod,
            Self::Cast => ObjectCategory::Cast,
            Self::Publication | Self::Subscription => ObjectCategory::Replication,
            Self::EventTrigger => ObjectCategory::EventTrigger,
            Self::Language => ObjectCategory::Language,
            Self::Transform => ObjectCategory::Transform,
            Self::Conversion => ObjectCategory::Conversion,
            Self::Acl | Self::DefaultAcl | Self::SecurityLabel => ObjectCategory::Security,
            Self::Comment => ObjectCategory::Comment,
            Self::Unknown => ObjectCategory::Unknown,
        }
    }

    /// Get the layer for dependency ordering.
    ///
    /// Objects are assigned to layers to ensure proper creation order:
    /// - `prepend`: Foundation objects (schemas, extensions, languages, types)
    /// - `normal`: Main objects (tables, views, functions)
    /// - `append`: Dependent objects (constraints, triggers, grants)
    pub fn layer(&self) -> Layer {
        match self {
            // Foundation layer - must exist before anything else
            Self::Schema
            | Self::Extension
            | Self::Language
            | Self::Type
            | Self::Domain
            | Self::Collation
            | Self::TextSearchParser
            | Self::TextSearchTemplate
            | Self::TextSearchDictionary
            | Self::TextSearchConfiguration
            | Self::ForeignDataWrapper
            | Self::Server
            | Self::AccessMethod
            | Self::OperatorFamily
            | Self::OperatorClass
            | Self::Operator
            | Self::Cast
            | Self::Transform
            | Self::Conversion => Layer::Prepend,

            // Normal layer - main structural objects
            Self::Sequence
            | Self::Table
            | Self::ForeignTable
            | Self::View
            | Self::MaterializedView
            | Self::Function
            | Self::Procedure
            | Self::Aggregate
            | Self::UserMapping
            | Self::Publication
            | Self::Subscription
            | Self::EventTrigger => Layer::Normal,

            // Append layer - objects that depend on tables/functions
            Self::Index
            | Self::Constraint
            | Self::FkConstraint
            | Self::Trigger
            | Self::Policy
            | Self::RowSecurity
            | Self::Default
            | Self::Statistics
            | Self::Rule
            | Self::Acl
            | Self::DefaultAcl
            | Self::SecurityLabel
            | Self::Comment
            | Self::Unknown => Layer::Append,
        }
    }

    /// Get the category directory name for file organization.
    ///
    /// Returns the subdirectory name within a schema directory where
    /// objects of this type should be placed.
    pub fn category_dir(&self) -> String {
        match self {
            Self::Table => "table",
            Self::View => "view",
            Self::MaterializedView => "materialized_view",
            Self::ForeignTable => "foreign_table",
            Self::Sequence => "sequence",
            Self::Schema => "schema",
            Self::Extension => "extension",
            Self::Domain => "type/domain",
            Self::Collation => "collation",
            Self::Conversion => "conversion",
            Self::Function | Self::Procedure => "functions",
            Self::Aggregate => "functions/aggregate",
            Self::TextSearchConfiguration => "fts/configuration",
            Self::TextSearchDictionary => "fts/dictionary",
            Self::TextSearchParser => "fts/parser",
            Self::TextSearchTemplate => "fts/template",
            _ => "unknown",
        }
        .to_string()
    }

    /// Get category and subcategory for global objects (no schema).
    ///
    /// Returns a tuple of (category, optional subcategory) used for
    /// organizing global objects in the `_global/` directory.
    pub fn global_category(&self) -> (String, Option<String>) {
        match self {
            Self::ForeignDataWrapper => ("fdw".into(), Some("wrapper".into())),
            Self::Server => ("fdw".into(), Some("server".into())),
            Self::UserMapping => ("fdw".into(), Some("user_mapping".into())),
            Self::Language => ("language".into(), None),
            Self::EventTrigger => ("event_trigger".into(), None),
            Self::Publication => ("replication".into(), Some("publication".into())),
            Self::Subscription => ("replication".into(), Some("subscription".into())),
            Self::AccessMethod => ("access_method".into(), None),
            Self::OperatorClass => ("operator".into(), Some("class".into())),
            Self::OperatorFamily => ("operator".into(), Some("family".into())),
            Self::Transform => ("transform".into(), None),
            Self::Cast => ("cast".into(), None),
            Self::Operator => ("operator".into(), None),
            _ => ("other".into(), None),
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_pg_dump_type())
    }
}

/// Categories for organizing output directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectCategory {
    Schema,
    Extension,
    Table,
    View,
    MaterializedView,
    ForeignTable,
    Sequence,
    Type,
    Collation,
    Function,
    TableAttachment,
    TextSearch,
    ForeignDataWrapper,
    Operator,
    AccessMethod,
    Cast,
    Replication,
    EventTrigger,
    Language,
    Transform,
    Conversion,
    Security,
    Comment,
    Unknown,
}

impl ObjectCategory {
    /// Get the directory path for this category.
    ///
    /// Returns the relative path within a schema directory, or a global path prefix.
    pub fn directory_path(&self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Extension => "extension",
            Self::Table => "table",
            Self::View => "view",
            Self::MaterializedView => "materialized_view",
            Self::ForeignTable => "foreign_table",
            Self::Sequence => "sequence",
            Self::Type => "type",
            Self::Collation => "collation",
            Self::Function => "functions",
            Self::TableAttachment => "", // Attached to parent
            Self::TextSearch => "fts",
            Self::ForeignDataWrapper => "_global/fdw",
            Self::Operator => "_global/operator",
            Self::AccessMethod => "_global/access_method",
            Self::Cast => "_global/cast",
            Self::Replication => "_global/replication",
            Self::EventTrigger => "_global/event_trigger",
            Self::Language => "_global/language",
            Self::Transform => "_global/transform",
            Self::Conversion => "conversion",
            Self::Security => "_acl",
            Self::Comment => "", // Attached to parent
            Self::Unknown => "_unknown",
        }
    }

    /// Check if this category produces global (non-schema-qualified) files.
    pub fn is_global(&self) -> bool {
        matches!(
            self,
            Self::ForeignDataWrapper
                | Self::Operator
                | Self::AccessMethod
                | Self::Cast
                | Self::Replication
                | Self::EventTrigger
                | Self::Language
                | Self::Transform
        )
    }
}

impl fmt::Display for ObjectCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.directory_path())
    }
}

/// Layers for dependency ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Layer {
    /// Foundation objects that must be created first.
    Prepend,
    /// Main structural objects.
    Normal,
    /// Dependent objects that must be created after their parents.
    Append,
}

impl Layer {
    /// Get the layer name for topcat headers.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Prepend => "prepend",
            Self::Normal => "normal",
            Self::Append => "append",
        }
    }
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration for how to handle a specific object type during import.
#[derive(Debug, Clone)]
pub struct ObjectTypeConfig {
    /// Whether this type should be skipped during import.
    pub skip: bool,

    /// Whether to attach to parent object instead of creating separate file.
    pub attach_to_parent: bool,

    /// The parent object types to try attaching to.
    pub parent_types: Vec<ObjectType>,
}

impl ObjectTypeConfig {
    /// Create default configuration for an object type.
    pub fn default_for(object_type: ObjectType) -> Self {
        let (attach_to_parent, parent_types) = match object_type {
            // Table attachments
            ObjectType::Index
            | ObjectType::Constraint
            | ObjectType::FkConstraint
            | ObjectType::Default
            | ObjectType::Statistics
            | ObjectType::Rule => (true, vec![ObjectType::Table]),

            // Trigger attaches to function
            ObjectType::Trigger => (true, vec![ObjectType::Function]),

            // Policies attach to table
            ObjectType::Policy | ObjectType::RowSecurity => (true, vec![ObjectType::Table]),

            // Cast can attach to type or table
            ObjectType::Cast => (false, vec![ObjectType::Type, ObjectType::Table]),

            // Operator can attach to type, function, or procedure
            ObjectType::Operator => (
                false,
                vec![
                    ObjectType::Type,
                    ObjectType::Function,
                    ObjectType::Procedure,
                    ObjectType::Table,
                ],
            ),

            // Comments attach to their target
            ObjectType::Comment => (true, vec![]),

            // Security objects attach to their target
            ObjectType::Acl | ObjectType::SecurityLabel => (true, vec![]),

            // Everything else is standalone
            _ => (false, vec![]),
        };

        Self {
            skip: object_type == ObjectType::Unknown,
            attach_to_parent,
            parent_types,
        }
    }
}

/// Type subcategorization for types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSubcategory {
    Enum,
    Composite,
    Domain,
    Range,
    Base,
}

impl TypeSubcategory {
    /// Detect type subcategory from SQL content.
    pub fn from_content(content: &str) -> Self {
        if content.contains("AS ENUM") {
            Self::Enum
        } else if content.contains("CREATE DOMAIN") {
            Self::Domain
        } else if content.contains("AS RANGE") {
            Self::Range
        } else if content.contains("AS (") || content.contains("AS\n(") {
            Self::Composite
        } else {
            Self::Base
        }
    }

    /// Get subdirectory for this type subcategory.
    pub fn subdirectory(&self) -> &'static str {
        match self {
            Self::Enum => "enum",
            Self::Composite => "composite",
            Self::Domain => "domain",
            Self::Range => "range",
            Self::Base => "base",
        }
    }
}

/// FTS object subcategorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtsSubcategory {
    Configuration,
    Dictionary,
    Parser,
    Template,
}

impl FtsSubcategory {
    /// Get subdirectory for this FTS subcategory.
    pub fn subdirectory(&self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Dictionary => "dictionary",
            Self::Parser => "parser",
            Self::Template => "template",
        }
    }
}

impl From<ObjectType> for Option<FtsSubcategory> {
    fn from(obj_type: ObjectType) -> Self {
        match obj_type {
            ObjectType::TextSearchConfiguration => Some(FtsSubcategory::Configuration),
            ObjectType::TextSearchDictionary => Some(FtsSubcategory::Dictionary),
            ObjectType::TextSearchParser => Some(FtsSubcategory::Parser),
            ObjectType::TextSearchTemplate => Some(FtsSubcategory::Template),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_type_all_round_trip() {
        let all_types = ObjectType::all();
        assert!(all_types.contains(&ObjectType::Unknown));

        for object_type in all_types {
            let as_text = object_type.as_pg_dump_type();
            let parsed = ObjectType::from_pg_dump_type(as_text);
            assert_eq!(
                parsed, object_type,
                "round-trip failed for canonical type string: {as_text}"
            );
        }
    }

    #[test]
    fn test_object_type_parsing() {
        assert_eq!(ObjectType::from_pg_dump_type("TABLE"), ObjectType::Table);
        assert_eq!(ObjectType::from_pg_dump_type("table"), ObjectType::Table);
        assert_eq!(
            ObjectType::from_pg_dump_type("MATERIALIZED VIEW"),
            ObjectType::MaterializedView
        );
        assert_eq!(
            ObjectType::from_pg_dump_type("FK CONSTRAINT"),
            ObjectType::FkConstraint
        );
        assert_eq!(
            ObjectType::from_pg_dump_type("TEXT SEARCH CONFIGURATION"),
            ObjectType::TextSearchConfiguration
        );
        assert_eq!(
            ObjectType::from_pg_dump_type("FOREIGN DATA WRAPPER"),
            ObjectType::ForeignDataWrapper
        );
        assert_eq!(
            ObjectType::from_pg_dump_type("PROCEDURAL LANGUAGE"),
            ObjectType::Language
        );
        assert_eq!(
            ObjectType::from_pg_dump_type("UNKNOWN_TYPE"),
            ObjectType::Unknown
        );
    }

    #[test]
    fn test_object_type_is_primary() {
        assert!(ObjectType::Table.is_primary());
        assert!(ObjectType::Function.is_primary());
        assert!(ObjectType::MaterializedView.is_primary());
        assert!(!ObjectType::Index.is_primary());
        assert!(!ObjectType::Constraint.is_primary());
        assert!(!ObjectType::Comment.is_primary());
    }

    #[test]
    fn test_object_type_is_table_attachment() {
        assert!(ObjectType::Index.is_table_attachment());
        assert!(ObjectType::Constraint.is_table_attachment());
        assert!(ObjectType::FkConstraint.is_table_attachment());
        assert!(ObjectType::Trigger.is_table_attachment());
        assert!(!ObjectType::Table.is_table_attachment());
        assert!(!ObjectType::Function.is_table_attachment());
    }

    #[test]
    fn test_object_type_is_global() {
        assert!(ObjectType::ForeignDataWrapper.is_global());
        assert!(ObjectType::Server.is_global());
        assert!(ObjectType::Language.is_global());
        assert!(!ObjectType::Table.is_global());
        assert!(!ObjectType::Function.is_global());
    }

    #[test]
    fn test_layer_assignment() {
        // Foundation layer
        assert_eq!(ObjectType::Schema.layer(), Layer::Prepend);
        assert_eq!(ObjectType::Extension.layer(), Layer::Prepend);
        assert_eq!(ObjectType::Type.layer(), Layer::Prepend);
        assert_eq!(ObjectType::Language.layer(), Layer::Prepend);

        // Normal layer
        assert_eq!(ObjectType::Table.layer(), Layer::Normal);
        assert_eq!(ObjectType::View.layer(), Layer::Normal);
        assert_eq!(ObjectType::Function.layer(), Layer::Normal);

        // Append layer
        assert_eq!(ObjectType::Index.layer(), Layer::Append);
        assert_eq!(ObjectType::Constraint.layer(), Layer::Append);
        assert_eq!(ObjectType::Acl.layer(), Layer::Append);
    }

    #[test]
    fn test_type_subcategory_detection() {
        assert_eq!(
            TypeSubcategory::from_content("CREATE TYPE foo AS ENUM ('a', 'b')"),
            TypeSubcategory::Enum
        );
        assert_eq!(
            TypeSubcategory::from_content("CREATE DOMAIN foo AS text"),
            TypeSubcategory::Domain
        );
        assert_eq!(
            TypeSubcategory::from_content("CREATE TYPE foo AS (x int, y int)"),
            TypeSubcategory::Composite
        );
        assert_eq!(
            TypeSubcategory::from_content("CREATE TYPE foo AS RANGE (subtype = int)"),
            TypeSubcategory::Range
        );
    }

    #[test]
    fn test_category_directory_paths() {
        assert_eq!(ObjectCategory::Table.directory_path(), "table");
        assert_eq!(ObjectCategory::Function.directory_path(), "functions");
        assert_eq!(ObjectCategory::TextSearch.directory_path(), "fts");
        assert_eq!(
            ObjectCategory::ForeignDataWrapper.directory_path(),
            "_global/fdw"
        );
        assert_eq!(
            ObjectCategory::Replication.directory_path(),
            "_global/replication"
        );
    }
}
