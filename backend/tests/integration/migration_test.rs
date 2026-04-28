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
