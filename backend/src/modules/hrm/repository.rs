//! HRM 数据访问层。
//!
//! repository 只负责显式 SQL 和数据库方言差异，不承载业务规则。所有默认查询都过滤
//! `deleted = FALSE`，写入方法必须由 service 先完成参数、权限、状态和层级校验。

use chrono::{DateTime, Utc};
use sqlx::{MySql, Postgres, QueryBuilder, Row};

use crate::{
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::hrm::{
        dto::{OrgPageQuery, PostPageQuery, UserOrgPostPageQuery, UserPageQuery},
        model::{HrmOrg, HrmPost, HrmUser, HrmUserOrgPost},
    },
    response::page::NormalizedPageQuery,
};

/// 新增或更新用户时 repository 需要的完整字段。
#[derive(Debug, Clone)]
pub struct UserWrite {
    pub id: String,
    pub tenant_id: String,
    pub employee_no: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrgWrite {
    pub id: String,
    pub tenant_id: String,
    pub parent_id: Option<String>,
    pub org_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub org_type: String,
    pub sort_no: i64,
    pub status: String,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PostWrite {
    pub id: String,
    pub tenant_id: String,
    pub post_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub sort_no: i64,
    pub status: String,
    pub operator: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct UserOrgPostWrite {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub org_id: String,
    pub post_id: String,
    pub primary_org: bool,
    pub primary_post: bool,
    pub sort_no: i64,
    pub operator: String,
    pub now: DateTime<Utc>,
}

pub struct HrmRepository;

impl HrmRepository {
    pub async fn insert_user(pool: &DatabasePool, data: &UserWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query(
                    "INSERT INTO hrm_users (id, tenant_id, employee_no, name, name_full_pinyin, name_simple_pinyin, mobile, email, sort_no, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)",
                )
                .bind(&data.id)
                .bind(&data.tenant_id)
                .bind(&data.employee_no)
                .bind(&data.name)
                .bind(&data.name_full_pinyin)
                .bind(&data.name_simple_pinyin)
                .bind(&data.mobile)
                .bind(&data.email)
                .bind(data.sort_no)
                .bind(&data.status)
                .bind(&data.operator)
                .bind(data.now)
                .bind(&data.operator)
                .bind(data.now)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO hrm_users (id, tenant_id, employee_no, name, name_full_pinyin, name_simple_pinyin, mobile, email, sort_no, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, FALSE, $11, $12, $13, $14)",
                )
                .bind(&data.id)
                .bind(&data.tenant_id)
                .bind(&data.employee_no)
                .bind(&data.name)
                .bind(&data.name_full_pinyin)
                .bind(&data.name_simple_pinyin)
                .bind(&data.mobile)
                .bind(&data.email)
                .bind(data.sort_no)
                .bind(&data.status)
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

    pub async fn update_user(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        data: &UserWrite,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE hrm_users SET employee_no = ?, name = ?, name_full_pinyin = ?, name_simple_pinyin = ?, mobile = ?, email = ?, sort_no = ?, status = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND tenant_id = ? AND version = ? AND deleted = FALSE",
            )
            .bind(&data.employee_no)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.mobile)
            .bind(&data.email)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE hrm_users SET employee_no = $1, name = $2, name_full_pinyin = $3, name_simple_pinyin = $4, mobile = $5, email = $6, sort_no = $7, status = $8, updated_by = $9, updated_time = $10, version = version + 1 WHERE id = $11 AND tenant_id = $12 AND version = $13 AND deleted = FALSE",
            )
            .bind(&data.employee_no)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.mobile)
            .bind(&data.email)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(affected)
    }

    pub async fn insert_org(pool: &DatabasePool, data: &OrgWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query(
                "INSERT INTO hrm_orgs (id, tenant_id, parent_id, org_code, name, name_full_pinyin, name_simple_pinyin, org_type, sort_no, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)",
            )
            .bind(&data.id)
            .bind(&data.tenant_id)
            .bind(&data.parent_id)
            .bind(&data.org_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.org_type)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.operator)
            .bind(data.now)
            .execute(pool)
            .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                "INSERT INTO hrm_orgs (id, tenant_id, parent_id, org_code, name, name_full_pinyin, name_simple_pinyin, org_type, sort_no, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 1, FALSE, $11, $12, $13, $14)",
            )
            .bind(&data.id)
            .bind(&data.tenant_id)
            .bind(&data.parent_id)
            .bind(&data.org_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.org_type)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.operator)
            .bind(data.now)
            .execute(pool)
            .await?;
            }
        };
        Ok(())
    }

    pub async fn update_org(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        data: &OrgWrite,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE hrm_orgs SET parent_id = ?, org_code = ?, name = ?, name_full_pinyin = ?, name_simple_pinyin = ?, org_type = ?, sort_no = ?, status = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND tenant_id = ? AND version = ? AND deleted = FALSE",
            )
            .bind(&data.parent_id)
            .bind(&data.org_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.org_type)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE hrm_orgs SET parent_id = $1, org_code = $2, name = $3, name_full_pinyin = $4, name_simple_pinyin = $5, org_type = $6, sort_no = $7, status = $8, updated_by = $9, updated_time = $10, version = version + 1 WHERE id = $11 AND tenant_id = $12 AND version = $13 AND deleted = FALSE",
            )
            .bind(&data.parent_id)
            .bind(&data.org_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(&data.org_type)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(affected)
    }

    pub async fn insert_post(pool: &DatabasePool, data: &PostWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query(
                "INSERT INTO hrm_posts (id, tenant_id, post_code, name, name_full_pinyin, name_simple_pinyin, sort_no, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)",
            )
            .bind(&data.id)
            .bind(&data.tenant_id)
            .bind(&data.post_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.operator)
            .bind(data.now)
            .execute(pool)
            .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                "INSERT INTO hrm_posts (id, tenant_id, post_code, name, name_full_pinyin, name_simple_pinyin, sort_no, status, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, FALSE, $9, $10, $11, $12)",
            )
            .bind(&data.id)
            .bind(&data.tenant_id)
            .bind(&data.post_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.operator)
            .bind(data.now)
            .execute(pool)
            .await?;
            }
        };
        Ok(())
    }

    pub async fn update_post(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        data: &PostWrite,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE hrm_posts SET post_code = ?, name = ?, name_full_pinyin = ?, name_simple_pinyin = ?, sort_no = ?, status = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND tenant_id = ? AND version = ? AND deleted = FALSE",
            )
            .bind(&data.post_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE hrm_posts SET post_code = $1, name = $2, name_full_pinyin = $3, name_simple_pinyin = $4, sort_no = $5, status = $6, updated_by = $7, updated_time = $8, version = version + 1 WHERE id = $9 AND tenant_id = $10 AND version = $11 AND deleted = FALSE",
            )
            .bind(&data.post_code)
            .bind(&data.name)
            .bind(&data.name_full_pinyin)
            .bind(&data.name_simple_pinyin)
            .bind(data.sort_no)
            .bind(&data.status)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(affected)
    }

    pub async fn insert_relation(pool: &DatabasePool, data: &UserOrgPostWrite) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                sqlx::query(
                "INSERT INTO hrm_user_org_posts (id, tenant_id, user_id, org_id, post_id, primary_org, primary_post, sort_no, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)",
            )
            .bind(&data.id)
            .bind(&data.tenant_id)
            .bind(&data.user_id)
            .bind(&data.org_id)
            .bind(&data.post_id)
            .bind(data.primary_org)
            .bind(data.primary_post)
            .bind(data.sort_no)
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.operator)
            .bind(data.now)
            .execute(pool)
            .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                "INSERT INTO hrm_user_org_posts (id, tenant_id, user_id, org_id, post_id, primary_org, primary_post, sort_no, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, FALSE, $9, $10, $11, $12)",
            )
            .bind(&data.id)
            .bind(&data.tenant_id)
            .bind(&data.user_id)
            .bind(&data.org_id)
            .bind(&data.post_id)
            .bind(data.primary_org)
            .bind(data.primary_post)
            .bind(data.sort_no)
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.operator)
            .bind(data.now)
            .execute(pool)
            .await?;
            }
        };
        Ok(())
    }

    pub async fn update_relation(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        data: &UserOrgPostWrite,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => sqlx::query(
                "UPDATE hrm_user_org_posts SET user_id = ?, org_id = ?, post_id = ?, primary_org = ?, primary_post = ?, sort_no = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND tenant_id = ? AND version = ? AND deleted = FALSE",
            )
            .bind(&data.user_id)
            .bind(&data.org_id)
            .bind(&data.post_id)
            .bind(data.primary_org)
            .bind(data.primary_post)
            .bind(data.sort_no)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(
                "UPDATE hrm_user_org_posts SET user_id = $1, org_id = $2, post_id = $3, primary_org = $4, primary_post = $5, sort_no = $6, updated_by = $7, updated_time = $8, version = version + 1 WHERE id = $9 AND tenant_id = $10 AND version = $11 AND deleted = FALSE",
            )
            .bind(&data.user_id)
            .bind(&data.org_id)
            .bind(&data.post_id)
            .bind(data.primary_org)
            .bind(data.primary_post)
            .bind(data.sort_no)
            .bind(&data.operator)
            .bind(data.now)
            .bind(id)
            .bind(&data.tenant_id)
            .bind(version)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(affected)
    }

    pub async fn insert_relation_with_primary_clear(
        pool: &DatabasePool,
        data: &UserOrgPostWrite,
    ) -> AppResult<()> {
        match pool {
            DatabasePool::MySql(pool) => {
                let mut tx = pool.begin().await?;
                clear_primary_flags_mysql(&mut tx, data).await?;
                sqlx::query(
                    "INSERT INTO hrm_user_org_posts (id, tenant_id, user_id, org_id, post_id, primary_org, primary_post, sort_no, version, deleted, created_by, created_time, updated_by, updated_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, FALSE, ?, ?, ?, ?)",
                )
                .bind(&data.id)
                .bind(&data.tenant_id)
                .bind(&data.user_id)
                .bind(&data.org_id)
                .bind(&data.post_id)
                .bind(data.primary_org)
                .bind(data.primary_post)
                .bind(data.sort_no)
                .bind(&data.operator)
                .bind(data.now)
                .bind(&data.operator)
                .bind(data.now)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
            }
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                clear_primary_flags_postgres(&mut tx, data).await?;
                sqlx::query(
                    "INSERT INTO hrm_user_org_posts (id, tenant_id, user_id, org_id, post_id, primary_org, primary_post, sort_no, version, deleted, created_by, created_time, updated_by, updated_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, FALSE, $9, $10, $11, $12)",
                )
                .bind(&data.id)
                .bind(&data.tenant_id)
                .bind(&data.user_id)
                .bind(&data.org_id)
                .bind(&data.post_id)
                .bind(data.primary_org)
                .bind(data.primary_post)
                .bind(data.sort_no)
                .bind(&data.operator)
                .bind(data.now)
                .bind(&data.operator)
                .bind(data.now)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
            }
        }
        Ok(())
    }

    pub async fn update_relation_with_primary_clear(
        pool: &DatabasePool,
        id: &str,
        version: i64,
        data: &UserOrgPostWrite,
    ) -> AppResult<u64> {
        let affected = match pool {
            DatabasePool::MySql(pool) => {
                let mut tx = pool.begin().await?;
                clear_primary_flags_mysql(&mut tx, data).await?;
                let affected = sqlx::query(
                    "UPDATE hrm_user_org_posts SET user_id = ?, org_id = ?, post_id = ?, primary_org = ?, primary_post = ?, sort_no = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE id = ? AND tenant_id = ? AND version = ? AND deleted = FALSE",
                )
                .bind(&data.user_id)
                .bind(&data.org_id)
                .bind(&data.post_id)
                .bind(data.primary_org)
                .bind(data.primary_post)
                .bind(data.sort_no)
                .bind(&data.operator)
                .bind(data.now)
                .bind(id)
                .bind(&data.tenant_id)
                .bind(version)
                .execute(&mut *tx)
                .await?
                .rows_affected();
                if affected == 0 {
                    tx.rollback().await?;
                    return Ok(0);
                }
                tx.commit().await?;
                affected
            }
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                clear_primary_flags_postgres(&mut tx, data).await?;
                let affected = sqlx::query(
                    "UPDATE hrm_user_org_posts SET user_id = $1, org_id = $2, post_id = $3, primary_org = $4, primary_post = $5, sort_no = $6, updated_by = $7, updated_time = $8, version = version + 1 WHERE id = $9 AND tenant_id = $10 AND version = $11 AND deleted = FALSE",
                )
                .bind(&data.user_id)
                .bind(&data.org_id)
                .bind(&data.post_id)
                .bind(data.primary_org)
                .bind(data.primary_post)
                .bind(data.sort_no)
                .bind(&data.operator)
                .bind(data.now)
                .bind(id)
                .bind(&data.tenant_id)
                .bind(version)
                .execute(&mut *tx)
                .await?
                .rows_affected();
                if affected == 0 {
                    tx.rollback().await?;
                    return Ok(0);
                }
                tx.commit().await?;
                affected
            }
        };
        Ok(affected)
    }

    pub async fn clear_primary_flags(
        pool: &DatabasePool,
        tenant_id: &str,
        user_id: &str,
        keep_id: &str,
        clear_org: bool,
        clear_post: bool,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<()> {
        if !clear_org && !clear_post {
            return Ok(());
        }
        let set_clause = match (clear_org, clear_post) {
            (true, true) => "primary_org = FALSE, primary_post = FALSE",
            (true, false) => "primary_org = FALSE",
            (false, true) => "primary_post = FALSE",
            (false, false) => unreachable!(),
        };
        match pool {
            DatabasePool::MySql(pool) => {
                let sql = format!(
                    "UPDATE hrm_user_org_posts SET {set_clause}, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND user_id = ? AND id <> ? AND deleted = FALSE"
                );
                sqlx::query(&sql)
                    .bind(operator)
                    .bind(now)
                    .bind(tenant_id)
                    .bind(user_id)
                    .bind(keep_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                let sql = format!(
                    "UPDATE hrm_user_org_posts SET {set_clause}, updated_by = $1, updated_time = $2, version = version + 1 WHERE tenant_id = $3 AND user_id = $4 AND id <> $5 AND deleted = FALSE"
                );
                sqlx::query(&sql)
                    .bind(operator)
                    .bind(now)
                    .bind(tenant_id)
                    .bind(user_id)
                    .bind(keep_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_user(
        pool: &DatabasePool,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<Option<HrmUser>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, HrmUser>(
                "SELECT * FROM hrm_users WHERE tenant_id = ? AND id = ? AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, HrmUser>(
                "SELECT * FROM hrm_users WHERE tenant_id = $1 AND id = $2 AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn list_users_by_ids(
        pool: &DatabasePool,
        tenant_id: &str,
        ids: &[String],
    ) -> AppResult<Vec<HrmUser>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        match pool {
            DatabasePool::MySql(pool) => {
                let mut builder = QueryBuilder::<MySql>::new(
                    "SELECT * FROM hrm_users WHERE tenant_id = ",
                );
                builder
                    .push_bind(tenant_id.to_string())
                    .push(" AND deleted = FALSE AND id IN (");
                let mut separated = builder.separated(", ");
                for id in ids {
                    separated.push_bind(id);
                }
                separated.push_unseparated(")");
                Ok(builder.build_query_as::<HrmUser>().fetch_all(pool).await?)
            }
            DatabasePool::Postgres(pool) => {
                let mut builder = QueryBuilder::<Postgres>::new(
                    "SELECT * FROM hrm_users WHERE tenant_id = ",
                );
                builder
                    .push_bind(tenant_id.to_string())
                    .push(" AND deleted = FALSE AND id IN (");
                let mut separated = builder.separated(", ");
                for id in ids {
                    separated.push_bind(id);
                }
                separated.push_unseparated(")");
                Ok(builder.build_query_as::<HrmUser>().fetch_all(pool).await?)
            }
        }
    }

    pub async fn get_org(
        pool: &DatabasePool,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<Option<HrmOrg>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, HrmOrg>(
                "SELECT * FROM hrm_orgs WHERE tenant_id = ? AND id = ? AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, HrmOrg>(
                "SELECT * FROM hrm_orgs WHERE tenant_id = $1 AND id = $2 AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn get_post(
        pool: &DatabasePool,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<Option<HrmPost>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, HrmPost>(
                "SELECT * FROM hrm_posts WHERE tenant_id = ? AND id = ? AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, HrmPost>(
                "SELECT * FROM hrm_posts WHERE tenant_id = $1 AND id = $2 AND deleted = FALSE",
            )
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn get_relation(
        pool: &DatabasePool,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<Option<HrmUserOrgPost>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, HrmUserOrgPost>("SELECT * FROM hrm_user_org_posts WHERE tenant_id = ? AND id = ? AND deleted = FALSE")
                .bind(tenant_id).bind(id).fetch_optional(pool).await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, HrmUserOrgPost>("SELECT * FROM hrm_user_org_posts WHERE tenant_id = $1 AND id = $2 AND deleted = FALSE")
                .bind(tenant_id).bind(id).fetch_optional(pool).await?),
        }
    }

    pub async fn logical_delete(
        pool: &DatabasePool,
        table: &str,
        tenant_id: &str,
        id: &str,
        version: i64,
        operator: &str,
        now: DateTime<Utc>,
    ) -> AppResult<u64> {
        let allowed = matches!(
            table,
            "hrm_users" | "hrm_orgs" | "hrm_posts" | "hrm_user_org_posts"
        );
        if !allowed {
            return Err(AppError::system("HRM 逻辑删除表名不在白名单内"));
        }
        let affected = match pool {
            DatabasePool::MySql(pool) => {
                let sql = format!(
                    "UPDATE {table} SET deleted = TRUE, deleted_by = ?, deleted_time = ?, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND id = ? AND version = ? AND deleted = FALSE"
                );
                sqlx::query(&sql)
                    .bind(operator)
                    .bind(now)
                    .bind(operator)
                    .bind(now)
                    .bind(tenant_id)
                    .bind(id)
                    .bind(version)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                let sql = format!(
                    "UPDATE {table} SET deleted = TRUE, deleted_by = $1, deleted_time = $2, updated_by = $3, updated_time = $4, version = version + 1 WHERE tenant_id = $5 AND id = $6 AND version = $7 AND deleted = FALSE"
                );
                sqlx::query(&sql)
                    .bind(operator)
                    .bind(now)
                    .bind(operator)
                    .bind(now)
                    .bind(tenant_id)
                    .bind(id)
                    .bind(version)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };
        Ok(affected)
    }

    pub async fn physical_delete(
        pool: &DatabasePool,
        table: &str,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<u64> {
        let allowed = matches!(
            table,
            "hrm_users" | "hrm_orgs" | "hrm_posts" | "hrm_user_org_posts"
        );
        if !allowed {
            return Err(AppError::system("HRM 物理删除表名不在白名单内"));
        }
        let affected = match pool {
            DatabasePool::MySql(pool) => {
                let sql = format!(
                    "DELETE FROM {table} WHERE tenant_id = ? AND id = ? AND deleted = TRUE"
                );
                sqlx::query(&sql)
                    .bind(tenant_id)
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                let sql = format!(
                    "DELETE FROM {table} WHERE tenant_id = $1 AND id = $2 AND deleted = TRUE"
                );
                sqlx::query(&sql)
                    .bind(tenant_id)
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };
        Ok(affected)
    }

    pub async fn count_active_children(
        pool: &DatabasePool,
        tenant_id: &str,
        parent_id: &str,
    ) -> AppResult<i64> {
        scalar_count(pool, "SELECT COUNT(*) FROM hrm_orgs WHERE tenant_id = ? AND parent_id = ? AND deleted = FALSE", "SELECT COUNT(*) FROM hrm_orgs WHERE tenant_id = $1 AND parent_id = $2 AND deleted = FALSE", tenant_id, parent_id).await
    }

    pub async fn count_branch_children(
        pool: &DatabasePool,
        tenant_id: &str,
        parent_id: &str,
    ) -> AppResult<i64> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM hrm_orgs WHERE tenant_id = ? AND parent_id = ? AND org_type = 'branch' AND deleted = FALSE").bind(tenant_id).bind(parent_id).fetch_one(pool).await?.0),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM hrm_orgs WHERE tenant_id = $1 AND parent_id = $2 AND org_type = 'branch' AND deleted = FALSE").bind(tenant_id).bind(parent_id).fetch_one(pool).await?.0),
        }
    }

    pub async fn count_relations_by_user(
        pool: &DatabasePool,
        tenant_id: &str,
        user_id: &str,
    ) -> AppResult<i64> {
        scalar_count(pool, "SELECT COUNT(*) FROM hrm_user_org_posts WHERE tenant_id = ? AND user_id = ? AND deleted = FALSE", "SELECT COUNT(*) FROM hrm_user_org_posts WHERE tenant_id = $1 AND user_id = $2 AND deleted = FALSE", tenant_id, user_id).await
    }

    pub async fn count_relations_by_org(
        pool: &DatabasePool,
        tenant_id: &str,
        org_id: &str,
    ) -> AppResult<i64> {
        scalar_count(pool, "SELECT COUNT(*) FROM hrm_user_org_posts WHERE tenant_id = ? AND org_id = ? AND deleted = FALSE", "SELECT COUNT(*) FROM hrm_user_org_posts WHERE tenant_id = $1 AND org_id = $2 AND deleted = FALSE", tenant_id, org_id).await
    }

    pub async fn count_relations_by_post(
        pool: &DatabasePool,
        tenant_id: &str,
        post_id: &str,
    ) -> AppResult<i64> {
        scalar_count(pool, "SELECT COUNT(*) FROM hrm_user_org_posts WHERE tenant_id = ? AND post_id = ? AND deleted = FALSE", "SELECT COUNT(*) FROM hrm_user_org_posts WHERE tenant_id = $1 AND post_id = $2 AND deleted = FALSE", tenant_id, post_id).await
    }

    pub async fn page_users(
        pool: &DatabasePool,
        tenant_id: &str,
        query: &UserPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<HrmUser>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => page_users_db(pool, tenant_id, query, page).await,
            DatabasePool::Postgres(pool) => page_users_pg_db(pool, tenant_id, query, page).await,
        }
    }

    pub async fn page_orgs(
        pool: &DatabasePool,
        tenant_id: &str,
        query: &OrgPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<HrmOrg>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => page_orgs_db(pool, tenant_id, query, page).await,
            DatabasePool::Postgres(pool) => page_orgs_pg_db(pool, tenant_id, query, page).await,
        }
    }

    pub async fn page_posts(
        pool: &DatabasePool,
        tenant_id: &str,
        query: &PostPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<HrmPost>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => page_posts_db(pool, tenant_id, query, page).await,
            DatabasePool::Postgres(pool) => page_posts_pg_db(pool, tenant_id, query, page).await,
        }
    }

    pub async fn page_relations(
        pool: &DatabasePool,
        tenant_id: &str,
        query: &UserOrgPostPageQuery,
        page: NormalizedPageQuery,
    ) -> AppResult<(Vec<HrmUserOrgPost>, u64)> {
        match pool {
            DatabasePool::MySql(pool) => page_relations_db(pool, tenant_id, query, page).await,
            DatabasePool::Postgres(pool) => {
                page_relations_pg_db(pool, tenant_id, query, page).await
            }
        }
    }

    pub async fn list_orgs(pool: &DatabasePool, tenant_id: &str) -> AppResult<Vec<HrmOrg>> {
        match pool {
            DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, HrmOrg>("SELECT * FROM hrm_orgs WHERE tenant_id = ? AND deleted = FALSE ORDER BY sort_no ASC, created_time ASC").bind(tenant_id).fetch_all(pool).await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, HrmOrg>("SELECT * FROM hrm_orgs WHERE tenant_id = $1 AND deleted = FALSE ORDER BY sort_no ASC, created_time ASC").bind(tenant_id).fetch_all(pool).await?),
        }
    }
}

async fn scalar_count(
    pool: &DatabasePool,
    mysql_sql: &str,
    pg_sql: &str,
    tenant_id: &str,
    id: &str,
) -> AppResult<i64> {
    match pool {
        DatabasePool::MySql(pool) => Ok(sqlx::query_as::<_, (i64,)>(mysql_sql)
            .bind(tenant_id)
            .bind(id)
            .fetch_one(pool)
            .await?
            .0),
        DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, (i64,)>(pg_sql)
            .bind(tenant_id)
            .bind(id)
            .fetch_one(pool)
            .await?
            .0),
    }
}

async fn clear_primary_flags_mysql(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    data: &UserOrgPostWrite,
) -> AppResult<()> {
    if data.primary_org {
        sqlx::query("UPDATE hrm_user_org_posts SET primary_org = FALSE, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND user_id = ? AND id <> ? AND deleted = FALSE")
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.tenant_id)
            .bind(&data.user_id)
            .bind(&data.id)
            .execute(&mut **tx)
            .await?;
    }
    if data.primary_post {
        sqlx::query("UPDATE hrm_user_org_posts SET primary_post = FALSE, updated_by = ?, updated_time = ?, version = version + 1 WHERE tenant_id = ? AND user_id = ? AND id <> ? AND deleted = FALSE")
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.tenant_id)
            .bind(&data.user_id)
            .bind(&data.id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn clear_primary_flags_postgres(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    data: &UserOrgPostWrite,
) -> AppResult<()> {
    if data.primary_org {
        sqlx::query("UPDATE hrm_user_org_posts SET primary_org = FALSE, updated_by = $1, updated_time = $2, version = version + 1 WHERE tenant_id = $3 AND user_id = $4 AND id <> $5 AND deleted = FALSE")
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.tenant_id)
            .bind(&data.user_id)
            .bind(&data.id)
            .execute(&mut **tx)
            .await?;
    }
    if data.primary_post {
        sqlx::query("UPDATE hrm_user_org_posts SET primary_post = FALSE, updated_by = $1, updated_time = $2, version = version + 1 WHERE tenant_id = $3 AND user_id = $4 AND id <> $5 AND deleted = FALSE")
            .bind(&data.operator)
            .bind(data.now)
            .bind(&data.tenant_id)
            .bind(&data.user_id)
            .bind(&data.id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn page_users_db(
    pool: &sqlx::MySqlPool,
    tenant_id: &str,
    query: &UserPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmUser>, u64)> {
    let mut count =
        QueryBuilder::<MySql>::new("SELECT COUNT(*) AS total FROM hrm_users WHERE tenant_id = ");
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_user_filters_mysql(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<MySql>::new("SELECT * FROM hrm_users WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_user_filters_mysql(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<HrmUser>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

async fn page_users_pg_db(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    query: &UserPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmUser>, u64)> {
    let mut count =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) AS total FROM hrm_users WHERE tenant_id = ");
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_user_filters_postgres(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<Postgres>::new("SELECT * FROM hrm_users WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_user_filters_postgres(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<HrmUser>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

async fn page_orgs_db(
    pool: &sqlx::MySqlPool,
    tenant_id: &str,
    query: &OrgPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmOrg>, u64)> {
    let mut count =
        QueryBuilder::<MySql>::new("SELECT COUNT(*) AS total FROM hrm_orgs WHERE tenant_id = ");
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_org_filters_mysql(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<MySql>::new("SELECT * FROM hrm_orgs WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_org_filters_mysql(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<HrmOrg>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

async fn page_orgs_pg_db(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    query: &OrgPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmOrg>, u64)> {
    let mut count =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) AS total FROM hrm_orgs WHERE tenant_id = ");
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_org_filters_postgres(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<Postgres>::new("SELECT * FROM hrm_orgs WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_org_filters_postgres(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<HrmOrg>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

async fn page_posts_db(
    pool: &sqlx::MySqlPool,
    tenant_id: &str,
    query: &PostPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmPost>, u64)> {
    let mut count =
        QueryBuilder::<MySql>::new("SELECT COUNT(*) AS total FROM hrm_posts WHERE tenant_id = ");
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_post_filters_mysql(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<MySql>::new("SELECT * FROM hrm_posts WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_post_filters_mysql(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<HrmPost>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

async fn page_posts_pg_db(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    query: &PostPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmPost>, u64)> {
    let mut count =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) AS total FROM hrm_posts WHERE tenant_id = ");
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_post_filters_postgres(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows = QueryBuilder::<Postgres>::new("SELECT * FROM hrm_posts WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_post_filters_postgres(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows.build_query_as::<HrmPost>().fetch_all(pool).await?;
    Ok((records, total as u64))
}

async fn page_relations_db(
    pool: &sqlx::MySqlPool,
    tenant_id: &str,
    query: &UserOrgPostPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmUserOrgPost>, u64)> {
    let mut count = QueryBuilder::<MySql>::new(
        "SELECT COUNT(*) AS total FROM hrm_user_org_posts WHERE tenant_id = ",
    );
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_relation_filters_mysql(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows =
        QueryBuilder::<MySql>::new("SELECT * FROM hrm_user_org_posts WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_relation_filters_mysql(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows
        .build_query_as::<HrmUserOrgPost>()
        .fetch_all(pool)
        .await?;
    Ok((records, total as u64))
}

async fn page_relations_pg_db(
    pool: &sqlx::PgPool,
    tenant_id: &str,
    query: &UserOrgPostPageQuery,
    page: NormalizedPageQuery,
) -> AppResult<(Vec<HrmUserOrgPost>, u64)> {
    let mut count = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*) AS total FROM hrm_user_org_posts WHERE tenant_id = ",
    );
    count
        .push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_relation_filters_postgres(&mut count, query);
    let total = count
        .build()
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("total")?;

    let mut rows =
        QueryBuilder::<Postgres>::new("SELECT * FROM hrm_user_org_posts WHERE tenant_id = ");
    rows.push_bind(tenant_id.to_string())
        .push(" AND deleted = FALSE");
    append_relation_filters_postgres(&mut rows, query);
    rows.push(" ORDER BY sort_no ASC, updated_time DESC, id ASC LIMIT ");
    rows.push_bind(page.page_size as i64);
    rows.push(" OFFSET ");
    rows.push_bind(page.offset as i64);
    let records = rows
        .build_query_as::<HrmUserOrgPost>()
        .fetch_all(pool)
        .await?;
    Ok((records, total as u64))
}

fn append_user_filters_mysql(builder: &mut QueryBuilder<MySql>, query: &UserPageQuery) {
    if let Some(value) = normalized_like(query.employee_no.as_deref()) {
        builder.push(" AND employee_no LIKE ").push_bind(value);
    }
    append_name_filter_mysql(builder, query.name.as_deref());
    if let Some(value) = normalized_like(query.mobile.as_deref()) {
        builder.push(" AND mobile LIKE ").push_bind(value);
    }
    if let Some(value) = normalized_like(query.email.as_deref()) {
        builder.push(" AND email LIKE ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    if let Some(value) = query.sort_no {
        builder.push(" AND sort_no = ").push_bind(value);
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

fn append_user_filters_postgres(builder: &mut QueryBuilder<Postgres>, query: &UserPageQuery) {
    if let Some(value) = normalized_like(query.employee_no.as_deref()) {
        builder.push(" AND employee_no LIKE ").push_bind(value);
    }
    append_name_filter_postgres(builder, query.name.as_deref());
    if let Some(value) = normalized_like(query.mobile.as_deref()) {
        builder.push(" AND mobile LIKE ").push_bind(value);
    }
    if let Some(value) = normalized_like(query.email.as_deref()) {
        builder.push(" AND email LIKE ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    if let Some(value) = query.sort_no {
        builder.push(" AND sort_no = ").push_bind(value);
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

fn append_org_filters_mysql(builder: &mut QueryBuilder<MySql>, query: &OrgPageQuery) {
    if let Some(value) = normalize_non_empty(query.parent_id.as_deref()) {
        builder.push(" AND parent_id = ").push_bind(value);
    }
    if let Some(value) = normalized_like(query.org_code.as_deref()) {
        builder.push(" AND org_code LIKE ").push_bind(value);
    }
    append_name_filter_mysql(builder, query.name.as_deref());
    if let Some(value) = normalize_non_empty(query.org_type.as_deref()) {
        builder.push(" AND org_type = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    append_time_range_mysql(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
    );
}

fn append_org_filters_postgres(builder: &mut QueryBuilder<Postgres>, query: &OrgPageQuery) {
    if let Some(value) = normalize_non_empty(query.parent_id.as_deref()) {
        builder.push(" AND parent_id = ").push_bind(value);
    }
    if let Some(value) = normalized_like(query.org_code.as_deref()) {
        builder.push(" AND org_code LIKE ").push_bind(value);
    }
    append_name_filter_postgres(builder, query.name.as_deref());
    if let Some(value) = normalize_non_empty(query.org_type.as_deref()) {
        builder.push(" AND org_type = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    append_time_range_postgres(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
    );
}

fn append_post_filters_mysql(builder: &mut QueryBuilder<MySql>, query: &PostPageQuery) {
    if let Some(value) = normalized_like(query.post_code.as_deref()) {
        builder.push(" AND post_code LIKE ").push_bind(value);
    }
    append_name_filter_mysql(builder, query.name.as_deref());
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    append_time_range_mysql(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
    );
}

fn append_post_filters_postgres(builder: &mut QueryBuilder<Postgres>, query: &PostPageQuery) {
    if let Some(value) = normalized_like(query.post_code.as_deref()) {
        builder.push(" AND post_code LIKE ").push_bind(value);
    }
    append_name_filter_postgres(builder, query.name.as_deref());
    if let Some(value) = normalize_non_empty(query.status.as_deref()) {
        builder.push(" AND status = ").push_bind(value);
    }
    append_time_range_postgres(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
    );
}

fn append_relation_filters_mysql(builder: &mut QueryBuilder<MySql>, query: &UserOrgPostPageQuery) {
    if let Some(value) = normalize_non_empty(query.user_id.as_deref()) {
        builder.push(" AND user_id = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.org_id.as_deref()) {
        builder.push(" AND org_id = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.post_id.as_deref()) {
        builder.push(" AND post_id = ").push_bind(value);
    }
    if let Some(value) = query.primary_org {
        builder.push(" AND primary_org = ").push_bind(value);
    }
    if let Some(value) = query.primary_post {
        builder.push(" AND primary_post = ").push_bind(value);
    }
    append_time_range_mysql(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
    );
}

fn append_relation_filters_postgres(
    builder: &mut QueryBuilder<Postgres>,
    query: &UserOrgPostPageQuery,
) {
    if let Some(value) = normalize_non_empty(query.user_id.as_deref()) {
        builder.push(" AND user_id = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.org_id.as_deref()) {
        builder.push(" AND org_id = ").push_bind(value);
    }
    if let Some(value) = normalize_non_empty(query.post_id.as_deref()) {
        builder.push(" AND post_id = ").push_bind(value);
    }
    if let Some(value) = query.primary_org {
        builder.push(" AND primary_org = ").push_bind(value);
    }
    if let Some(value) = query.primary_post {
        builder.push(" AND primary_post = ").push_bind(value);
    }
    append_time_range_postgres(
        builder,
        "created_time",
        query.created_time_start,
        query.created_time_end,
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
