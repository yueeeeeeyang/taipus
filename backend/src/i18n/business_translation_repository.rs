//! 业务翻译 repository。
//!
//! SQL 方言差异统一封装在本模块，业务 service 只依赖 `DatabasePool` 和稳定数据结构，避免
//! MySQL/PostgreSQL 分支散落到各业务模块。

use sqlx::{Acquire, MySql, MySqlConnection, Postgres, QueryBuilder, Row, Transaction};

use crate::{
    db::executor::DatabasePool,
    error::{
        app_error::{AppError, AppResult},
        error_code::ErrorCode,
    },
    i18n::business_translation::{
        BusinessTranslationFieldMap, BusinessTranslationReadResponse, BusinessTranslationValue,
    },
    utils::id::generate_business_id,
};

/// MySQL active 翻译 upsert 语句。
///
/// 唯一约束由生成列 `active_translation_key` 提供，只约束未删除记录。
pub const MYSQL_UPSERT_ACTIVE_TRANSLATION_SQL: &str = r#"
INSERT INTO business_translations (
    id, resource_type, resource_id, field_name, locale, text_value, version, deleted, created_by, updated_by
) VALUES (?, ?, ?, ?, ?, ?, ?, FALSE, ?, ?)
ON DUPLICATE KEY UPDATE
    text_value = VALUES(text_value),
    version = VALUES(version),
    deleted = FALSE,
    updated_by = VALUES(updated_by),
    updated_time = CURRENT_TIMESTAMP(6),
    deleted_by = NULL,
    deleted_time = NULL
"#;

/// PostgreSQL active 翻译 upsert 语句。
///
/// `WHERE deleted = FALSE` 必须与 partial unique index 条件一致。
pub const POSTGRES_UPSERT_ACTIVE_TRANSLATION_SQL: &str = r#"
INSERT INTO business_translations (
    id, resource_type, resource_id, field_name, locale, text_value, version, deleted, created_by, updated_by
) VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, $8, $9)
ON CONFLICT (resource_type, resource_id, field_name, locale) WHERE deleted = FALSE
DO UPDATE SET
    text_value = EXCLUDED.text_value,
    version = EXCLUDED.version,
    deleted = FALSE,
    updated_by = EXCLUDED.updated_by,
    updated_time = CURRENT_TIMESTAMP,
    deleted_by = NULL,
    deleted_time = NULL
"#;

/// MySQL 资源级命名锁获取语句。
///
/// 新资源首写时没有 active 行可 `FOR UPDATE`，因此必须额外使用资源级锁串行化版本检查。
pub const MYSQL_ACQUIRE_RESOURCE_LOCK_SQL: &str = "SELECT GET_LOCK(?, ?)";

/// MySQL 资源级命名锁释放语句。
///
/// MySQL 命名锁绑定连接而不是事务，提交或回滚后必须在同一连接上显式释放。
pub const MYSQL_RELEASE_RESOURCE_LOCK_SQL: &str = "SELECT RELEASE_LOCK(?)";

/// PostgreSQL 事务级 advisory lock 获取语句。
///
/// 事务级锁会在 commit/rollback 后自动释放，适合保护新资源首写的版本检查。
pub const POSTGRES_ACQUIRE_RESOURCE_LOCK_SQL: &str = "SELECT pg_advisory_xact_lock($1)";

const MYSQL_RESOURCE_LOCK_TIMEOUT_SECONDS: i64 = 10;

#[derive(Debug, Clone, Default)]
pub struct BusinessTranslationRepository;

impl BusinessTranslationRepository {
    /// 创建业务翻译 repository。
    pub fn new() -> Self {
        Self
    }

    /// 批量读取 active 翻译。
    ///
    /// 该方法面向列表本地化场景，调用方应把同一页数据一次性传入，避免 N+1 查询。
    pub async fn find_active(
        &self,
        database: &DatabasePool,
        resource_type: &str,
        resource_ids: &[String],
        field_names: &[String],
        locales: Option<&[String]>,
    ) -> AppResult<Vec<BusinessTranslationValue>> {
        if resource_ids.is_empty() || field_names.is_empty() {
            return Ok(Vec::new());
        }

        match database {
            DatabasePool::MySql(pool) => {
                find_active_mysql(pool, resource_type, resource_ids, field_names, locales).await
            }
            DatabasePool::Postgres(pool) => {
                find_active_postgres(pool, resource_type, resource_ids, field_names, locales).await
            }
        }
    }

    /// 读取单个资源的 active 翻译集合。
    pub async fn read_resource(
        &self,
        database: &DatabasePool,
        resource_type: &str,
        resource_id: &str,
        field_names: &[String],
    ) -> AppResult<BusinessTranslationReadResponse> {
        let resource_ids = vec![resource_id.to_string()];
        let values = self
            .find_active(database, resource_type, &resource_ids, field_names, None)
            .await?;
        let version = current_active_version(database, resource_type, resource_id).await?;
        Ok(read_response_from_values(
            resource_type,
            resource_id,
            field_names,
            version,
            values,
        ))
    }

    /// 按字段批量覆盖写入翻译。
    ///
    /// 请求中包含的字段会整体覆盖该字段下的 locale 集合；未提交的旧 locale 翻译会被逻辑删除。
    pub async fn replace_resource_fields(
        &self,
        database: &DatabasePool,
        resource_type: &str,
        resource_id: &str,
        expected_version: i64,
        fields: &BusinessTranslationFieldMap,
        actor: &str,
    ) -> AppResult<BusinessTranslationReadResponse> {
        match database {
            DatabasePool::MySql(pool) => {
                replace_resource_fields_mysql(
                    pool,
                    resource_type,
                    resource_id,
                    expected_version,
                    fields,
                    actor,
                )
                .await?
            }
            DatabasePool::Postgres(pool) => {
                replace_resource_fields_postgres(
                    pool,
                    resource_type,
                    resource_id,
                    expected_version,
                    fields,
                    actor,
                )
                .await?
            }
        }

        let field_names = fields.keys().cloned().collect::<Vec<_>>();
        self.read_resource(database, resource_type, resource_id, &field_names)
            .await
    }
}

async fn find_active_mysql(
    pool: &sqlx::Pool<MySql>,
    resource_type: &str,
    resource_ids: &[String],
    field_names: &[String],
    locales: Option<&[String]>,
) -> AppResult<Vec<BusinessTranslationValue>> {
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT resource_id, field_name, locale, text_value, version \
         FROM business_translations \
         WHERE deleted = FALSE AND resource_type = ",
    );
    builder.push_bind(resource_type);
    push_mysql_in(&mut builder, "resource_id", resource_ids);
    push_mysql_in(&mut builder, "field_name", field_names);
    if let Some(locales) = locales.filter(|values| !values.is_empty()) {
        push_mysql_in(&mut builder, "locale", locales);
    }
    builder.push(" ORDER BY resource_id, field_name, locale");

    let rows = builder.build().fetch_all(pool).await?;
    rows.into_iter()
        .map(mysql_row_to_translation_value)
        .collect()
}

async fn find_active_postgres(
    pool: &sqlx::Pool<Postgres>,
    resource_type: &str,
    resource_ids: &[String],
    field_names: &[String],
    locales: Option<&[String]>,
) -> AppResult<Vec<BusinessTranslationValue>> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT resource_id, field_name, locale, text_value, version \
         FROM business_translations \
         WHERE deleted = FALSE AND resource_type = ",
    );
    builder.push_bind(resource_type);
    push_postgres_in(&mut builder, "resource_id", resource_ids);
    push_postgres_in(&mut builder, "field_name", field_names);
    if let Some(locales) = locales.filter(|values| !values.is_empty()) {
        push_postgres_in(&mut builder, "locale", locales);
    }
    builder.push(" ORDER BY resource_id, field_name, locale");

    let rows = builder.build().fetch_all(pool).await?;
    rows.into_iter()
        .map(postgres_row_to_translation_value)
        .collect()
}

async fn replace_resource_fields_mysql(
    pool: &sqlx::Pool<MySql>,
    resource_type: &str,
    resource_id: &str,
    expected_version: i64,
    fields: &BusinessTranslationFieldMap,
    actor: &str,
) -> AppResult<()> {
    let mut conn = pool.acquire().await?;
    let lock_key = acquire_mysql_resource_lock(&mut conn, resource_type, resource_id).await?;
    let write_result = match conn.begin().await {
        Ok(mut tx) => {
            let operation_result = apply_resource_fields_mysql_tx(
                &mut tx,
                resource_type,
                resource_id,
                expected_version,
                fields,
                actor,
            )
            .await;
            match operation_result {
                Ok(()) => tx.commit().await.map_err(AppError::from),
                Err(err) => match tx.rollback().await {
                    Ok(()) => Err(err),
                    Err(rollback_err) => Err(AppError::system(format!(
                        "业务翻译事务回滚失败: {rollback_err}; 原始错误: {}",
                        describe_app_error(&err)
                    ))),
                },
            }
        }
        Err(err) => Err(AppError::from(err)),
    };
    let release_result = release_mysql_resource_lock(&mut conn, &lock_key).await;

    match (write_result, release_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Err(_write_err), Err(release_err)) => Err(release_err),
    }
}

async fn apply_resource_fields_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
    resource_type: &str,
    resource_id: &str,
    expected_version: i64,
    fields: &BusinessTranslationFieldMap,
    actor: &str,
) -> AppResult<()> {
    let current_version = current_active_version_mysql_tx(tx, resource_type, resource_id).await?;
    ensure_expected_version(
        resource_type,
        resource_id,
        expected_version,
        current_version,
    )?;
    let next_version = current_version + 1;

    for (field_name, locale_values) in fields {
        let submitted_locales = locale_values.keys().cloned().collect::<Vec<_>>();
        logically_delete_missing_locales_mysql(
            tx,
            resource_type,
            resource_id,
            field_name,
            &submitted_locales,
            next_version,
            actor,
        )
        .await?;

        for (locale, text_value) in locale_values {
            upsert_active_translation_mysql(
                tx,
                resource_type,
                resource_id,
                field_name,
                locale,
                text_value,
                next_version,
                actor,
            )
            .await?;
        }
    }

    Ok(())
}

async fn replace_resource_fields_postgres(
    pool: &sqlx::Pool<Postgres>,
    resource_type: &str,
    resource_id: &str,
    expected_version: i64,
    fields: &BusinessTranslationFieldMap,
    actor: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    acquire_postgres_resource_lock(&mut tx, resource_type, resource_id).await?;
    let current_version =
        current_active_version_postgres_tx(&mut tx, resource_type, resource_id).await?;
    ensure_expected_version(
        resource_type,
        resource_id,
        expected_version,
        current_version,
    )?;
    let next_version = current_version + 1;

    for (field_name, locale_values) in fields {
        let submitted_locales = locale_values.keys().cloned().collect::<Vec<_>>();
        logically_delete_missing_locales_postgres(
            &mut tx,
            resource_type,
            resource_id,
            field_name,
            &submitted_locales,
            next_version,
            actor,
        )
        .await?;

        for (locale, text_value) in locale_values {
            upsert_active_translation_postgres(
                &mut tx,
                resource_type,
                resource_id,
                field_name,
                locale,
                text_value,
                next_version,
                actor,
            )
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn current_active_version(
    database: &DatabasePool,
    resource_type: &str,
    resource_id: &str,
) -> AppResult<i64> {
    match database {
        DatabasePool::MySql(pool) => {
            let version = sqlx::query_scalar::<_, i64>(
                "SELECT COALESCE(MAX(version), 0) \
                 FROM business_translations \
                 WHERE resource_type = ? AND resource_id = ? AND deleted = FALSE",
            )
            .bind(resource_type)
            .bind(resource_id)
            .fetch_one(pool)
            .await?;
            Ok(version)
        }
        DatabasePool::Postgres(pool) => {
            let version = sqlx::query_scalar::<_, i64>(
                "SELECT COALESCE(MAX(version), 0) \
                 FROM business_translations \
                 WHERE resource_type = $1 AND resource_id = $2 AND deleted = FALSE",
            )
            .bind(resource_type)
            .bind(resource_id)
            .fetch_one(pool)
            .await?;
            Ok(version)
        }
    }
}

async fn acquire_mysql_resource_lock(
    conn: &mut MySqlConnection,
    resource_type: &str,
    resource_id: &str,
) -> AppResult<String> {
    let lock_key = business_translation_lock_key(resource_type, resource_id);
    let acquired = sqlx::query_scalar::<_, Option<i64>>(MYSQL_ACQUIRE_RESOURCE_LOCK_SQL)
        .bind(&lock_key)
        .bind(MYSQL_RESOURCE_LOCK_TIMEOUT_SECONDS)
        .fetch_one(&mut *conn)
        .await?;

    if acquired == Some(1) {
        Ok(lock_key)
    } else {
        Err(resource_lock_conflict(resource_type, resource_id))
    }
}

async fn release_mysql_resource_lock(conn: &mut MySqlConnection, lock_key: &str) -> AppResult<()> {
    let released = sqlx::query_scalar::<_, Option<i64>>(MYSQL_RELEASE_RESOURCE_LOCK_SQL)
        .bind(lock_key)
        .fetch_one(&mut *conn)
        .await?;

    if released == Some(1) {
        Ok(())
    } else {
        Err(AppError::system(format!(
            "业务翻译资源锁释放失败: {lock_key}"
        )))
    }
}

async fn acquire_postgres_resource_lock(
    tx: &mut Transaction<'_, Postgres>,
    resource_type: &str,
    resource_id: &str,
) -> AppResult<()> {
    let lock_id = business_translation_lock_id(resource_type, resource_id);
    sqlx::query(POSTGRES_ACQUIRE_RESOURCE_LOCK_SQL)
        .bind(lock_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn current_active_version_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
    resource_type: &str,
    resource_id: &str,
) -> AppResult<i64> {
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT version \
         FROM business_translations \
         WHERE resource_type = ? AND resource_id = ? AND deleted = FALSE \
         ORDER BY version DESC \
         LIMIT 1 \
         FOR UPDATE",
    )
    .bind(resource_type)
    .bind(resource_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(version.unwrap_or(0))
}

async fn current_active_version_postgres_tx(
    tx: &mut Transaction<'_, Postgres>,
    resource_type: &str,
    resource_id: &str,
) -> AppResult<i64> {
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT version \
         FROM business_translations \
         WHERE resource_type = $1 AND resource_id = $2 AND deleted = FALSE \
         ORDER BY version DESC \
         LIMIT 1 \
         FOR UPDATE",
    )
    .bind(resource_type)
    .bind(resource_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(version.unwrap_or(0))
}

async fn logically_delete_missing_locales_mysql(
    tx: &mut Transaction<'_, MySql>,
    resource_type: &str,
    resource_id: &str,
    field_name: &str,
    submitted_locales: &[String],
    next_version: i64,
    actor: &str,
) -> AppResult<()> {
    let mut builder = QueryBuilder::<MySql>::new(
        "UPDATE business_translations \
         SET deleted = TRUE, version = ",
    );
    builder.push_bind(next_version);
    builder.push(", updated_by = ");
    builder.push_bind(actor);
    builder.push(", updated_time = CURRENT_TIMESTAMP(6), deleted_by = ");
    builder.push_bind(actor);
    builder.push(
        ", deleted_time = CURRENT_TIMESTAMP(6) \
         WHERE deleted = FALSE AND resource_type = ",
    );
    builder.push_bind(resource_type);
    builder.push(" AND resource_id = ");
    builder.push_bind(resource_id);
    builder.push(" AND field_name = ");
    builder.push_bind(field_name);
    if !submitted_locales.is_empty() {
        push_mysql_not_in(&mut builder, "locale", submitted_locales);
    }
    builder.build().execute(&mut **tx).await?;
    Ok(())
}

async fn logically_delete_missing_locales_postgres(
    tx: &mut Transaction<'_, Postgres>,
    resource_type: &str,
    resource_id: &str,
    field_name: &str,
    submitted_locales: &[String],
    next_version: i64,
    actor: &str,
) -> AppResult<()> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "UPDATE business_translations \
         SET deleted = TRUE, version = ",
    );
    builder.push_bind(next_version);
    builder.push(", updated_by = ");
    builder.push_bind(actor);
    builder.push(", updated_time = CURRENT_TIMESTAMP, deleted_by = ");
    builder.push_bind(actor);
    builder.push(
        ", deleted_time = CURRENT_TIMESTAMP \
         WHERE deleted = FALSE AND resource_type = ",
    );
    builder.push_bind(resource_type);
    builder.push(" AND resource_id = ");
    builder.push_bind(resource_id);
    builder.push(" AND field_name = ");
    builder.push_bind(field_name);
    if !submitted_locales.is_empty() {
        push_postgres_not_in(&mut builder, "locale", submitted_locales);
    }
    builder.build().execute(&mut **tx).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn upsert_active_translation_mysql(
    tx: &mut Transaction<'_, MySql>,
    resource_type: &str,
    resource_id: &str,
    field_name: &str,
    locale: &str,
    text_value: &str,
    version: i64,
    actor: &str,
) -> AppResult<()> {
    sqlx::query(MYSQL_UPSERT_ACTIVE_TRANSLATION_SQL)
        .bind(generate_business_id())
        .bind(resource_type)
        .bind(resource_id)
        .bind(field_name)
        .bind(locale)
        .bind(text_value)
        .bind(version)
        .bind(actor)
        .bind(actor)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn upsert_active_translation_postgres(
    tx: &mut Transaction<'_, Postgres>,
    resource_type: &str,
    resource_id: &str,
    field_name: &str,
    locale: &str,
    text_value: &str,
    version: i64,
    actor: &str,
) -> AppResult<()> {
    sqlx::query(POSTGRES_UPSERT_ACTIVE_TRANSLATION_SQL)
        .bind(generate_business_id())
        .bind(resource_type)
        .bind(resource_id)
        .bind(field_name)
        .bind(locale)
        .bind(text_value)
        .bind(version)
        .bind(actor)
        .bind(actor)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn ensure_expected_version(
    resource_type: &str,
    resource_id: &str,
    expected_version: i64,
    current_version: i64,
) -> AppResult<()> {
    if expected_version == current_version {
        return Ok(());
    }

    Err(AppError::new(
        ErrorCode::Conflict,
        ErrorCode::Conflict.default_message(),
    )
    .with_internal_message(format!(
        "业务翻译版本冲突: resource_type={resource_type}, resource_id={resource_id}, expected={expected_version}, current={current_version}"
    )))
}

fn resource_lock_conflict(resource_type: &str, resource_id: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, ErrorCode::Conflict.default_message()).with_internal_message(
        format!("业务翻译资源锁等待超时: resource_type={resource_type}, resource_id={resource_id}"),
    )
}

fn describe_app_error(err: &AppError) -> String {
    err.internal_message
        .clone()
        .unwrap_or_else(|| err.message.clone())
}

fn business_translation_lock_key(resource_type: &str, resource_id: &str) -> String {
    format!(
        "bt:{:016x}",
        stable_business_translation_hash(resource_type, resource_id)
    )
}

fn business_translation_lock_id(resource_type: &str, resource_id: &str) -> i64 {
    stable_business_translation_hash(resource_type, resource_id) as i64
}

fn stable_business_translation_hash(resource_type: &str, resource_id: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in resource_type
        .bytes()
        .chain(std::iter::once(0x1f))
        .chain(resource_id.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn read_response_from_values(
    resource_type: &str,
    resource_id: &str,
    field_names: &[String],
    version: i64,
    values: Vec<BusinessTranslationValue>,
) -> BusinessTranslationReadResponse {
    let mut fields = BusinessTranslationFieldMap::new();
    for field_name in field_names {
        fields.entry(field_name.clone()).or_default();
    }
    for value in values {
        fields
            .entry(value.field_name)
            .or_default()
            .insert(value.locale, value.text_value);
    }

    BusinessTranslationReadResponse {
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
        version,
        fields,
    }
}

fn mysql_row_to_translation_value(
    row: sqlx::mysql::MySqlRow,
) -> AppResult<BusinessTranslationValue> {
    Ok(BusinessTranslationValue {
        resource_id: row.try_get("resource_id")?,
        field_name: row.try_get("field_name")?,
        locale: row.try_get("locale")?,
        text_value: row.try_get("text_value")?,
        version: row.try_get("version")?,
    })
}

fn postgres_row_to_translation_value(
    row: sqlx::postgres::PgRow,
) -> AppResult<BusinessTranslationValue> {
    Ok(BusinessTranslationValue {
        resource_id: row.try_get("resource_id")?,
        field_name: row.try_get("field_name")?,
        locale: row.try_get("locale")?,
        text_value: row.try_get("text_value")?,
        version: row.try_get("version")?,
    })
}

fn push_mysql_in<'args>(
    builder: &mut QueryBuilder<'args, MySql>,
    column: &str,
    values: &'args [String],
) {
    builder.push(" AND ");
    builder.push(column);
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for value in values {
        separated.push_bind(value);
    }
    separated.push_unseparated(")");
}

fn push_postgres_in<'args>(
    builder: &mut QueryBuilder<'args, Postgres>,
    column: &str,
    values: &'args [String],
) {
    builder.push(" AND ");
    builder.push(column);
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for value in values {
        separated.push_bind(value);
    }
    separated.push_unseparated(")");
}

fn push_mysql_not_in<'args>(
    builder: &mut QueryBuilder<'args, MySql>,
    column: &str,
    values: &'args [String],
) {
    builder.push(" AND ");
    builder.push(column);
    builder.push(" NOT IN (");
    let mut separated = builder.separated(", ");
    for value in values {
        separated.push_bind(value);
    }
    separated.push_unseparated(")");
}

fn push_postgres_not_in<'args>(
    builder: &mut QueryBuilder<'args, Postgres>,
    column: &str,
    values: &'args [String],
) {
    builder.push(" AND ");
    builder.push(column);
    builder.push(" NOT IN (");
    let mut separated = builder.separated(", ");
    for value in values {
        separated.push_bind(value);
    }
    separated.push_unseparated(")");
}

#[cfg(test)]
mod tests {
    use super::{
        MYSQL_ACQUIRE_RESOURCE_LOCK_SQL, MYSQL_RELEASE_RESOURCE_LOCK_SQL,
        MYSQL_UPSERT_ACTIVE_TRANSLATION_SQL, POSTGRES_ACQUIRE_RESOURCE_LOCK_SQL,
        POSTGRES_UPSERT_ACTIVE_TRANSLATION_SQL, business_translation_lock_key,
    };

    #[test]
    fn mysql_upsert_targets_active_unique_key() {
        // MySQL 依赖 active_translation_key 唯一约束触发 upsert，必须保留对应语句形态。
        assert!(MYSQL_UPSERT_ACTIVE_TRANSLATION_SQL.contains("ON DUPLICATE KEY UPDATE"));
        assert!(MYSQL_UPSERT_ACTIVE_TRANSLATION_SQL.contains("deleted = FALSE"));
    }

    #[test]
    fn postgres_upsert_matches_partial_unique_index() {
        // PostgreSQL 的冲突条件必须与 partial unique index 完全一致，否则 active 记录会重复。
        assert!(POSTGRES_UPSERT_ACTIVE_TRANSLATION_SQL.contains("ON CONFLICT"));
        assert!(POSTGRES_UPSERT_ACTIVE_TRANSLATION_SQL.contains("WHERE deleted = FALSE"));
    }

    #[test]
    fn resource_lock_sql_covers_mysql_and_postgres() {
        // 新资源首写没有行锁可用，必须保留资源级 advisory lock 语句保护乐观锁语义。
        assert!(MYSQL_ACQUIRE_RESOURCE_LOCK_SQL.contains("GET_LOCK"));
        assert!(MYSQL_RELEASE_RESOURCE_LOCK_SQL.contains("RELEASE_LOCK"));
        assert!(POSTGRES_ACQUIRE_RESOURCE_LOCK_SQL.contains("pg_advisory_xact_lock"));
    }

    #[test]
    fn mysql_resource_lock_key_stays_within_named_lock_limit() {
        // MySQL 命名锁 key 最长 64 字节，长业务 ID 也必须被压缩成稳定短 key。
        let lock_key = business_translation_lock_key(
            "form_definition",
            "resource_id_with_a_very_long_value_that_would_exceed_mysql_named_lock_limit",
        );

        assert!(lock_key.len() <= 64);
        assert_eq!(
            lock_key,
            business_translation_lock_key(
                "form_definition",
                "resource_id_with_a_very_long_value_that_would_exceed_mysql_named_lock_limit"
            )
        );
    }
}
