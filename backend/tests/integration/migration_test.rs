//! migration 契约集成测试。
//!
//! 这些测试不连接真实数据库，只固定跨方言 migration 文件的版本同步和逻辑删除唯一约束策略。

#[test]
fn active_unique_index_migration_exists_for_mysql_and_postgres() {
    // MySQL 和 PostgreSQL 必须用同一版本号修复逻辑删除唯一约束，避免兼容脚本漂移。
    let mysql = include_str!(
        "../../migrations/mysql/V202604280002__fix_app_metadata_active_unique_index.sql"
    );
    let postgres = include_str!(
        "../../migrations/postgres/V202604280002__fix_app_metadata_active_unique_index.sql"
    );

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
    let mysql =
        include_str!("../../migrations/mysql/V202604280003__create_business_translations.sql");
    let postgres =
        include_str!("../../migrations/postgres/V202604280003__create_business_translations.sql");

    assert!(mysql.contains("CREATE TABLE business_translations"));
    assert!(mysql.contains("active_translation_key"));
    assert!(mysql.contains("uk_business_translations_active_key"));
    assert!(postgres.contains("CREATE TABLE business_translations"));
    assert!(postgres.contains("WHERE deleted = FALSE"));
    assert!(postgres.contains("uk_business_translations_active_key"));
}
