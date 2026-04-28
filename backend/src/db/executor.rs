//! 数据库执行器和连接池适配层。
//!
//! 业务模块只能依赖 `DatabasePool`，不得在 handler 或 service 中散落 `MySqlPool` / `PgPool`
//! 判断。确有 SQL 方言差异时，应在 repository 或 dialect adapter 内部封装。

use std::{fmt, time::Duration};

use sqlx::{MySql, Pool, Postgres, mysql::MySqlPoolOptions, postgres::PgPoolOptions};

use crate::{config::settings::DatabaseConfig, error::app_error::AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    /// 默认运行数据库，首版实现和迁移以 MySQL 为主路径。
    MySql,
    /// 兼容数据库，用于保留跨数据库部署能力。
    Postgres,
}

#[derive(Clone)]
pub enum DatabasePool {
    /// MySQL 连接池。业务模块不得直接依赖该变体，应通过统一适配层访问。
    MySql(Pool<MySql>),
    /// PostgreSQL 连接池。方言差异必须收敛在 repository 或适配层内部。
    Postgres(Pool<Postgres>),
}

impl DatabaseType {
    /// 返回稳定的小写方言名称，用于日志、健康检查和配置错误提示。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MySql => "mysql",
            Self::Postgres => "postgres",
        }
    }
}

impl fmt::Display for DatabaseType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl DatabasePool {
    /// 获取当前连接池的数据库方言，健康检查和迁移日志需要使用该信息。
    pub fn database_type(&self) -> DatabaseType {
        match self {
            Self::MySql(_) => DatabaseType::MySql,
            Self::Postgres(_) => DatabaseType::Postgres,
        }
    }

    /// 执行轻量级连通性检查。
    ///
    /// `SELECT 1` 同时兼容 MySQL 和 PostgreSQL，适合作为 `/health/ready` 的基础检查。
    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        match self {
            Self::MySql(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            Self::Postgres(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
        }
        Ok(())
    }
}

pub async fn connect_pool(config: &DatabaseConfig) -> AppResult<DatabasePool> {
    // 最小 1 秒超时可以防止误配置为 0 导致连接获取立即失败，提升本地排错体验。
    let connect_timeout = config.connect_timeout.max(Duration::from_secs(1));

    match config.database_type {
        DatabaseType::MySql => {
            let pool = MySqlPoolOptions::new()
                .min_connections(config.min_connections)
                .max_connections(config.max_connections)
                .acquire_timeout(connect_timeout)
                .connect(&config.database_url)
                .await?;
            Ok(DatabasePool::MySql(pool))
        }
        DatabaseType::Postgres => {
            let pool = PgPoolOptions::new()
                .min_connections(config.min_connections)
                .max_connections(config.max_connections)
                .acquire_timeout(connect_timeout)
                .connect(&config.database_url)
                .await?;
            Ok(DatabasePool::Postgres(pool))
        }
    }
}
