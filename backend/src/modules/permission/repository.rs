//! 权限模块数据访问层。
//!
//! repository 只负责显式 SQL 和数据库方言差异，角色继承、授权合并和权限动作校验放在 service 层。

use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::{
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::permission::model::{
        AccountRole, PermissionApi, PermissionApplication, PermissionButton, PermissionGrant,
        PermissionMenu, PermissionRole,
    },
    response::page::NormalizedPageQuery,
};

#[derive(Debug, Clone)]
pub struct ApplicationWrite {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub platform: String,
    pub home_path: Option<String>,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MenuWrite {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_id: String,
    pub parent_id: Option<String>,
    pub menu_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub platform: String,
    pub route_path: String,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub visible: bool,
    pub keep_alive: bool,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ButtonWrite {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_id: String,
    pub menu_id: String,
    pub button_code: String,
    pub name: String,
    pub action_key: String,
    pub button_type: String,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ApiWrite {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_id: Option<String>,
    pub api_code: String,
    pub name: String,
    pub http_method: String,
    pub path_pattern: String,
    pub normalized_path: String,
    pub related_menu_id: Option<String>,
    pub related_button_id: Option<String>,
    pub public_access: bool,
    pub auth_required: bool,
    pub status: String,
    pub remark: Option<String>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RoleWrite {
    pub id: String,
    pub tenant_id: String,
    pub role_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub role_type: String,
    pub status: String,
    pub sort_no: i64,
    pub remark: Option<String>,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct GrantWrite {
    pub id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
    pub operator: String,
    pub now: DateTime<Utc>,
}

pub struct PermissionRepository;

impl PermissionRepository {
    pub async fn ensure_bootstrap_admin_permissions(
        pool: &DatabasePool,
        tenant_id: &str,
        account_id: &str,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                let role_id = sqlx::query_scalar::<_, String>("SELECT id FROM sys_roles WHERE tenant_id = ? AND role_code = 'system_admin' AND deleted = FALSE")
                    .bind(tenant_id).fetch_optional(pool).await?.unwrap_or_else(crate::utils::id::generate_business_id);
                sqlx::query("INSERT INTO sys_roles (id, tenant_id, role_code, name, name_full_pinyin, name_simple_pinyin, role_type, status, sort_no, remark, version, deleted, created_by, created_time, updated_by, updated_time) SELECT ?, ?, 'system_admin', '系统管理员', 'xitongguanliyuan', 'xtgly', 'system', 'enabled', 0, '启动期内置管理员角色', 1, FALSE, ?, ?, ?, ? WHERE NOT EXISTS (SELECT 1 FROM sys_roles WHERE tenant_id = ? AND role_code = 'system_admin' AND deleted = FALSE)")
                    .bind(&role_id).bind(tenant_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).execute(pool).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) SELECT ?, ?, ?, ?, 0, 1, FALSE, ?, ?, ?, ? WHERE NOT EXISTS (SELECT 1 FROM sys_role_closures WHERE tenant_id = ? AND ancestor_role_id = ? AND descendant_role_id = ? AND deleted = FALSE)")
                    .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(&role_id).bind(&role_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(&role_id).bind(&role_id).execute(pool).await?;
                sqlx::query("INSERT INTO sys_account_roles (id, tenant_id, account_id, role_id, status, version, deleted, created_by, created_time, updated_by, updated_time) SELECT ?, ?, ?, ?, 'enabled', 1, FALSE, ?, ?, ?, ? WHERE NOT EXISTS (SELECT 1 FROM sys_account_roles WHERE tenant_id = ? AND account_id = ? AND role_id = ? AND deleted = FALSE)")
                    .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(account_id).bind(&role_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(account_id).bind(&role_id).execute(pool).await?;
                sqlx::query("INSERT INTO sys_permission_grants (id, tenant_id, subject_type, subject_id, resource_type, resource_id, action, effect, grant_source, version, deleted, created_by, created_time, updated_by, updated_time) SELECT REPLACE(UUID(), '-', ''), ?, 'role', ?, 'api', api.id, 'call', 'allow', 'system', 1, FALSE, ?, ?, ?, ? FROM sys_permission_apis api WHERE api.deleted = FALSE AND NOT EXISTS (SELECT 1 FROM sys_permission_grants g WHERE g.tenant_id = ? AND g.subject_type = 'role' AND g.subject_id = ? AND g.resource_type = 'api' AND g.resource_id = api.id AND g.action = 'call' AND g.deleted = FALSE)")
                    .bind(tenant_id).bind(&role_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(&role_id).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                let role_id = sqlx::query_scalar::<_, String>("SELECT id FROM sys_roles WHERE tenant_id = $1 AND role_code = 'system_admin' AND deleted = FALSE")
                    .bind(tenant_id).fetch_optional(pool).await?.unwrap_or_else(crate::utils::id::generate_business_id);
                sqlx::query("INSERT INTO sys_roles (id, tenant_id, role_code, name, name_full_pinyin, name_simple_pinyin, role_type, status, sort_no, remark, version, deleted, created_by, created_time, updated_by, updated_time) SELECT $1, $2, 'system_admin', '系统管理员', 'xitongguanliyuan', 'xtgly', 'system', 'enabled', 0, '启动期内置管理员角色', 1, FALSE, $3, $4, $5, $6 WHERE NOT EXISTS (SELECT 1 FROM sys_roles WHERE tenant_id = $7 AND role_code = 'system_admin' AND deleted = FALSE)")
                    .bind(&role_id).bind(tenant_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).execute(pool).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) SELECT $1, $2, $3, $4, 0, 1, FALSE, $5, $6, $7, $8 WHERE NOT EXISTS (SELECT 1 FROM sys_role_closures WHERE tenant_id = $9 AND ancestor_role_id = $10 AND descendant_role_id = $11 AND deleted = FALSE)")
                    .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(&role_id).bind(&role_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(&role_id).bind(&role_id).execute(pool).await?;
                sqlx::query("INSERT INTO sys_account_roles (id, tenant_id, account_id, role_id, status, version, deleted, created_by, created_time, updated_by, updated_time) SELECT $1, $2, $3, $4, 'enabled', 1, FALSE, $5, $6, $7, $8 WHERE NOT EXISTS (SELECT 1 FROM sys_account_roles WHERE tenant_id = $9 AND account_id = $10 AND role_id = $11 AND deleted = FALSE)")
                    .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(account_id).bind(&role_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(account_id).bind(&role_id).execute(pool).await?;
                sqlx::query("INSERT INTO sys_permission_grants (id, tenant_id, subject_type, subject_id, resource_type, resource_id, action, effect, grant_source, version, deleted, created_by, created_time, updated_by, updated_time) SELECT md5(api.id || random()::text || clock_timestamp()::text), $1, 'role', $2, 'api', api.id, 'call', 'allow', 'system', 1, FALSE, $3, $4, $5, $6 FROM sys_permission_apis api WHERE api.deleted = FALSE AND NOT EXISTS (SELECT 1 FROM sys_permission_grants g WHERE g.tenant_id = $7 AND g.subject_type = 'role' AND g.subject_id = $8 AND g.resource_type = 'api' AND g.resource_id = api.id AND g.action = 'call' AND g.deleted = FALSE)")
                    .bind(tenant_id).bind(&role_id).bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(&role_id).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn insert_application(pool: &DatabasePool, data: &ApplicationWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_permission_applications (id, tenant_id, app_code, name, name_full_pinyin, name_simple_pinyin, platform, home_path, icon, sort_no, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_code).bind(&data.name).bind(&data.name_full_pinyin).bind(&data.name_simple_pinyin).bind(&data.platform).bind(&data.home_path).bind(&data.icon).bind(data.sort_no).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_permission_applications (id, tenant_id, app_code, name, name_full_pinyin, name_simple_pinyin, platform, home_path, icon, sort_no, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 1, FALSE, $13, $14, $15, $16)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_code).bind(&data.name).bind(&data.name_full_pinyin).bind(&data.name_simple_pinyin).bind(&data.platform).bind(&data.home_path).bind(&data.icon).bind(data.sort_no).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        };
        Ok(())
    }

    pub async fn get_application(
        pool: &DatabasePool,
        id: &str,
    ) -> AppResult<Option<PermissionApplication>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionApplication>(
                "SELECT * FROM sys_permission_applications WHERE id = ? AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionApplication>(
                "SELECT * FROM sys_permission_applications WHERE id = $1 AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn page_applications(
        pool: &DatabasePool,
        tenant_id: &str,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<PermissionApplication>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => {
                let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sys_permission_applications WHERE (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE")
                    .bind(tenant_id).fetch_one(pool).await?;
                let records = sqlx::query_as::<_, PermissionApplication>("SELECT * FROM sys_permission_applications WHERE (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE ORDER BY sort_no ASC, updated_time DESC LIMIT ? OFFSET ?")
                    .bind(tenant_id).bind(page.page_size as i64).bind(page.offset as i64).fetch_all(pool).await?;
                Ok((records, total as u64))
            }
            DatabasePool::Postgres(pool) => {
                let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sys_permission_applications WHERE (tenant_id = $1 OR tenant_id IS NULL) AND deleted = FALSE")
                    .bind(tenant_id).fetch_one(pool).await?;
                let records = sqlx::query_as::<_, PermissionApplication>("SELECT * FROM sys_permission_applications WHERE (tenant_id = $1 OR tenant_id IS NULL) AND deleted = FALSE ORDER BY sort_no ASC, updated_time DESC LIMIT $2 OFFSET $3")
                    .bind(tenant_id).bind(page.page_size as i64).bind(page.offset as i64).fetch_all(pool).await?;
                Ok((records, total as u64))
            }
        }
    }

    pub async fn insert_menu(pool: &DatabasePool, data: &MenuWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_permission_menus (id, tenant_id, app_id, parent_id, menu_code, name, name_full_pinyin, name_simple_pinyin, platform, route_path, component, icon, visible, keep_alive, sort_no, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_id).bind(&data.parent_id).bind(&data.menu_code).bind(&data.name).bind(&data.name_full_pinyin).bind(&data.name_simple_pinyin).bind(&data.platform).bind(&data.route_path).bind(&data.component).bind(&data.icon).bind(data.visible).bind(data.keep_alive).bind(data.sort_no).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_permission_menus (id, tenant_id, app_id, parent_id, menu_code, name, name_full_pinyin, name_simple_pinyin, platform, route_path, component, icon, visible, keep_alive, sort_no, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 1, FALSE, $18, $19, $20, $21)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_id).bind(&data.parent_id).bind(&data.menu_code).bind(&data.name).bind(&data.name_full_pinyin).bind(&data.name_simple_pinyin).bind(&data.platform).bind(&data.route_path).bind(&data.component).bind(&data.icon).bind(data.visible).bind(data.keep_alive).bind(data.sort_no).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        };
        Ok(())
    }

    pub async fn page_menus(
        pool: &DatabasePool,
        tenant_id: &str,
    ) -> AppResult<Vec<PermissionMenu>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionMenu>("SELECT * FROM sys_permission_menus WHERE (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE ORDER BY sort_no ASC")
                .bind(tenant_id).fetch_all(pool).await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionMenu>("SELECT * FROM sys_permission_menus WHERE (tenant_id = $1 OR tenant_id IS NULL) AND deleted = FALSE ORDER BY sort_no ASC")
                .bind(tenant_id).fetch_all(pool).await?),
        }
    }

    pub async fn insert_button(pool: &DatabasePool, data: &ButtonWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_permission_buttons (id, tenant_id, app_id, menu_id, button_code, name, action_key, button_type, icon, sort_no, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_id).bind(&data.menu_id).bind(&data.button_code).bind(&data.name).bind(&data.action_key).bind(&data.button_type).bind(&data.icon).bind(data.sort_no).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_permission_buttons (id, tenant_id, app_id, menu_id, button_code, name, action_key, button_type, icon, sort_no, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 1, FALSE, $13, $14, $15, $16)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_id).bind(&data.menu_id).bind(&data.button_code).bind(&data.name).bind(&data.action_key).bind(&data.button_type).bind(&data.icon).bind(data.sort_no).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        };
        Ok(())
    }

    pub async fn page_buttons(
        pool: &DatabasePool,
        tenant_id: &str,
        menu_id: Option<&str>,
    ) -> AppResult<Vec<PermissionButton>> {
        match (pool, menu_id) {
            (DatabasePool::MySql(pool), Some(menu_id)) => Ok(sqlx::query_as::<_, PermissionButton>("SELECT * FROM sys_permission_buttons WHERE (tenant_id = ? OR tenant_id IS NULL) AND menu_id = ? AND deleted = FALSE ORDER BY sort_no ASC")
                .bind(tenant_id).bind(menu_id).fetch_all(pool).await?),
            (DatabasePool::MySql(pool), None) => Ok(sqlx::query_as::<_, PermissionButton>("SELECT * FROM sys_permission_buttons WHERE (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE ORDER BY sort_no ASC")
                .bind(tenant_id).fetch_all(pool).await?),
            (DatabasePool::Postgres(pool), Some(menu_id)) => Ok(sqlx::query_as::<_, PermissionButton>("SELECT * FROM sys_permission_buttons WHERE (tenant_id = $1 OR tenant_id IS NULL) AND menu_id = $2 AND deleted = FALSE ORDER BY sort_no ASC")
                .bind(tenant_id).bind(menu_id).fetch_all(pool).await?),
            (DatabasePool::Postgres(pool), None) => Ok(sqlx::query_as::<_, PermissionButton>("SELECT * FROM sys_permission_buttons WHERE (tenant_id = $1 OR tenant_id IS NULL) AND deleted = FALSE ORDER BY sort_no ASC")
                .bind(tenant_id).fetch_all(pool).await?),
        }
    }

    pub async fn insert_api(pool: &DatabasePool, data: &ApiWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_permission_apis (id, tenant_id, app_id, api_code, name, http_method, path_pattern, normalized_path, related_menu_id, related_button_id, public_access, auth_required, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_id).bind(&data.api_code).bind(&data.name).bind(&data.http_method).bind(&data.path_pattern).bind(&data.normalized_path).bind(&data.related_menu_id).bind(&data.related_button_id).bind(data.public_access).bind(data.auth_required).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_permission_apis (id, tenant_id, app_id, api_code, name, http_method, path_pattern, normalized_path, related_menu_id, related_button_id, public_access, auth_required, status, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 1, FALSE, $15, $16, $17, $18)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.app_id).bind(&data.api_code).bind(&data.name).bind(&data.http_method).bind(&data.path_pattern).bind(&data.normalized_path).bind(&data.related_menu_id).bind(&data.related_button_id).bind(data.public_access).bind(data.auth_required).bind(&data.status).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        };
        Ok(())
    }

    pub async fn find_api_by_route(
        pool: &DatabasePool,
        method: &str,
        normalized_path: &str,
    ) -> AppResult<Option<PermissionApi>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionApi>("SELECT * FROM sys_permission_apis WHERE http_method = ? AND normalized_path = ? AND deleted = FALSE")
                .bind(method).bind(normalized_path).fetch_optional(pool).await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionApi>("SELECT * FROM sys_permission_apis WHERE http_method = $1 AND normalized_path = $2 AND deleted = FALSE")
                .bind(method).bind(normalized_path).fetch_optional(pool).await?),
        }
    }

    pub async fn list_apis_by_method(
        pool: &DatabasePool,
        method: &str,
    ) -> AppResult<Vec<PermissionApi>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionApi>(
                "SELECT * FROM sys_permission_apis WHERE http_method = ? AND deleted = FALSE",
            )
            .bind(method)
            .fetch_all(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionApi>(
                "SELECT * FROM sys_permission_apis WHERE http_method = $1 AND deleted = FALSE",
            )
            .bind(method)
            .fetch_all(pool)
            .await?),
        }
    }

    pub async fn page_apis(
        pool: &DatabasePool,
        tenant_id: &str,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<PermissionApi>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => {
                let total: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM sys_permission_apis WHERE (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE",
                )
                .bind(tenant_id)
                .fetch_one(pool)
                .await?;
                let records = sqlx::query_as::<_, PermissionApi>(
                    "SELECT * FROM sys_permission_apis WHERE (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE ORDER BY http_method ASC, normalized_path ASC LIMIT ? OFFSET ?",
                )
                .bind(tenant_id)
                .bind(page.page_size as i64)
                .bind(page.offset as i64)
                .fetch_all(pool)
                .await?;
                Ok((records, total as u64))
            }
            DatabasePool::Postgres(pool) => {
                let total: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM sys_permission_apis WHERE (tenant_id = $1 OR tenant_id IS NULL) AND deleted = FALSE",
                )
                .bind(tenant_id)
                .fetch_one(pool)
                .await?;
                let records = sqlx::query_as::<_, PermissionApi>(
                    "SELECT * FROM sys_permission_apis WHERE (tenant_id = $1 OR tenant_id IS NULL) AND deleted = FALSE ORDER BY http_method ASC, normalized_path ASC LIMIT $2 OFFSET $3",
                )
                .bind(tenant_id)
                .bind(page.page_size as i64)
                .bind(page.offset as i64)
                .fetch_all(pool)
                .await?;
                Ok((records, total as u64))
            }
        }
    }

    pub async fn insert_role(pool: &DatabasePool, data: &RoleWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query("INSERT INTO sys_roles (id, tenant_id, role_code, name, name_full_pinyin, name_simple_pinyin, role_type, status, sort_no, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.role_code).bind(&data.name).bind(&data.name_full_pinyin).bind(&data.name_simple_pinyin).bind(&data.role_type).bind(&data.status).bind(data.sort_no).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, 0, 1, FALSE, ?, ?, ?, ?)")
                    .bind(crate::utils::id::generate_business_id()).bind(&data.tenant_id).bind(&data.id).bind(&data.id).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO sys_roles (id, tenant_id, role_code, name, name_full_pinyin, name_simple_pinyin, role_type, status, sort_no, remark, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, FALSE, $11, $12, $13, $14)")
                    .bind(&data.id).bind(&data.tenant_id).bind(&data.role_code).bind(&data.name).bind(&data.name_full_pinyin).bind(&data.name_simple_pinyin).bind(&data.role_type).bind(&data.status).bind(data.sort_no).bind(&data.remark).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, 0, 1, FALSE, $5, $6, $7, $8)")
                    .bind(crate::utils::id::generate_business_id()).bind(&data.tenant_id).bind(&data.id).bind(&data.id).bind(&data.operator).bind(data.now).bind(&data.operator).bind(data.now).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn get_role(pool: &DatabasePool, id: &str) -> AppResult<Option<PermissionRole>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionRole>(
                "SELECT * FROM sys_roles WHERE id = ? AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionRole>(
                "SELECT * FROM sys_roles WHERE id = $1 AND deleted = FALSE",
            )
            .bind(id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn page_roles(
        pool: &DatabasePool,
        tenant_id: &str,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<PermissionRole>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => {
                let total: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM sys_roles WHERE tenant_id = ? AND deleted = FALSE",
                )
                .bind(tenant_id)
                .fetch_one(pool)
                .await?;
                let records = sqlx::query_as::<_, PermissionRole>("SELECT * FROM sys_roles WHERE tenant_id = ? AND deleted = FALSE ORDER BY sort_no ASC, updated_time DESC LIMIT ? OFFSET ?")
                    .bind(tenant_id).bind(page.page_size as i64).bind(page.offset as i64).fetch_all(pool).await?;
                Ok((records, total as u64))
            }
            DatabasePool::Postgres(pool) => {
                let total: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM sys_roles WHERE tenant_id = $1 AND deleted = FALSE",
                )
                .bind(tenant_id)
                .fetch_one(pool)
                .await?;
                let records = sqlx::query_as::<_, PermissionRole>("SELECT * FROM sys_roles WHERE tenant_id = $1 AND deleted = FALSE ORDER BY sort_no ASC, updated_time DESC LIMIT $2 OFFSET $3")
                    .bind(tenant_id).bind(page.page_size as i64).bind(page.offset as i64).fetch_all(pool).await?;
                Ok((records, total as u64))
            }
        }
    }

    pub async fn list_roles(
        pool: &DatabasePool,
        tenant_id: &str,
    ) -> AppResult<Vec<PermissionRole>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionRole>(
                "SELECT * FROM sys_roles WHERE tenant_id = ? AND deleted = FALSE ORDER BY sort_no ASC, updated_time DESC",
            )
            .bind(tenant_id)
            .fetch_all(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionRole>(
                "SELECT * FROM sys_roles WHERE tenant_id = $1 AND deleted = FALSE ORDER BY sort_no ASC, updated_time DESC",
            )
            .bind(tenant_id)
            .fetch_all(pool)
            .await?),
        }
    }

    pub async fn list_role_parent_pairs(
        pool: &DatabasePool,
        tenant_id: &str,
    ) -> AppResult<Vec<(String, String)>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query("SELECT child_role_id, parent_role_id FROM sys_role_relations WHERE tenant_id = ? AND deleted = FALSE")
                .bind(tenant_id).fetch_all(pool).await?.into_iter().map(|row| (row.get("child_role_id"), row.get("parent_role_id"))).collect()),
            DatabasePool::Postgres(pool) => Ok(sqlx::query("SELECT child_role_id, parent_role_id FROM sys_role_relations WHERE tenant_id = $1 AND deleted = FALSE")
                .bind(tenant_id).fetch_all(pool).await?.into_iter().map(|row| (row.get("child_role_id"), row.get("parent_role_id"))).collect()),
        }
    }

    pub async fn replace_role_grants(
        pool: &DatabasePool,
        tenant_id: &str,
        role_id: &str,
        expected_version: i64,
        grants: &[GrantWrite],
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                let mut tx = pool.begin().await?;
                let affected = sqlx::query("UPDATE sys_roles SET version = version + 1, updated_by = ?, updated_time = ? WHERE tenant_id = ? AND id = ? AND version = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(tenant_id).bind(role_id).bind(expected_version).execute(&mut *tx).await?.rows_affected();
                if affected == 0 {
                    return Err(AppError::conflict("角色权限已被修改，请刷新后重试"));
                }
                sqlx::query("UPDATE sys_permission_grants SET deleted = TRUE, deleted_by = ?, deleted_time = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND subject_type = 'role' AND subject_id = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(role_id).execute(&mut *tx).await?;
                for grant in grants {
                    sqlx::query("INSERT INTO sys_permission_grants (id, tenant_id, subject_type, subject_id, resource_type, resource_id, action, effect, grant_source, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'manual', 1, FALSE, ?, ?, ?, ?)")
                        .bind(&grant.id).bind(&grant.tenant_id).bind(&grant.subject_type).bind(&grant.subject_id).bind(&grant.resource_type).bind(&grant.resource_id).bind(&grant.action).bind(&grant.effect).bind(&grant.operator).bind(grant.now).bind(&grant.operator).bind(grant.now).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                let affected = sqlx::query("UPDATE sys_roles SET version = version + 1, updated_by = $1, updated_time = $2 WHERE tenant_id = $3 AND id = $4 AND version = $5 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(tenant_id).bind(role_id).bind(expected_version).execute(&mut *tx).await?.rows_affected();
                if affected == 0 {
                    return Err(AppError::conflict("角色权限已被修改，请刷新后重试"));
                }
                sqlx::query("UPDATE sys_permission_grants SET deleted = TRUE, deleted_by = $1, deleted_time = $2, updated_by = $3, updated_time = $4, version = version + 1 WHERE tenant_id = $5 AND subject_type = 'role' AND subject_id = $6 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(role_id).execute(&mut *tx).await?;
                for grant in grants {
                    sqlx::query("INSERT INTO sys_permission_grants (id, tenant_id, subject_type, subject_id, resource_type, resource_id, action, effect, grant_source, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'manual', 1, FALSE, $9, $10, $11, $12)")
                        .bind(&grant.id).bind(&grant.tenant_id).bind(&grant.subject_type).bind(&grant.subject_id).bind(&grant.resource_type).bind(&grant.resource_id).bind(&grant.action).bind(&grant.effect).bind(&grant.operator).bind(grant.now).bind(&grant.operator).bind(grant.now).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
        }
        Ok(())
    }

    pub async fn list_role_grants(
        pool: &DatabasePool,
        tenant_id: &str,
        role_id: &str,
    ) -> AppResult<Vec<PermissionGrant>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionGrant>("SELECT * FROM sys_permission_grants WHERE tenant_id = ? AND subject_type = 'role' AND subject_id = ? AND deleted = FALSE")
                .bind(tenant_id).bind(role_id).fetch_all(pool).await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionGrant>("SELECT * FROM sys_permission_grants WHERE tenant_id = $1 AND subject_type = 'role' AND subject_id = $2 AND deleted = FALSE")
                .bind(tenant_id).bind(role_id).fetch_all(pool).await?),
        }
    }

    pub async fn list_inherited_role_grants(
        pool: &DatabasePool,
        tenant_id: &str,
        role_id: &str,
    ) -> AppResult<Vec<PermissionGrant>> {
        let sql_mysql = "SELECT g.* FROM sys_role_closures rc JOIN sys_permission_grants g ON g.tenant_id = rc.tenant_id AND g.subject_type = 'role' AND g.subject_id = rc.ancestor_role_id AND g.deleted = FALSE WHERE rc.tenant_id = ? AND rc.descendant_role_id = ? AND rc.deleted = FALSE ORDER BY rc.depth ASC, g.updated_time DESC";
        let sql_pg = "SELECT g.* FROM sys_role_closures rc JOIN sys_permission_grants g ON g.tenant_id = rc.tenant_id AND g.subject_type = 'role' AND g.subject_id = rc.ancestor_role_id AND g.deleted = FALSE WHERE rc.tenant_id = $1 AND rc.descendant_role_id = $2 AND rc.deleted = FALSE ORDER BY rc.depth ASC, g.updated_time DESC";
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionGrant>(sql_mysql)
                .bind(tenant_id)
                .bind(role_id)
                .fetch_all(pool)
                .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionGrant>(sql_pg)
                .bind(tenant_id)
                .bind(role_id)
                .fetch_all(pool)
                .await?),
        }
    }

    pub async fn list_effective_permissions(
        pool: &DatabasePool,
        tenant_id: &str,
        account_id: &str,
    ) -> AppResult<Vec<PermissionGrant>> {
        let sql_mysql = "SELECT g.* FROM sys_account_roles ar JOIN sys_role_closures rc ON rc.tenant_id = ar.tenant_id AND rc.descendant_role_id = ar.role_id AND rc.deleted = FALSE JOIN sys_roles r ON r.id = rc.ancestor_role_id AND r.tenant_id = ar.tenant_id AND r.deleted = FALSE AND r.status = 'enabled' JOIN sys_permission_grants g ON g.tenant_id = ar.tenant_id AND g.subject_type = 'role' AND g.subject_id = rc.ancestor_role_id AND g.deleted = FALSE WHERE ar.tenant_id = ? AND ar.account_id = ? AND ar.deleted = FALSE AND ar.status = 'enabled'";
        let sql_pg = sql_mysql.replacen('?', "$1", 1).replacen('?', "$2", 1);
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionGrant>(sql_mysql)
                .bind(tenant_id)
                .bind(account_id)
                .fetch_all(pool)
                .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionGrant>(&sql_pg)
                .bind(tenant_id)
                .bind(account_id)
                .fetch_all(pool)
                .await?),
        }
    }

    pub async fn resource_exists(
        pool: &DatabasePool,
        tenant_id: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> AppResult<bool> {
        let sql_mysql = match resource_type {
            "application" => {
                "SELECT COUNT(*) FROM sys_permission_applications WHERE id = ? AND (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE"
            }
            "menu" => {
                "SELECT COUNT(*) FROM sys_permission_menus WHERE id = ? AND (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE"
            }
            "button" => {
                "SELECT COUNT(*) FROM sys_permission_buttons WHERE id = ? AND (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE"
            }
            "api" => {
                "SELECT COUNT(*) FROM sys_permission_apis WHERE id = ? AND (tenant_id = ? OR tenant_id IS NULL) AND deleted = FALSE"
            }
            _ => return Ok(false),
        };
        let sql_pg = match resource_type {
            "application" => {
                "SELECT COUNT(*) FROM sys_permission_applications WHERE id = $1 AND (tenant_id = $2 OR tenant_id IS NULL) AND deleted = FALSE"
            }
            "menu" => {
                "SELECT COUNT(*) FROM sys_permission_menus WHERE id = $1 AND (tenant_id = $2 OR tenant_id IS NULL) AND deleted = FALSE"
            }
            "button" => {
                "SELECT COUNT(*) FROM sys_permission_buttons WHERE id = $1 AND (tenant_id = $2 OR tenant_id IS NULL) AND deleted = FALSE"
            }
            "api" => {
                "SELECT COUNT(*) FROM sys_permission_apis WHERE id = $1 AND (tenant_id = $2 OR tenant_id IS NULL) AND deleted = FALSE"
            }
            _ => return Ok(false),
        };
        let count: i64 = match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query_scalar(sql_mysql)
                    .bind(resource_id)
                    .bind(tenant_id)
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar(sql_pg)
                    .bind(resource_id)
                    .bind(tenant_id)
                    .fetch_one(pool)
                    .await?
            }
        };
        Ok(count > 0)
    }

    pub async fn list_resource_grants(
        pool: &DatabasePool,
        tenant_id: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> AppResult<Vec<PermissionGrant>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, PermissionGrant>("SELECT * FROM sys_permission_grants WHERE tenant_id = ? AND resource_type = ? AND resource_id = ? AND deleted = FALSE ORDER BY updated_time DESC")
                .bind(tenant_id).bind(resource_type).bind(resource_id).fetch_all(pool).await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, PermissionGrant>("SELECT * FROM sys_permission_grants WHERE tenant_id = $1 AND resource_type = $2 AND resource_id = $3 AND deleted = FALSE ORDER BY updated_time DESC")
                .bind(tenant_id).bind(resource_type).bind(resource_id).fetch_all(pool).await?),
        }
    }

    pub async fn replace_resource_role_grants(
        pool: &DatabasePool,
        tenant_id: &str,
        resource_type: &str,
        resource_id: &str,
        action: &str,
        grants: &[GrantWrite],
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                let mut tx = pool.begin().await?;
                sqlx::query("UPDATE sys_permission_grants SET deleted = TRUE, deleted_by = ?, deleted_time = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND subject_type = 'role' AND resource_type = ? AND resource_id = ? AND action = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(resource_type).bind(resource_id).bind(action).execute(&mut *tx).await?;
                for grant in grants {
                    sqlx::query("INSERT INTO sys_permission_grants (id, tenant_id, subject_type, subject_id, resource_type, resource_id, action, effect, grant_source, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'manual', 1, FALSE, ?, ?, ?, ?)")
                        .bind(&grant.id).bind(&grant.tenant_id).bind(&grant.subject_type).bind(&grant.subject_id).bind(&grant.resource_type).bind(&grant.resource_id).bind(&grant.action).bind(&grant.effect).bind(&grant.operator).bind(grant.now).bind(&grant.operator).bind(grant.now).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                sqlx::query("UPDATE sys_permission_grants SET deleted = TRUE, deleted_by = $1, deleted_time = $2, updated_by = $3, updated_time = $4, version = version + 1 WHERE tenant_id = $5 AND subject_type = 'role' AND resource_type = $6 AND resource_id = $7 AND action = $8 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(resource_type).bind(resource_id).bind(action).execute(&mut *tx).await?;
                for grant in grants {
                    sqlx::query("INSERT INTO sys_permission_grants (id, tenant_id, subject_type, subject_id, resource_type, resource_id, action, effect, grant_source, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'manual', 1, FALSE, $9, $10, $11, $12)")
                        .bind(&grant.id).bind(&grant.tenant_id).bind(&grant.subject_type).bind(&grant.subject_id).bind(&grant.resource_type).bind(&grant.resource_id).bind(&grant.action).bind(&grant.effect).bind(&grant.operator).bind(grant.now).bind(&grant.operator).bind(grant.now).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
        }
        Ok(())
    }

    pub async fn list_account_roles(
        pool: &DatabasePool,
        tenant_id: &str,
        account_id: &str,
    ) -> AppResult<Vec<AccountRole>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, AccountRole>("SELECT * FROM sys_account_roles WHERE tenant_id = ? AND account_id = ? AND deleted = FALSE ORDER BY updated_time DESC")
                .bind(tenant_id).bind(account_id).fetch_all(pool).await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, AccountRole>("SELECT * FROM sys_account_roles WHERE tenant_id = $1 AND account_id = $2 AND deleted = FALSE ORDER BY updated_time DESC")
                .bind(tenant_id).bind(account_id).fetch_all(pool).await?),
        }
    }

    pub async fn replace_account_roles(
        pool: &DatabasePool,
        tenant_id: &str,
        account_id: &str,
        expected_version: Option<i64>,
        role_ids: &[String],
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                let mut tx = pool.begin().await?;
                let current_version: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM sys_account_roles WHERE tenant_id = ? AND account_id = ? AND deleted = FALSE")
                    .bind(tenant_id).bind(account_id).fetch_one(&mut *tx).await?;
                if let Some(expected_version) = expected_version {
                    if current_version != expected_version {
                        return Err(AppError::conflict("账号角色已被修改，请刷新后重试"));
                    }
                }
                let next_version = current_version + 1;
                sqlx::query("UPDATE sys_account_roles SET deleted = TRUE, deleted_by = ?, deleted_time = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND account_id = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(account_id).execute(&mut *tx).await?;
                for role_id in role_ids {
                    sqlx::query("INSERT INTO sys_account_roles (id, tenant_id, account_id, role_id, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, 'enabled', ?, FALSE, ?, ?, ?, ?)")
                        .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(account_id).bind(role_id).bind(next_version).bind(operator).bind(now).bind(operator).bind(now).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                let current_version: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM sys_account_roles WHERE tenant_id = $1 AND account_id = $2 AND deleted = FALSE")
                    .bind(tenant_id).bind(account_id).fetch_one(&mut *tx).await?;
                if let Some(expected_version) = expected_version {
                    if current_version != expected_version {
                        return Err(AppError::conflict("账号角色已被修改，请刷新后重试"));
                    }
                }
                let next_version = current_version + 1;
                sqlx::query("UPDATE sys_account_roles SET deleted = TRUE, deleted_by = $1, deleted_time = $2, updated_by = $3, updated_time = $4, version = version + 1 WHERE tenant_id = $5 AND account_id = $6 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(account_id).execute(&mut *tx).await?;
                for role_id in role_ids {
                    sqlx::query("INSERT INTO sys_account_roles (id, tenant_id, account_id, role_id, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, 'enabled', $5, FALSE, $6, $7, $8, $9)")
                        .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(account_id).bind(role_id).bind(next_version).bind(operator).bind(now).bind(operator).bind(now).execute(&mut *tx).await?;
                }
                tx.commit().await?;
            }
        }
        Ok(())
    }

    pub async fn permission_version(pool: &DatabasePool, tenant_id: &str) -> AppResult<i64> {
        match pool {
            DatabasePool::MySql(pool) => {
                let value: Option<i64> = sqlx::query_scalar(
                    "SELECT version_no FROM sys_permission_versions WHERE tenant_id = ? AND deleted = FALSE",
                )
                .bind(tenant_id)
                .fetch_optional(pool)
                .await?;
                Ok(value.unwrap_or(0))
            }
            DatabasePool::Postgres(pool) => {
                let value: Option<i64> = sqlx::query_scalar(
                    "SELECT version_no FROM sys_permission_versions WHERE tenant_id = $1 AND deleted = FALSE",
                )
                .bind(tenant_id)
                .fetch_optional(pool)
                .await?;
                Ok(value.unwrap_or(0))
            }
        }
    }

    pub async fn bump_permission_version(
        pool: &DatabasePool,
        tenant_id: &str,
        reason: &str,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                let affected = sqlx::query("UPDATE sys_permission_versions SET version_no = version_no + 1, changed_reason = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND deleted = FALSE")
                    .bind(reason).bind(operator).bind(now).bind(tenant_id).execute(pool).await?.rows_affected();
                if affected == 0 {
                    sqlx::query("INSERT INTO sys_permission_versions (id, tenant_id, version_no, changed_reason, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, 1, ?, 1, FALSE, ?, ?, ?, ?)")
                        .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(reason).bind(operator).bind(now).bind(operator).bind(now).execute(pool).await?;
                }
            }
            DatabasePool::Postgres(pool) => {
                let affected = sqlx::query("UPDATE sys_permission_versions SET version_no = version_no + 1, changed_reason = $1, updated_by = $2, updated_time = $3, version = version + 1 WHERE tenant_id = $4 AND deleted = FALSE")
                    .bind(reason).bind(operator).bind(now).bind(tenant_id).execute(pool).await?.rows_affected();
                if affected == 0 {
                    sqlx::query("INSERT INTO sys_permission_versions (id, tenant_id, version_no, changed_reason, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, 1, $3, 1, FALSE, $4, $5, $6, $7)")
                        .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(reason).bind(operator).bind(now).bind(operator).bind(now).execute(pool).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn role_exists(
        pool: &DatabasePool,
        tenant_id: &str,
        role_id: &str,
    ) -> AppResult<bool> {
        let count: i64 = match pool {
            DatabasePool::MySql(pool) => sqlx::query_scalar(
                "SELECT COUNT(*) FROM sys_roles WHERE tenant_id = ? AND id = ? AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(role_id)
            .fetch_one(pool)
            .await?,
            DatabasePool::Postgres(pool) => sqlx::query_scalar(
                "SELECT COUNT(*) FROM sys_roles WHERE tenant_id = $1 AND id = $2 AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(role_id)
            .fetch_one(pool)
            .await?,
        };
        Ok(count > 0)
    }

    pub async fn account_exists_in_tenant(
        pool: &DatabasePool,
        tenant_id: &str,
        account_id: &str,
    ) -> AppResult<bool> {
        let count: i64 = match pool {
            DatabasePool::MySql(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM sys_accounts a JOIN sys_account_tenants at ON at.account_id = a.id AND at.tenant_id = ? AND at.deleted = FALSE WHERE a.id = ? AND a.deleted = FALSE")
                .bind(tenant_id).bind(account_id).fetch_one(pool).await?,
            DatabasePool::Postgres(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM sys_accounts a JOIN sys_account_tenants at ON at.account_id = a.id AND at.tenant_id = $1 AND at.deleted = FALSE WHERE a.id = $2 AND a.deleted = FALSE")
                .bind(tenant_id).bind(account_id).fetch_one(pool).await?,
        };
        Ok(count > 0)
    }

    pub async fn role_is_descendant(
        pool: &DatabasePool,
        tenant_id: &str,
        ancestor_role_id: &str,
        descendant_role_id: &str,
    ) -> AppResult<bool> {
        let count: i64 = match pool {
            DatabasePool::MySql(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM sys_role_closures WHERE tenant_id = ? AND ancestor_role_id = ? AND descendant_role_id = ? AND deleted = FALSE")
                .bind(tenant_id).bind(ancestor_role_id).bind(descendant_role_id).fetch_one(pool).await?,
            DatabasePool::Postgres(pool) => sqlx::query_scalar("SELECT COUNT(*) FROM sys_role_closures WHERE tenant_id = $1 AND ancestor_role_id = $2 AND descendant_role_id = $3 AND deleted = FALSE")
                .bind(tenant_id).bind(ancestor_role_id).bind(descendant_role_id).fetch_one(pool).await?,
        };
        Ok(count > 0)
    }

    pub async fn replace_role_parents(
        pool: &DatabasePool,
        tenant_id: &str,
        role_id: &str,
        expected_version: i64,
        parent_role_ids: &[String],
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                let mut tx = pool.begin().await?;
                let affected = sqlx::query("UPDATE sys_roles SET version = version + 1, updated_by = ?, updated_time = ? WHERE tenant_id = ? AND id = ? AND version = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(tenant_id).bind(role_id).bind(expected_version).execute(&mut *tx).await?.rows_affected();
                if affected == 0 {
                    return Err(AppError::conflict("角色继承已被修改，请刷新后重试"));
                }
                sqlx::query("UPDATE sys_role_relations SET deleted = TRUE, deleted_by = ?, deleted_time = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND child_role_id = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(role_id).execute(&mut *tx).await?;
                for parent_id in parent_role_ids {
                    sqlx::query("INSERT INTO sys_role_relations (id, tenant_id, parent_role_id, child_role_id, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)")
                        .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(parent_id).bind(role_id).bind(operator).bind(now).bind(operator).bind(now).execute(&mut *tx).await?;
                }
                sqlx::query("UPDATE sys_role_closures SET deleted = TRUE, deleted_by = ?, deleted_time = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) SELECT REPLACE(UUID(), '-', ''), tenant_id, id, id, 0, 1, FALSE, ?, ?, ?, ? FROM sys_roles WHERE tenant_id = ? AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) WITH RECURSIVE cte AS (SELECT tenant_id, parent_role_id, child_role_id, 1 AS depth FROM sys_role_relations WHERE tenant_id = ? AND deleted = FALSE UNION ALL SELECT c.tenant_id, c.parent_role_id, r.child_role_id, c.depth + 1 FROM cte c JOIN sys_role_relations r ON r.tenant_id = c.tenant_id AND r.parent_role_id = c.child_role_id AND r.deleted = FALSE WHERE c.depth < 32) SELECT REPLACE(UUID(), '-', ''), tenant_id, parent_role_id, child_role_id, MIN(depth), 1, FALSE, ?, ?, ?, ? FROM cte GROUP BY tenant_id, parent_role_id, child_role_id")
                    .bind(tenant_id).bind(operator).bind(now).bind(operator).bind(now).execute(&mut *tx).await?;
                tx.commit().await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                let affected = sqlx::query("UPDATE sys_roles SET version = version + 1, updated_by = $1, updated_time = $2 WHERE tenant_id = $3 AND id = $4 AND version = $5 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(tenant_id).bind(role_id).bind(expected_version).execute(&mut *tx).await?.rows_affected();
                if affected == 0 {
                    return Err(AppError::conflict("角色继承已被修改，请刷新后重试"));
                }
                sqlx::query("UPDATE sys_role_relations SET deleted = TRUE, deleted_by = $1, deleted_time = $2, updated_by = $3, updated_time = $4, version = version + 1 WHERE tenant_id = $5 AND child_role_id = $6 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).bind(role_id).execute(&mut *tx).await?;
                for parent_id in parent_role_ids {
                    sqlx::query("INSERT INTO sys_role_relations (id, tenant_id, parent_role_id, child_role_id, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, 1, FALSE, $5, $6, $7, $8)")
                        .bind(crate::utils::id::generate_business_id()).bind(tenant_id).bind(parent_id).bind(role_id).bind(operator).bind(now).bind(operator).bind(now).execute(&mut *tx).await?;
                }
                sqlx::query("UPDATE sys_role_closures SET deleted = TRUE, deleted_by = $1, deleted_time = $2, updated_by = $3, updated_time = $4, version = version + 1 WHERE tenant_id = $5 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) SELECT md5(random()::text || clock_timestamp()::text || id), tenant_id, id, id, 0, 1, FALSE, $1, $2, $3, $4 FROM sys_roles WHERE tenant_id = $5 AND deleted = FALSE")
                    .bind(operator).bind(now).bind(operator).bind(now).bind(tenant_id).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO sys_role_closures (id, tenant_id, ancestor_role_id, descendant_role_id, depth, version, deleted, created_by, created_time, updated_by, updated_time) WITH RECURSIVE cte AS (SELECT tenant_id, parent_role_id, child_role_id, 1 AS depth FROM sys_role_relations WHERE tenant_id = $1 AND deleted = FALSE UNION ALL SELECT c.tenant_id, c.parent_role_id, r.child_role_id, c.depth + 1 FROM cte c JOIN sys_role_relations r ON r.tenant_id = c.tenant_id AND r.parent_role_id = c.child_role_id AND r.deleted = FALSE WHERE c.depth < 32) SELECT md5(parent_role_id || '#' || child_role_id || '#' || random()::text || clock_timestamp()::text), tenant_id, parent_role_id, child_role_id, MIN(depth), 1, FALSE, $2, $3, $4, $5 FROM cte GROUP BY tenant_id, parent_role_id, child_role_id")
                    .bind(tenant_id).bind(operator).bind(now).bind(operator).bind(now).execute(&mut *tx).await?;
                tx.commit().await?;
            }
        }
        Ok(())
    }

    pub async fn role_version(pool: &DatabasePool, role_id: &str) -> AppResult<Option<i64>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query(
                "SELECT version FROM sys_roles WHERE id = ? AND deleted = FALSE",
            )
            .bind(role_id)
            .fetch_optional(pool)
            .await?
            .map(|row| row.get::<i64, _>("version"))),
            DatabasePool::Postgres(pool) => Ok(sqlx::query(
                "SELECT version FROM sys_roles WHERE id = $1 AND deleted = FALSE",
            )
            .bind(role_id)
            .fetch_optional(pool)
            .await?
            .map(|row| row.get::<i64, _>("version"))),
        }
    }
}
