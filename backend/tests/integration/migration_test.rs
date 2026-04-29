//! migration 契约集成测试。
//!
//! 这些测试不连接真实数据库，只固定跨方言 migration 文件的版本同步、Refinery 文件名约束
//! 和逻辑删除唯一约束策略。

use std::{collections::BTreeMap, fs, path::Path};

#[test]
fn active_unique_index_migration_exists_for_mysql_and_postgres() {
    // MySQL 和 PostgreSQL 必须用同一版本号修复逻辑删除唯一约束，避免兼容脚本漂移。
    let mysql = include_str!("../../migrations/mysql/V2__fix_app_metadata_active_unique_index.sql");
    let postgres =
        include_str!("../../migrations/postgres/V2__fix_app_metadata_active_unique_index.sql");

    assert!(mysql.contains("DROP INDEX uk_app_metadata_key_deleted"));
    assert!(mysql.contains("active_metadata_key"));
    assert!(mysql.contains("uk_app_metadata_active_key"));
    assert!(postgres.contains("DROP INDEX uk_app_metadata_key_deleted"));
    assert!(postgres.contains("WHERE deleted = FALSE"));
    assert!(postgres.contains("uk_app_metadata_active_key"));
}

#[test]
fn business_translation_migration_exists_for_mysql_and_postgres() {
    // 业务翻译表必须在 MySQL 和 PostgreSQL 中保持同版本，并只约束未删除翻译唯一性。
    let mysql = include_str!("../../migrations/mysql/V3__create_business_translations.sql");
    let postgres = include_str!("../../migrations/postgres/V3__create_business_translations.sql");

    assert!(mysql.contains("CREATE TABLE business_translations"));
    assert!(mysql.contains("active_translation_key"));
    assert!(mysql.contains("uk_business_translations_active_key"));
    assert!(postgres.contains("CREATE TABLE business_translations"));
    assert!(postgres.contains("WHERE deleted = FALSE"));
    assert!(postgres.contains("uk_business_translations_active_key"));
}

#[test]
fn hrm_migration_exists_for_mysql_and_postgres() {
    // HRM 主数据表必须在 MySQL 和 PostgreSQL 中保持同版本，并固定租户内唯一和主关系唯一约束。
    let mysql = include_str!("../../migrations/mysql/V4__create_hrm_tables.sql");
    let postgres = include_str!("../../migrations/postgres/V4__create_hrm_tables.sql");

    for table in ["hrm_users", "hrm_orgs", "hrm_posts", "hrm_user_org_posts"] {
        assert!(mysql.contains(&format!("CREATE TABLE {table}")));
        assert!(postgres.contains(&format!("CREATE TABLE {table}")));
    }

    for column in [
        "tenant_id",
        "name_full_pinyin",
        "name_simple_pinyin",
        "sort_no",
    ] {
        assert!(mysql.contains(column));
        assert!(postgres.contains(column));
    }

    assert!(mysql.contains("active_employee_key"));
    assert!(mysql.contains("active_org_key"));
    assert!(mysql.contains("active_post_key"));
    assert!(mysql.contains("active_primary_org_key"));
    assert!(mysql.contains("active_primary_post_key"));
    assert!(postgres.contains("WHERE deleted = FALSE"));
    assert!(postgres.contains("WHERE deleted = FALSE AND primary_org = TRUE"));
    assert!(postgres.contains("WHERE deleted = FALSE AND primary_post = TRUE"));
}

#[test]
fn tenant_migration_exists_for_mysql_and_postgres() {
    // 租户主数据表必须在 MySQL 和 PostgreSQL 中保持同版本，并初始化默认租户。
    let mysql = include_str!("../../migrations/mysql/V5__create_tenant_tables.sql");
    let postgres = include_str!("../../migrations/postgres/V5__create_tenant_tables.sql");

    for migration in [mysql, postgres] {
        assert!(migration.contains("CREATE TABLE sys_tenants"));
        assert!(migration.contains("tenant_code"));
        assert!(migration.contains("name_full_pinyin"));
        assert!(migration.contains("name_simple_pinyin"));
        assert!(migration.contains("isolation_mode"));
        assert!(migration.contains("'default'"));
        assert!(migration.contains("'shared_schema'"));
    }

    assert!(mysql.contains("active_tenant_code"));
    assert!(mysql.contains("uk_sys_tenants_active_code"));
    assert!(postgres.contains("WHERE deleted = FALSE"));
    assert!(postgres.contains("uk_sys_tenants_active_code"));
}

#[test]
fn auth_migration_exists_for_mysql_and_postgres() {
    // 认证首版只创建账号、账号租户关系、刷新令牌和审计表，不包含角色或权限表。
    let mysql = include_str!("../../migrations/mysql/V6__create_auth_tables.sql");
    let postgres = include_str!("../../migrations/postgres/V6__create_auth_tables.sql");

    for migration in [mysql, postgres] {
        for table in [
            "sys_accounts",
            "sys_account_tenants",
            "sys_refresh_tokens",
            "sys_auth_audit_logs",
        ] {
            assert!(migration.contains(&format!("CREATE TABLE {table}")));
        }
        assert!(migration.contains("display_name_full_pinyin"));
        assert!(migration.contains("display_name_simple_pinyin"));
        assert!(migration.contains("token_hash"));
        assert!(migration.contains("token_family"));
        assert!(!migration.contains(" mobile "));
        assert!(!migration.contains(" email "));
        assert!(!migration.contains("sys_roles"));
        assert!(!migration.contains("sys_permissions"));
    }

    assert!(mysql.contains("active_username"));
    assert!(mysql.contains("active_relation_key"));
    assert!(mysql.contains("active_default_key"));
    assert!(postgres.contains("WHERE deleted = FALSE"));
    assert!(postgres.contains("WHERE deleted = FALSE AND is_default = TRUE"));
}

#[test]
fn persistent_tables_include_unified_base_fields() {
    // 当前所有已创建的持久化表都必须包含统一基础字段，避免后续业务模块字段语义漂移。
    let migrations = [
        include_str!("../../migrations/mysql/V1__init_foundation.sql"),
        include_str!("../../migrations/postgres/V1__init_foundation.sql"),
        include_str!("../../migrations/mysql/V3__create_business_translations.sql"),
        include_str!("../../migrations/postgres/V3__create_business_translations.sql"),
        include_str!("../../migrations/mysql/V4__create_hrm_tables.sql"),
        include_str!("../../migrations/postgres/V4__create_hrm_tables.sql"),
        include_str!("../../migrations/mysql/V5__create_tenant_tables.sql"),
        include_str!("../../migrations/postgres/V5__create_tenant_tables.sql"),
        include_str!("../../migrations/mysql/V6__create_auth_tables.sql"),
        include_str!("../../migrations/postgres/V6__create_auth_tables.sql"),
    ];
    let required_columns = [
        "version",
        "deleted",
        "created_by",
        "created_time",
        "updated_by",
        "updated_time",
        "deleted_by",
        "deleted_time",
    ];

    for migration in migrations {
        for column in required_columns {
            assert!(
                migration.contains(column),
                "migration 缺少统一基础字段: {column}"
            );
        }
    }
}

#[test]
fn repository_sql_contract_keeps_version_and_logical_delete_guards() {
    // repository 可以显式写 SQL，但更新、逻辑删除和默认查询的基础约束必须保持一致。
    let repository = include_str!("../../src/i18n/business_translation_repository.rs");

    assert!(repository.contains("WHERE deleted = FALSE"));
    assert!(repository.contains("SET deleted = TRUE, version = "));
    assert!(repository.contains("deleted_by = "));
    assert!(repository.contains("deleted_time = CURRENT_TIMESTAMP"));
    assert!(repository.contains("FOR UPDATE"));
}

#[test]
fn migration_versions_are_refinery_i32_and_paired() {
    // Refinery 0.8 会把文件名中的版本号解析为 i32；超出范围会在 embed_migrations 阶段 panic。
    let mysql_versions = migration_versions("migrations/mysql");
    let postgres_versions = migration_versions("migrations/postgres");

    assert_eq!(mysql_versions, postgres_versions);
}

fn migration_versions(relative_dir: &str) -> BTreeMap<i32, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_dir);
    let mut versions = BTreeMap::new();

    for entry in fs::read_dir(&root).expect("migration 目录必须可读取") {
        let path = entry.expect("migration 文件必须可读取").path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("migration 文件名必须是 UTF-8");
        if !file_name.ends_with(".sql") {
            continue;
        }

        let (version, name) = parse_refinery_file_name(file_name);
        assert!(
            versions.insert(version, name).is_none(),
            "migration 版本号不得重复: {version}"
        );
    }

    versions
}

fn parse_refinery_file_name(file_name: &str) -> (i32, String) {
    let stem = file_name
        .strip_suffix(".sql")
        .expect("migration 文件必须使用 .sql 后缀");
    let rest = stem
        .strip_prefix('V')
        .expect("migration 文件必须使用 V 前缀");
    let (version, name) = rest
        .split_once("__")
        .expect("migration 文件名必须使用 V{version}__{name}.sql 格式");
    let parsed_version = version
        .parse::<i32>()
        .expect("migration 版本号必须能被 Refinery 解析为 i32");

    (parsed_version, name.to_string())
}
