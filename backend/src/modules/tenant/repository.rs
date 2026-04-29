//! 租户数据访问层。
//!
//! repository 只负责显式 SQL 和数据库方言差异，不承载业务规则；默认查询过滤 `deleted = FALSE`。

use chrono::{DateTime, Utc};
use sqlx::{MySql, Postgres, QueryBuilder, Row};

use crate::{
    db::executor::DatabasePool,
    error::app_error::AppResult,
    modules::tenant::{dto::TenantPageQuery, model::Tenant},
    response::page::NormalizedPageQuery,
};

/// 新增或更新租户时 repository 需要的完整字段。
#[derive(Debug, Clone)]
pub struct TenantWrite {
    pub id: String,
    pub tenant_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub isolation_mode: String,
    pub primary_domain: Option<String>,
    pub remark: Option<String>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

pub struct TenantRepository;

impl TenantRepository {
    pub async fn insert(pool: &DatabasePool, data: &TenantWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT INTO sys_tenants (id, tenant_code, name, name_full_pinyin, name_simple_pinyin, status, isolation_mode, primary_domain, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, 'enabled', ?, ?, ?, 1, FALSE, ?, ?, ?, ?)",
                )
                .bind(&data.id)
                .bind(&data.tenant_code)
                .bind(&data.name)
                .bind(&data.name_full_pinyin)
                .bind(&data.name_simple_pinyin)
                .bind(&data.isolation_mode)
                .bind(&data.primary_domain)
                .bind(&data.remark)
                .bind(&data.operator)
                .bind(data.now)
                .bind(&data.operator)
                .bind(data.now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO sys_tenants (id, tenant_code, name, name_full_pinyin, name_simple_pinyin, status, isolation_mode, primary_domain, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, 'enabled', $6, $7, $8, 1, FALSE, $9, $10, $11, $12)",
                )
                .bind(&data.id)
                .bind(&data.tenant_code)
                .bind(&data.name)
                .bind(&data.name_full_pinyin)
                .bind(&data.name_simple_pinyin)
                .bind(&data.isolation_mode)
                .bind(&data.primary_domain)
                .bind(&data.remark)
                .bind(&data.operator)
                .bind(data.now)
                .bind(&data.operator)
                .bind(data.now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        data: &TenantWrite,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE sys_tenants SET tenant_code = ?, name = ?, name_full_pinyin = ?, name_simple_pinyin = ?, isolation_mode = ?, primary_domain = ?, remark = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND version = ? AND deleted = FALSE",
            )
            .bind(&data.tenant_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.isolation_mode)
            .bind(&data.primary_domain)
            .bind(&data.remark)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE sys_tenants SET tenant_code = $1, name = $2, name_full_pinyin = $3, name_simple_pinyin = $4, isolation_mode = $5, primary_domain = $6, remark = $7, updated_by = $8, updated_time = $9, version = version + 1 WHERE id = $10 AND version = $11 AND deleted = FALSE",
            )
            .bind(&data.tenant_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.isolation_mode)
            .bind(&data.primary_domain)
            .bind(&data.remark)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(affected)
    }

    pub async fn update_status(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        status: &str,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE sys_tenants SET status = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND version = ? AND deleted = FALSE",
            )
            .bind(status)
            .bind(operator)
            .bind(now)
            .bind(id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE sys_tenants SET status = $1, updated_by = $2, updated_time = $3, version = version + 1 WHERE id = $4 AND version = $5 AND deleted = FALSE",
            )
            .bind(status)
            .bind(operator)
            .bind(now)
            .bind(id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(affected)
    }

    pub async fn get(pool: &DatabasePool, id: &str) -> AppResult<Option<Tenant>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, Tenant>(
                "SELECT * FROM sys_tenants WHERE id = ? AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, Tenant>(
                "SELECT * FROM sys_tenants WHERE id = $1 AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn logical_delete(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE sys_tenants SET deleted = TRUE, deleted_by = ?, deleted_time = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND version = ? AND deleted = FALSE",
            )
            .bind(operator)
            .bind(now)
            .bind(operator)
            .bind(now)
            .bind(id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE sys_tenants SET deleted = TRUE, deleted_by = $1, deleted_time = $2, updated_by = $3, updated_time = $4, version = version + 1 WHERE id = $5 AND version = $6 AND deleted = FALSE",
            )
            .bind(operator)
            .bind(now)
            .bind(operator)
            .bind(now)
            .bind(id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(affected)
    }

    pub async fn physical_delete(pool: &DatabasePool, id: &str) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("DELETE FROM sys_tenants WHERE id = ? AND deleted = TRUE")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM sys_tenants WHERE id = $1 AND deleted = TRUE")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };
        Ok(affected)
    }

    pub async fn page(
        pool: &DatabasePool,
        query: &TenantPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<Tenant>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => page_tenants_mysql(pool, query, page).await,
            DatabasePool::Postgres(pool) => page_tenants_postgres(pool, query, page).await,
        }
    }
}

async fn page_tenants_mysql(
    pool: &sqlx::MySqlPool,
    query: &TenantPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<Tenant>, u64)> {
    let mut count = QueryBuilder::<MySql>::new(
        "SELECT COUNT(*) AS total FROM sys_tenants WHERE deleted = FALSE",
    );
    append_filters_mysql(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<MySql>::new("SELECT * FROM sys_tenants WHERE deleted = FALSE");
    append_filters_mysql(&mut rows, query);
    rows.push(" ORDER BY updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<Tenant>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

async fn page_tenants_postgres(
    pool: &sqlx::PgPool,
    query: &TenantPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<Tenant>, u64)> {
    let mut count = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) AS total FROM sys_tenants WHERE deleted = FALSE",
    );
    append_filters_postgres(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<Postgres>::new("SELECT * FROM sys_tenants WHERE deleted = FALSE");
    append_filters_postgres(&mut rows, query);
    rows.push(" ORDER BY updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<Tenant>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

fn append_filters_mysql(builder: &mut QueryBuilder<MySql>, query: &TenantPageQuery) {
    if let Some(value) = normalized_like(query.tenant_code.as_deref()) {
        builder.push(" AND tenant_code LIKE ").push_bind(value);
    }
    append_name_filter_mysql(builder, query.name.as_deref());
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.isolation_mode.as_deref()) {
        builder.push(" AND isolation_mode = ").push_bind(value);
    }
    if let Some(value) = normalized_like(query.primary_domain.as_deref()) {
        builder.push(" AND primary_domain LIKE ").push_bind(value);
    }
    append_time_range_mysql(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
    );
    append_time_range_mysql(
        builder,
        "updated_time",
        query.updated_time_start,
        query.updated_time_end,
    );
}

fn append_filters_postgres(builder: &mut QueryBuilder<Postgres>, query: &TenantPageQuery) {
    if let Some(value) = normalized_like(query.tenant_code.as_deref()) {
        builder.push(" AND tenant_code LIKE ").push_bind(value);
    }
    append_name_filter_postgres(builder, query.name.as_deref());
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.isolation_mode.as_deref()) {
        builder.push(" AND isolation_mode = ").push_bind(value);
    }
    if let Some(value) = normalized_like(query.primary_domain.as_deref()) {
        builder.push(" AND primary_domain LIKE ").push_bind(value);
    }
    append_time_range_postgres(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
    );
    append_time_range_postgres(
        builder,
        "updated_time",
        query.updated_time_start,
        query.updated_time_end,
    );
}

fn append_name_filter_mysql(builder: &mut QueryBuilder<MySql>, value: Option<&str>) {
    if let Some(value) = normalized_like(value) {
        builder.push(" AND (name LIKE ");
        builder.push_bind(value.clone());
        builder.push(" OR name_full_pinyin LIKE ");
        builder.push_bind(value.clone());
        builder.push(" OR name_simple_pinyin LIKE ");
        builder.push_bind(value);
        builder.push(")");
    }
}

fn append_name_filter_postgres(builder: &mut QueryBuilder<Postgres>, value: Option<&str>) {
    if let Some(value) = normalized_like(value) {
        builder.push(" AND (name LIKE ");
        builder.push_bind(value.clone());
        builder.push(" OR name_full_pinyin LIKE ");
        builder.push_bind(value.clone());
        builder.push(" OR name_simple_pinyin LIKE ");
        builder.push_bind(value);
        builder.push(")");
    }
}

fn append_time_range_mysql(
    builder: &mut QueryBuilder<MySql>,
    column: &str,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) {
    if let Some(start) = start {
        builder.push(" AND ");
        builder.push(column);
        builder.push(" >= ");
        builder.push_bind(start);
    }
    if let Some(end) = end {
        builder.push(" AND ");
        builder.push(column);
        builder.push(" <= ");
        builder.push_bind(end);
    }
}

fn append_time_range_postgres(
    builder: &mut QueryBuilder<Postgres>,
    column: &str,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) {
    if let Some(start) = start {
        builder.push(" AND ");
        builder.push(column);
        builder.push(" >= ");
        builder.push_bind(start);
    }
    if let Some(end) = end {
        builder.push(" AND ");
        builder.push(column);
        builder.push(" <= ");
        builder.push_bind(end);
    }
}

fn normalize_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalized_like(value: Option<&str>) -> Option<String> {
    normalize_non_empty(value).map(|value| format!("%{value}%"))
}
