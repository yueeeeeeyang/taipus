//! 认证模块数据访问层。
//!
//! repository 只负责显式 SQL 和方言差异，业务规则、密码校验、令牌轮换和审计语义由 service 编排。

use chrono::{DateTime, Utc};
use sqlx::{MySql, Postgres, QueryBuilder};

use crate::{
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::auth::{
        dto::{AccountPageQuery, AccountTenantPageQuery, SessionPageQuery},
        model::{Account, AccountTenant, RefreshTokenSession},
    },
    response::page::{NormalizedPageQuery, PageResult},
};

#[derive(Debug, Clone)]
pub struct AccountWrite {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub display_name_full_pinyin: String,
    pub display_name_simple_pinyin: String,
    pub password_hash: String,
    pub password_algo: String,
    pub status: String,
    pub hrm_user_id: Option<String>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AccountTenantWrite {
    pub id: String,
    pub account_id: String,
    pub tenant_id: String,
    pub status: String,
    pub is_default: bool,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RefreshTokenWrite {
    pub id: String,
    pub account_id: String,
    pub tenant_id: String,
    pub token_hash: String,
    pub token_family: String,
    pub status: String,
    pub client_type: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub expires_time: DateTime<Utc>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

pub struct AuthRepository;

impl AuthRepository {
    pub async fn insert_account(pool: &DatabasePool, data: &AccountWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_accounts (id, username, display_name, display_name_full_pinyin, display_name_simple_pinyin, password_hash, password_algo, password_updated_time, status, hrm_user_id, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.username).bind(&data.display_name).bind(&data.display_name_full_pinyin).bind(&data.display_name_simple_pinyin).bind(&data.password_hash).bind(&data.password_algo).bind(data.now).bind(&data.status).bind(&data.hrm_user_id).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_accounts (id, username, display_name, display_name_full_pinyin, display_name_simple_pinyin, password_hash, password_algo, password_updated_time, status, hrm_user_id, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, FALSE, $11, $12, $13, $14)")
                    .bind(&data.id).bind(&data.username).bind(&data.display_name).bind(&data.display_name_full_pinyin).bind(&data.display_name_simple_pinyin).bind(&data.password_hash).bind(&data.password_algo).bind(data.now).bind(&data.status).bind(&data.hrm_user_id).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn update_account(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        data: &AccountWrite,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query("UPDATE sys_accounts SET username = ?, display_name = ?, display_name_full_pinyin = ?, display_name_simple_pinyin = ?, status = ?, hrm_user_id = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE id = ? AND version = ? AND deleted = FALSE")
                .bind(&data.username).bind(&data.display_name).bind(&data.display_name_full_pinyin).bind(&data.display_name_simple_pinyin).bind(&data.status).bind(&data.hrm_user_id).bind(&data.operator).bind(data.now).bind(id).bind(version).execute(pool).await?.rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("UPDATE sys_accounts SET username = $1, display_name = $2, display_name_full_pinyin = $3, display_name_simple_pinyin = $4, status = $5, hrm_user_id = $6, version = version + 1, updated_by = $7, updated_time = $8 WHERE id = $9 AND version = $10 AND deleted = FALSE")
                .bind(&data.username).bind(&data.display_name).bind(&data.display_name_full_pinyin).bind(&data.display_name_simple_pinyin).bind(&data.status).bind(&data.hrm_user_id).bind(&data.operator).bind(data.now).bind(id).bind(version).execute(pool).await?.rows_affected(),
        };
        Ok(affected)
    }

    pub async fn update_account_status(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        status: &str,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query("UPDATE sys_accounts SET status = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE id = ? AND version = ? AND deleted = FALSE")
                .bind(status).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("UPDATE sys_accounts SET status = $1, version = version + 1, updated_by = $2, updated_time = $3 WHERE id = $4 AND version = $5 AND deleted = FALSE")
                .bind(status).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
        };
        Ok(affected)
    }

    pub async fn reset_password(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        password_hash: &str,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query("UPDATE sys_accounts SET password_hash = ?, password_algo = 'argon2id', password_updated_time = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE id = ? AND version = ? AND deleted = FALSE")
                .bind(password_hash).bind(now).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("UPDATE sys_accounts SET password_hash = $1, password_algo = 'argon2id', password_updated_time = $2, version = version + 1, updated_by = $3, updated_time = $4 WHERE id = $5 AND version = $6 AND deleted = FALSE")
                .bind(password_hash).bind(now).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
        };
        Ok(affected)
    }

    pub async fn logical_delete_account(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query("UPDATE sys_accounts SET deleted = TRUE, deleted_by = ?, deleted_time = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE id = ? AND version = ? AND deleted = FALSE")
                .bind(operator).bind(now).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("UPDATE sys_accounts SET deleted = TRUE, deleted_by = $1, deleted_time = $2, version = version + 1, updated_by = $3, updated_time = $4 WHERE id = $5 AND version = $6 AND deleted = FALSE")
                .bind(operator).bind(now).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
        };
        Ok(affected)
    }

    pub async fn physical_delete_account(pool: &DatabasePool, id: &str) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("DELETE FROM sys_accounts WHERE id = ? AND deleted = TRUE")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM sys_accounts WHERE id = $1 AND deleted = TRUE")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };
        Ok(affected)
    }

    pub async fn find_account_by_username(
        pool: &DatabasePool,
        username: &str,
    ) -> AppResult<Option<Account>> {
        match pool {
            DatabasePool::MySql(pool) => sqlx::query_as::<_, Account>(
                "SELECT * FROM sys_accounts WHERE username = ? AND deleted = FALSE",
            )
            .bind(username)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, Account>(
                "SELECT * FROM sys_accounts WHERE username = $1 AND deleted = FALSE",
            )
            .bind(username)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
        }
    }

    pub async fn find_account_by_id(pool: &DatabasePool, id: &str) -> AppResult<Option<Account>> {
        match pool {
            DatabasePool::MySql(pool) => sqlx::query_as::<_, Account>(
                "SELECT * FROM sys_accounts WHERE id = ? AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, Account>(
                "SELECT * FROM sys_accounts WHERE id = $1 AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
        }
    }

    pub async fn insert_account_tenant(
        pool: &DatabasePool,
        data: &AccountTenantWrite,
    ) -> AppResult<()> {
        if data.is_default {
            Self::clear_default_tenant(pool, &data.account_id).await?;
        }
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_account_tenants (id, account_id, tenant_id, status, is_default, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.account_id).bind(&data.tenant_id).bind(&data.status).bind(data.is_default).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_account_tenants (id, account_id, tenant_id, status, is_default, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, 1, FALSE, $6, $7, $8, $9)")
                    .bind(&data.id).bind(&data.account_id).bind(&data.tenant_id).bind(&data.status).bind(data.is_default).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        };
        Ok(())
    }

    pub async fn update_account_tenant(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        status: &str,
        is_default: bool,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let existing = Self::find_account_tenant_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("账号租户关系不存在或已删除"))?;
        if existing.version != version {
            return Ok(0);
        }
        if is_default {
            Self::clear_default_tenant(pool, &existing.account_id).await?;
        }
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query("UPDATE sys_account_tenants SET status = ?, is_default = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE id = ? AND version = ? AND deleted = FALSE")
                .bind(status).bind(is_default).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("UPDATE sys_account_tenants SET status = $1, is_default = $2, version = version + 1, updated_by = $3, updated_time = $4 WHERE id = $5 AND version = $6 AND deleted = FALSE")
                .bind(status).bind(is_default).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
        };
        Ok(affected)
    }

    pub async fn clear_default_tenant(pool: &DatabasePool, account_id: &str) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("UPDATE sys_account_tenants SET is_default = FALSE WHERE account_id = ? AND deleted = FALSE")
                    .bind(account_id).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE sys_account_tenants SET is_default = FALSE WHERE account_id = $1 AND deleted = FALSE")
                    .bind(account_id).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn find_account_tenant(
        pool: &DatabasePool,
        account_id: &str,
        tenant_id: &str,
    ) -> AppResult<Option<AccountTenant>> {
        match pool {
            DatabasePool::MySql(pool) => sqlx::query_as::<_, AccountTenant>("SELECT * FROM sys_account_tenants WHERE account_id = ? AND tenant_id = ? AND deleted = FALSE").bind(account_id).bind(tenant_id).fetch_optional(pool).await.map_err(Into::into),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, AccountTenant>("SELECT * FROM sys_account_tenants WHERE account_id = $1 AND tenant_id = $2 AND deleted = FALSE").bind(account_id).bind(tenant_id).fetch_optional(pool).await.map_err(Into::into),
        }
    }

    pub async fn find_account_tenant_by_id(
        pool: &DatabasePool,
        id: &str,
    ) -> AppResult<Option<AccountTenant>> {
        match pool {
            DatabasePool::MySql(pool) => sqlx::query_as::<_, AccountTenant>(
                "SELECT * FROM sys_account_tenants WHERE id = ? AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, AccountTenant>(
                "SELECT * FROM sys_account_tenants WHERE id = $1 AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
        }
    }

    pub async fn list_account_tenants(
        pool: &DatabasePool,
        account_id: &str,
    ) -> AppResult<Vec<AccountTenant>> {
        match pool {
            DatabasePool::MySql(pool) => sqlx::query_as::<_, AccountTenant>("SELECT * FROM sys_account_tenants WHERE account_id = ? AND deleted = FALSE ORDER BY is_default DESC, updated_time DESC").bind(account_id).fetch_all(pool).await.map_err(Into::into),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, AccountTenant>("SELECT * FROM sys_account_tenants WHERE account_id = $1 AND deleted = FALSE ORDER BY is_default DESC, updated_time DESC").bind(account_id).fetch_all(pool).await.map_err(Into::into),
        }
    }

    pub async fn logical_delete_account_tenant(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query("UPDATE sys_account_tenants SET deleted = TRUE, deleted_by = ?, deleted_time = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE id = ? AND version = ? AND deleted = FALSE")
                .bind(operator).bind(now).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("UPDATE sys_account_tenants SET deleted = TRUE, deleted_by = $1, deleted_time = $2, version = version + 1, updated_by = $3, updated_time = $4 WHERE id = $5 AND version = $6 AND deleted = FALSE")
                .bind(operator).bind(now).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
        };
        Ok(affected)
    }

    pub async fn insert_refresh_token(
        pool: &DatabasePool,
        data: &RefreshTokenWrite,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_refresh_tokens (id, account_id, tenant_id, token_hash, token_family, status, client_type, device_id, device_name, ip, user_agent, expires_time, last_used_time, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.account_id).bind(&data.tenant_id).bind(&data.token_hash).bind(&data.token_family).bind(&data.status).bind(&data.client_type).bind(&data.device_id).bind(&data.device_name).bind(&data.ip).bind(&data.user_agent).bind(data.expires_time).bind(data.now).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_refresh_tokens (id, account_id, tenant_id, token_hash, token_family, status, client_type, device_id, device_name, ip, user_agent, expires_time, last_used_time, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 1, FALSE, $14, $15, $16, $17)")
                    .bind(&data.id).bind(&data.account_id).bind(&data.tenant_id).bind(&data.token_hash).bind(&data.token_family).bind(&data.status).bind(&data.client_type).bind(&data.device_id).bind(&data.device_name).bind(&data.ip).bind(&data.user_agent).bind(data.expires_time).bind(data.now).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        };
        Ok(())
    }

    pub async fn find_refresh_by_hash(
        pool: &DatabasePool,
        token_hash: &str,
    ) -> AppResult<Option<RefreshTokenSession>> {
        match pool {
            DatabasePool::MySql(pool) => sqlx::query_as::<_, RefreshTokenSession>(
                "SELECT * FROM sys_refresh_tokens WHERE token_hash = ? AND deleted = FALSE",
            )
            .bind(token_hash)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, RefreshTokenSession>(
                "SELECT * FROM sys_refresh_tokens WHERE token_hash = $1 AND deleted = FALSE",
            )
            .bind(token_hash)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
        }
    }

    pub async fn find_refresh_by_id(
        pool: &DatabasePool,
        id: &str,
    ) -> AppResult<Option<RefreshTokenSession>> {
        match pool {
            DatabasePool::MySql(pool) => sqlx::query_as::<_, RefreshTokenSession>(
                "SELECT * FROM sys_refresh_tokens WHERE id = ? AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, RefreshTokenSession>(
                "SELECT * FROM sys_refresh_tokens WHERE id = $1 AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(Into::into),
        }
    }

    pub async fn change_refresh_status(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        status: &str,
        operator: &str,
        reason: Option<&str>,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query("UPDATE sys_refresh_tokens SET status = ?, revoked_by = ?, revoked_time = ?, revoked_reason = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE id = ? AND version = ? AND deleted = FALSE")
                .bind(status).bind(operator).bind(now).bind(reason).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query("UPDATE sys_refresh_tokens SET status = $1, revoked_by = $2, revoked_time = $3, revoked_reason = $4, version = version + 1, updated_by = $5, updated_time = $6 WHERE id = $7 AND version = $8 AND deleted = FALSE")
                .bind(status).bind(operator).bind(now).bind(reason).bind(operator).bind(now).bind(id).bind(version).execute(pool).await?.rows_affected(),
        };
        Ok(affected)
    }

    pub async fn revoke_account_sessions(
        pool: &DatabasePool,
        account_id: &str,
        operator: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("UPDATE sys_refresh_tokens SET status = 'revoked', revoked_by = ?, revoked_time = ?, revoked_reason = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE account_id = ? AND status = 'active' AND deleted = FALSE")
                .bind(operator).bind(now).bind(reason).bind(operator).bind(now).bind(account_id).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE sys_refresh_tokens SET status = 'revoked', revoked_by = $1, revoked_time = $2, revoked_reason = $3, version = version + 1, updated_by = $4, updated_time = $5 WHERE account_id = $6 AND status = 'active' AND deleted = FALSE")
                .bind(operator).bind(now).bind(reason).bind(operator).bind(now).bind(account_id).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn revoke_token_family_sessions(
        pool: &DatabasePool,
        token_family: &str,
        operator: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("UPDATE sys_refresh_tokens SET status = 'revoked', revoked_by = ?, revoked_time = ?, revoked_reason = ?, version = version + 1, updated_by = ?, updated_time = ? WHERE token_family = ? AND status = 'active' AND deleted = FALSE")
                    .bind(operator).bind(now).bind(reason).bind(operator).bind(now).bind(token_family).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE sys_refresh_tokens SET status = 'revoked', revoked_by = $1, revoked_time = $2, revoked_reason = $3, version = version + 1, updated_by = $4, updated_time = $5 WHERE token_family = $6 AND status = 'active' AND deleted = FALSE")
                    .bind(operator).bind(now).bind(reason).bind(operator).bind(now).bind(token_family).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn update_last_login(
        pool: &DatabasePool,
        account_id: &str,
        ip: Option<&str>,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("UPDATE sys_accounts SET last_login_time = ?, last_login_ip = ?, updated_time = ? WHERE id = ?").bind(now).bind(ip).bind(now).bind(account_id).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE sys_accounts SET last_login_time = $1, last_login_ip = $2, updated_time = $3 WHERE id = $4").bind(now).bind(ip).bind(now).bind(account_id).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn insert_audit(
        pool: &DatabasePool,
        tenant_id: Option<&str>,
        account_id: Option<&str>,
        event_type: &str,
        result: &str,
        client_type: Option<&str>,
        ip: Option<&str>,
        user_agent: Option<&str>,
        trace_id: &str,
        message: Option<&str>,
    ) -> AppResult<()> {
        let id = crate::utils::id::generate_business_id();
        let now = Utc::now();
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_auth_audit_logs (id, tenant_id, account_id, event_type, result, client_type, ip, user_agent, trace_id, message, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&id).bind(tenant_id).bind(account_id).bind(event_type).bind(result).bind(client_type).bind(ip).bind(user_agent).bind(trace_id).bind(message).bind(account_id.unwrap_or("system")).bind(now).bind(account_id.unwrap_or("system")).bind(now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_auth_audit_logs (id, tenant_id, account_id, event_type, result, client_type, ip, user_agent, trace_id, message, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, FALSE, $11, $12, $13, $14)")
                    .bind(&id).bind(tenant_id).bind(account_id).bind(event_type).bind(result).bind(client_type).bind(ip).bind(user_agent).bind(trace_id).bind(message).bind(account_id.unwrap_or("system")).bind(now).bind(account_id.unwrap_or("system")).bind(now).execute(pool).await?;
            }
        };
        Ok(())
    }

    pub async fn page_accounts(
        pool: &DatabasePool,
        query: &AccountPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<PageResult<Account>> {
        match pool {
            DatabasePool::MySql(pool) => page_accounts_mysql(pool, query, page).await,
            DatabasePool::Postgres(pool) => page_accounts_postgres(pool, query, page).await,
        }
    }

    pub async fn page_account_tenants(
        pool: &DatabasePool,
        query: &AccountTenantPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<PageResult<AccountTenant>> {
        match pool {
            DatabasePool::MySql(pool) => page_account_tenants_mysql(pool, query, page).await,
            DatabasePool::Postgres(pool) => page_account_tenants_postgres(pool, query, page).await,
        }
    }

    pub async fn page_sessions(
        pool: &DatabasePool,
        query: &SessionPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<PageResult<RefreshTokenSession>> {
        match pool {
            DatabasePool::MySql(pool) => page_sessions_mysql(pool, query, page).await,
            DatabasePool::Postgres(pool) => page_sessions_postgres(pool, query, page).await,
        }
    }
}

async fn page_accounts_mysql(
    pool: &sqlx::Pool<MySql>,
    query: &AccountPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<PageResult<Account>> {
    let mut count =
        QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM sys_accounts WHERE deleted = FALSE");
    push_account_filters(&mut count, query);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;
    let mut list = QueryBuilder::<MySql>::new("SELECT * FROM sys_accounts WHERE deleted = FALSE");
    push_account_filters(&mut list, query);
    list.push(" ORDER BY updated_time DESC, id ASC LIMIT ")
        .push_bind(page.page_size as i64)
        .push(" OFFSET ")
        .push_bind(page.offset as i64);
    let records = list.build_query_as::<Account>().fetch_all(pool).await?;
    Ok(PageResult::new(records, page, total as u64))
}

async fn page_accounts_postgres(
    pool: &sqlx::Pool<Postgres>,
    query: &AccountPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<PageResult<Account>> {
    let mut count =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM sys_accounts WHERE deleted = FALSE");
    push_account_filters(&mut count, query);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;
    let mut list =
        QueryBuilder::<Postgres>::new("SELECT * FROM sys_accounts WHERE deleted = FALSE");
    push_account_filters(&mut list, query);
    list.push(" ORDER BY updated_time DESC, id ASC LIMIT ")
        .push_bind(page.page_size as i64)
        .push(" OFFSET ")
        .push_bind(page.offset as i64);
    let records = list.build_query_as::<Account>().fetch_all(pool).await?;
    Ok(PageResult::new(records, page, total as u64))
}

fn push_account_filters<'a, DB>(builder: &mut QueryBuilder<'a, DB>, query: &'a AccountPageQuery)
where
    DB: sqlx::Database,
    String: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
{
    if let Some(username) = query
        .username
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND username LIKE ")
            .push_bind(format!("%{}%", username.trim()));
    }
    if let Some(display_name) = query
        .display_name
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let value = format!("%{}%", display_name.trim());
        builder
            .push(" AND (display_name LIKE ")
            .push_bind(value.clone())
            .push(" OR display_name_full_pinyin LIKE ")
            .push_bind(value.clone())
            .push(" OR display_name_simple_pinyin LIKE ")
            .push_bind(value)
            .push(")");
    }
    if let Some(status) = query
        .status
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND status = ")
            .push_bind(status.trim().to_string());
    }
    if let Some(hrm_user_id) = query
        .hrm_user_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND hrm_user_id = ")
            .push_bind(hrm_user_id.trim().to_string());
    }
}

async fn page_account_tenants_mysql(
    pool: &sqlx::Pool<MySql>,
    query: &AccountTenantPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<PageResult<AccountTenant>> {
    let mut count = QueryBuilder::<MySql>::new(
        "SELECT COUNT(*) FROM sys_account_tenants WHERE deleted = FALSE",
    );
    push_account_tenant_filters(&mut count, query);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;
    let mut list =
        QueryBuilder::<MySql>::new("SELECT * FROM sys_account_tenants WHERE deleted = FALSE");
    push_account_tenant_filters(&mut list, query);
    list.push(" ORDER BY updated_time DESC, id ASC LIMIT ")
        .push_bind(page.page_size as i64)
        .push(" OFFSET ")
        .push_bind(page.offset as i64);
    Ok(PageResult::new(
        list.build_query_as::<AccountTenant>()
            .fetch_all(pool)
            .await?,
        page,
        total as u64,
    ))
}

async fn page_account_tenants_postgres(
    pool: &sqlx::Pool<Postgres>,
    query: &AccountTenantPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<PageResult<AccountTenant>> {
    let mut count = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM sys_account_tenants WHERE deleted = FALSE",
    );
    push_account_tenant_filters(&mut count, query);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;
    let mut list =
        QueryBuilder::<Postgres>::new("SELECT * FROM sys_account_tenants WHERE deleted = FALSE");
    push_account_tenant_filters(&mut list, query);
    list.push(" ORDER BY updated_time DESC, id ASC LIMIT ")
        .push_bind(page.page_size as i64)
        .push(" OFFSET ")
        .push_bind(page.offset as i64);
    Ok(PageResult::new(
        list.build_query_as::<AccountTenant>()
            .fetch_all(pool)
            .await?,
        page,
        total as u64,
    ))
}

fn push_account_tenant_filters<'a, DB>(
    builder: &mut QueryBuilder<'a, DB>,
    query: &'a AccountTenantPageQuery,
) where
    DB: sqlx::Database,
    String: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    bool: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
{
    if let Some(value) = query
        .account_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND account_id = ")
            .push_bind(value.trim().to_string());
    }
    if let Some(value) = query
        .tenant_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND tenant_id = ")
            .push_bind(value.trim().to_string());
    }
    if let Some(value) = query
        .status
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND status = ")
            .push_bind(value.trim().to_string());
    }
    if let Some(value) = query.is_default {
        builder.push(" AND is_default = ").push_bind(value);
    }
}

async fn page_sessions_mysql(
    pool: &sqlx::Pool<MySql>,
    query: &SessionPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<PageResult<RefreshTokenSession>> {
    let mut count =
        QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM sys_refresh_tokens WHERE deleted = FALSE");
    push_session_filters(&mut count, query);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;
    let mut list =
        QueryBuilder::<MySql>::new("SELECT * FROM sys_refresh_tokens WHERE deleted = FALSE");
    push_session_filters(&mut list, query);
    list.push(" ORDER BY updated_time DESC, id ASC LIMIT ")
        .push_bind(page.page_size as i64)
        .push(" OFFSET ")
        .push_bind(page.offset as i64);
    Ok(PageResult::new(
        list.build_query_as::<RefreshTokenSession>()
            .fetch_all(pool)
            .await?,
        page,
        total as u64,
    ))
}

async fn page_sessions_postgres(
    pool: &sqlx::Pool<Postgres>,
    query: &SessionPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<PageResult<RefreshTokenSession>> {
    let mut count = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) FROM sys_refresh_tokens WHERE deleted = FALSE",
    );
    push_session_filters(&mut count, query);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;
    let mut list =
        QueryBuilder::<Postgres>::new("SELECT * FROM sys_refresh_tokens WHERE deleted = FALSE");
    push_session_filters(&mut list, query);
    list.push(" ORDER BY updated_time DESC, id ASC LIMIT ")
        .push_bind(page.page_size as i64)
        .push(" OFFSET ")
        .push_bind(page.offset as i64);
    Ok(PageResult::new(
        list.build_query_as::<RefreshTokenSession>()
            .fetch_all(pool)
            .await?,
        page,
        total as u64,
    ))
}

fn push_session_filters<'a, DB>(builder: &mut QueryBuilder<'a, DB>, query: &'a SessionPageQuery)
where
    DB: sqlx::Database,
    String: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
    DateTime<Utc>: sqlx::Encode<'a, DB> + sqlx::Type<DB>,
{
    if let Some(value) = query
        .account_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND account_id = ")
            .push_bind(value.trim().to_string());
    }
    if let Some(value) = query
        .tenant_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND tenant_id = ")
            .push_bind(value.trim().to_string());
    }
    if let Some(value) = query
        .status
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND status = ")
            .push_bind(value.trim().to_string());
    }
    if let Some(value) = query
        .client_type
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder
            .push(" AND client_type = ")
            .push_bind(value.trim().to_string());
    }
    if let Some(value) = query.expires_time_start {
        builder.push(" AND expires_time >= ").push_bind(value);
    }
    if let Some(value) = query.expires_time_end {
        builder.push(" AND expires_time <= ").push_bind(value);
    }
}
