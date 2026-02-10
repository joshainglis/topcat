//! CLI integration tests for the `import pg-dump` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Get a Command instance for the topcat binary
fn topcat_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("topcat"))
}

/// Create a sample pg_dump file with various object types
fn create_sample_pg_dump(dir: &TempDir) -> PathBuf {
    let dump_file = dir.path().join("database.sql");

    let content = r#"--
-- PostgreSQL database dump
--

-- Name: public; Type: SCHEMA; Schema: -; Owner: postgres

CREATE SCHEMA public;

-- Name: auth; Type: SCHEMA; Schema: -; Owner: postgres

CREATE SCHEMA auth;

-- Name: users; Type: TABLE; Schema: public; Owner: postgres

CREATE TABLE "public"."users" (
    id serial PRIMARY KEY,
    email text NOT NULL,
    created_at timestamp DEFAULT NOW()
);

-- Name: roles; Type: TABLE; Schema: auth; Owner: postgres

CREATE TABLE "auth"."roles" (
    id serial PRIMARY KEY,
    name text NOT NULL
);

-- Name: user_roles; Type: TABLE; Schema: auth; Owner: postgres

CREATE TABLE "auth"."user_roles" (
    user_id integer REFERENCES "public"."users"(id),
    role_id integer REFERENCES "auth"."roles"(id),
    PRIMARY KEY (user_id, role_id)
);

-- Name: status; Type: TYPE; Schema: public; Owner: postgres

CREATE TYPE "public"."status" AS ENUM ('active', 'inactive', 'pending');

-- Name: get_user; Type: FUNCTION; Schema: public; Owner: postgres

CREATE OR REPLACE FUNCTION "public"."get_user"(p_id integer)
RETURNS TABLE(id integer, email text)
LANGUAGE sql
AS $$
    SELECT id, email FROM "public"."users" WHERE id = p_id;
$$;

-- Name: users_email_idx; Type: INDEX; Schema: public; Owner: postgres

CREATE INDEX users_email_idx ON "public"."users" (email);

-- Name: users_fk_constraint; Type: FK CONSTRAINT; Schema: auth; Owner: postgres

ALTER TABLE ONLY "auth"."user_roles"
    ADD CONSTRAINT users_fk_constraint FOREIGN KEY (user_id)
    REFERENCES "public"."users"(id);

-- Name: user_view; Type: VIEW; Schema: public; Owner: postgres

CREATE VIEW "public"."user_view" AS
    SELECT id, email FROM "public"."users";

-- Name: MATERIALIZED VIEW report_summary; Type: MATERIALIZED VIEW; Schema: public; Owner: postgres

CREATE MATERIALIZED VIEW "public"."report_summary" AS
    SELECT COUNT(*) as total FROM "public"."users";

-- Name: user_seq; Type: SEQUENCE; Schema: public; Owner: postgres

CREATE SEQUENCE "public"."user_seq" START 1 INCREMENT 1;

"#;

    fs::write(&dump_file, content).unwrap();
    dump_file
}

#[test]
fn test_import_pg_dump_basic() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    // Verify output directory was created
    assert!(output_dir.exists());

    // Verify schema directories were created
    assert!(output_dir.join("public").exists());
    assert!(output_dir.join("auth").exists());

    // Verify table files were created
    assert!(output_dir.join("public/table/users.sql").exists());
    assert!(output_dir.join("auth/table/roles.sql").exists());

    // Verify type file was created
    assert!(output_dir.join("public/type/enum/status.sql").exists());

    // Verify function file was created
    assert!(output_dir.join("public/functions/get_user.sql").exists());
}

#[test]
fn test_import_pg_dump_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would create"));

    // Verify output directory was NOT created
    assert!(!output_dir.exists());
}

#[test]
fn test_import_pg_dump_with_layers() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    // Layers are enabled by default, so just run without the flag
    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Check that a file contains layer header
    let schema_file = output_dir.join("public/public.sql");
    assert!(
        schema_file.exists(),
        "expected schema file at {schema_file:?}"
    );
    let content = fs::read_to_string(&schema_file).unwrap();
    assert!(content.contains("-- layer: prepend"));

    // Check a table file for normal layer
    let table_file = output_dir.join("public/table/users.sql");
    assert!(table_file.exists(), "expected table file at {table_file:?}");
    let content = fs::read_to_string(&table_file).unwrap();
    assert!(content.contains("-- layer: normal"));
}

#[test]
fn test_import_pg_dump_no_layers() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    // Note: To disable layers, we pass --generate-layers false (two separate args)
    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
            "--generate-layers",
            "false",
        ])
        .assert()
        .success();

    // Check that a file does NOT contain layer header
    let table_file = output_dir.join("public/table/users.sql");
    assert!(table_file.exists(), "expected table file at {table_file:?}");
    let content = fs::read_to_string(&table_file).unwrap();
    assert!(!content.contains("-- layer:"));
}

#[test]
fn test_import_pg_dump_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.sql");
    let output_dir = temp_dir.path().join("output");

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            nonexistent.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_import_pg_dump_file_headers() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Check that files have proper name headers
    let table_file = output_dir.join("public/table/users.sql");
    assert!(table_file.exists(), "expected table file at {table_file:?}");
    let content = fs::read_to_string(&table_file).unwrap();
    assert!(content.starts_with("-- name: public.users"));

    let func_file = output_dir.join("public/functions/get_user.sql");
    assert!(
        func_file.exists(),
        "expected function file at {func_file:?}"
    );
    let content = fs::read_to_string(&func_file).unwrap();
    assert!(content.starts_with("-- name: public.get_user"));
}

#[test]
fn test_import_pg_dump_materialized_view() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Check that materialized view was created in the right location
    let mv_file = output_dir.join("public/materialized_view/report_summary.sql");
    assert!(
        mv_file.exists(),
        "expected materialized view file at {mv_file:?}"
    );
    let content = fs::read_to_string(&mv_file).unwrap();
    assert!(content.contains("CREATE MATERIALIZED VIEW"));
}

#[test]
fn test_import_pg_dump_view() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Check that view was created
    let view_file = output_dir.join("public/view/user_view.sql");
    assert!(view_file.exists(), "expected view file at {view_file:?}");
    let content = fs::read_to_string(&view_file).unwrap();
    assert!(content.contains("CREATE VIEW"));
}

#[test]
fn test_import_pg_dump_sequence() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = create_sample_pg_dump(&temp_dir);
    let output_dir = temp_dir.path().join("output");

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Check that sequence was created in the right location
    let seq_file = output_dir.join("public/sequence/user_seq.sql");
    assert!(seq_file.exists(), "expected sequence file at {seq_file:?}");
    let content = fs::read_to_string(&seq_file).unwrap();
    assert!(content.contains("CREATE SEQUENCE"));
}

#[test]
fn test_import_pg_dump_schema_less_cast_and_operator_are_global() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = temp_dir.path().join("database.sql");
    let output_dir = temp_dir.path().join("output");

    let content = r#"
-- Name: my_cast; Type: CAST; Schema: -; Owner: postgres

CREATE CAST ("public"."src_type" AS "public"."dst_type")
    WITH FUNCTION "public"."cast_src_to_dst"("public"."src_type");

-- Name: my_op; Type: OPERATOR; Schema: -; Owner: postgres

CREATE OPERATOR "my_op" (
    LEFTARG = integer,
    RIGHTARG = integer,
    PROCEDURE = "public"."my_proc"
);
"#;
    fs::write(&dump_file, content).unwrap();

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_dir.join("_global/cast/my_cast.sql").exists());
    assert!(output_dir.join("_global/operator/my_op.sql").exists());
    assert!(!output_dir.join("public/unknown/my_cast.sql").exists());
    assert!(!output_dir.join("public/unknown/my_op.sql").exists());
}

#[test]
fn test_import_pg_dump_attaches_global_acl_and_owner_statements() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = temp_dir.path().join("database.sql");
    let output_dir = temp_dir.path().join("output");

    let content = r#"
-- Name: postgres_fdw; Type: FOREIGN DATA WRAPPER; Schema: -; Owner: postgres

CREATE FOREIGN DATA WRAPPER "postgres_fdw";

-- Name: remote_server; Type: SERVER; Schema: -; Owner: postgres

CREATE SERVER "remote_server" FOREIGN DATA WRAPPER "postgres_fdw";

-- Name: remote_server; Type: ACL; Schema: -; Owner: postgres

GRANT USAGE ON FOREIGN SERVER "remote_server" TO "reader";

-- Name: ddl_notify; Type: EVENT TRIGGER; Schema: -; Owner: postgres

CREATE EVENT TRIGGER "ddl_notify"
    ON ddl_command_end
    EXECUTE FUNCTION "public"."notify_ddl"();

-- Name: ddl_notify; Type: ACL; Schema: -; Owner: postgres

ALTER EVENT TRIGGER "ddl_notify" OWNER TO "admin";
"#;
    fs::write(&dump_file, content).unwrap();

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let server_file = output_dir.join("_global/fdw/server/remote_server.sql");
    let server_content = fs::read_to_string(&server_file).unwrap();
    assert!(
        server_content.contains(r#"GRANT USAGE ON FOREIGN SERVER "remote_server" TO "reader";"#)
    );

    let event_trigger_file = output_dir.join("_global/event_trigger/ddl_notify.sql");
    let event_trigger_content = fs::read_to_string(&event_trigger_file).unwrap();
    assert!(
        event_trigger_content.contains(r#"ALTER EVENT TRIGGER "ddl_notify" OWNER TO "admin";"#)
    );
}

#[test]
fn test_import_pg_dump_multi_word_name_prefixes_are_stripped() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = temp_dir.path().join("database.sql");
    let output_dir = temp_dir.path().join("output");

    let content = r#"
-- Name: MATERIALIZED VIEW report_summary; Type: MATERIALIZED VIEW; Schema: public; Owner: postgres

CREATE MATERIALIZED VIEW "public"."report_summary" AS SELECT 1;

-- Name: FOREIGN TABLE remote_data; Type: FOREIGN TABLE; Schema: public; Owner: postgres

CREATE FOREIGN TABLE "public"."remote_data" (id integer) SERVER "remote_server";

-- Name: EVENT TRIGGER ddl_notify; Type: EVENT TRIGGER; Schema: -; Owner: postgres

CREATE EVENT TRIGGER "ddl_notify"
    ON ddl_command_end
    EXECUTE FUNCTION "public"."notify_ddl"();
"#;
    fs::write(&dump_file, content).unwrap();

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        output_dir
            .join("public/materialized_view/report_summary.sql")
            .exists()
    );
    assert!(
        output_dir
            .join("public/foreign_table/remote_data.sql")
            .exists()
    );
    assert!(
        output_dir
            .join("_global/event_trigger/ddl_notify.sql")
            .exists()
    );
}

#[test]
fn test_import_pg_dump_deduplicates_acl_and_owner_from_acl_objects() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = temp_dir.path().join("database.sql");
    let output_dir = temp_dir.path().join("output");

    let grant_stmt = r#"GRANT SELECT ON TABLE "public"."users" TO "reader";"#;
    let owner_stmt = r#"ALTER TABLE "public"."users" OWNER TO "admin";"#;
    let content = format!(
        r#"
-- Name: users; Type: TABLE; Schema: public; Owner: postgres

CREATE TABLE "public"."users" (id integer);

-- Name: TABLE users; Type: ACL; Schema: public; Owner: postgres

{grant_stmt}

-- Name: TABLE users; Type: ACL; Schema: public; Owner: postgres

{owner_stmt}
"#
    );
    fs::write(&dump_file, content).unwrap();

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let table_file = output_dir.join("public/table/users.sql");
    assert!(table_file.exists(), "expected table file at {table_file:?}");
    let table_content = fs::read_to_string(table_file).unwrap();
    assert_eq!(table_content.matches(grant_stmt).count(), 1);
    assert_eq!(table_content.matches(owner_stmt).count(), 1);
}

#[test]
fn test_import_pg_dump_rejects_path_traversal_in_object_name() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = temp_dir.path().join("database.sql");
    let output_dir = temp_dir.path().join("output");
    let escaped_target = temp_dir.path().join("outside.sql");

    let content = r#"
-- Name: ../../../../outside; Type: TABLE; Schema: public; Owner: postgres

CREATE TABLE "public"."outside" (id integer);
"#;
    fs::write(&dump_file, content).unwrap();

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsafe object name"));

    assert!(
        !escaped_target.exists(),
        "import should not create files outside output dir"
    );
}

#[test]
fn test_import_pg_dump_rejects_path_traversal_in_schema_name() {
    let temp_dir = TempDir::new().unwrap();
    let dump_file = temp_dir.path().join("database.sql");
    let output_dir = temp_dir.path().join("output");

    let content = r#"
-- Name: users; Type: TABLE; Schema: ../../../../tmp; Owner: postgres

CREATE TABLE users (id integer);
"#;
    fs::write(&dump_file, content).unwrap();

    topcat_cmd()
        .args([
            "import",
            "pg-dump",
            dump_file.to_str().unwrap(),
            output_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsafe schema name"));
}
