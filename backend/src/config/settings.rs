//! 配置加载与校验。
//!
//! 配置只在启动期集中读取，业务模块不得直接访问环境变量。这样可以把缺失配置、非法端口、
//! 数据库方言不匹配等问题前置到启动阶段，避免运行过程中才暴露为不稳定错误。

use std::{env, str::FromStr, time::Duration};

use crate::{db::executor::DatabaseType, error::app_error::AppError};

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 应用运行环境，用于区分本地、测试、预发和生产等部署上下文。
    pub app_env: String,
    /// HTTP 服务监听配置，启动阶段必须完成合法性校验。
    pub server: ServerConfig,
    /// 数据库连接、连接池和迁移配置，是后端服务可用性的关键依赖。
    pub database: DatabaseConfig,
    /// tracing 日志过滤级别，允许通过环境变量控制不同环境的日志噪声。
    pub log_level: String,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 服务监听地址，容器部署通常使用 `0.0.0.0`，本地测试可使用 `127.0.0.1`。
    pub host: String,
    /// 服务监听端口，启动期解析失败必须直接终止，避免进程处于半启动状态。
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// 当前运行数据库方言，默认 MySQL，同时保留 PostgreSQL 兼容边界。
    pub database_type: DatabaseType,
    /// 数据库连接串。该值可能包含敏感信息，禁止写入接口响应。
    pub database_url: String,
    /// 连接池最大连接数，用于限制数据库连接资源占用。
    pub max_connections: u32,
    /// 连接池最小连接数，用于控制服务启动后的预热连接规模。
    pub min_connections: u32,
    /// 获取连接的超时时间，避免依赖不可用时请求无限等待。
    pub connect_timeout: Duration,
    /// 是否在服务启动时执行 Refinery migration，生产默认开启以保证结构一致性。
    pub run_migrations: bool,
}

impl AppConfig {
    /// 从环境变量加载配置，并立即执行完整校验。
    ///
    /// 启动配置必须前置失败，不能把缺失数据库连接串、方言不匹配等问题延迟到请求处理阶段。
    pub fn from_env() -> Result<Self, AppError> {
        dotenvy::dotenv().ok();

        let app_env = read_env("APP_ENV").unwrap_or_else(|| "local".to_string());
        let host = read_env("SERVER_HOST").unwrap_or_else(|| "0.0.0.0".to_string());
        let port = read_env("SERVER_PORT")
            .unwrap_or_else(|| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| AppError::param_invalid("SERVER_PORT 必须是合法端口号"))?;

        let database_type = read_env("DATABASE_TYPE")
            .unwrap_or_else(|| "mysql".to_string())
            .parse::<DatabaseType>()?;
        // 数据库是后端服务的强依赖，启动路径必须显式要求连接串存在。
        let database_url = read_env("DATABASE_URL")
            .ok_or_else(|| AppError::param_invalid("DATABASE_URL 是启动后端服务的必填配置"))?;

        let max_connections = read_env("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|| "20".to_string())
            .parse::<u32>()
            .map_err(|_| AppError::param_invalid("DATABASE_MAX_CONNECTIONS 必须是正整数"))?;
        let min_connections = read_env("DATABASE_MIN_CONNECTIONS")
            .unwrap_or_else(|| "1".to_string())
            .parse::<u32>()
            .map_err(|_| AppError::param_invalid("DATABASE_MIN_CONNECTIONS 必须是正整数"))?;
        let connect_timeout_secs = read_env("DATABASE_CONNECT_TIMEOUT_SECONDS")
            .unwrap_or_else(|| "5".to_string())
            .parse::<u64>()
            .map_err(|_| {
                AppError::param_invalid("DATABASE_CONNECT_TIMEOUT_SECONDS 必须是正整数")
            })?;
        // 迁移默认开启，只有测试、临时诊断或受控发布流程才应显式关闭。
        let run_migrations = parse_bool_env(
            "DATABASE_RUN_MIGRATIONS",
            read_env("DATABASE_RUN_MIGRATIONS"),
            true,
        )?;

        let config = Self {
            app_env,
            server: ServerConfig { host, port },
            database: DatabaseConfig {
                database_type,
                database_url,
                max_connections,
                min_connections,
                connect_timeout: Duration::from_secs(connect_timeout_secs),
                run_migrations,
            },
            log_level: read_env("RUST_LOG").unwrap_or_else(|| "info".to_string()),
        };
        config.validate()?;
        Ok(config)
    }

    /// 测试配置不携带数据库连接串，用于验证路由、中间件和统一响应。
    /// 生产启动不得使用该构造函数。
    pub fn for_test() -> Self {
        Self {
            app_env: "test".to_string(),
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
            database: DatabaseConfig {
                database_type: DatabaseType::MySql,
                database_url: "mysql://root:root@127.0.0.1:3306/taipus_test".to_string(),
                max_connections: 1,
                min_connections: 0,
                connect_timeout: Duration::from_secs(1),
                run_migrations: false,
            },
            log_level: "debug".to_string(),
        }
    }

    /// 校验跨字段约束。
    ///
    /// 单字段解析只能发现类型错误；连接池大小、数据库方言和连接串协议必须在这里统一校验。
    pub fn validate(&self) -> Result<(), AppError> {
        if self.server.host.trim().is_empty() {
            return Err(AppError::param_invalid("SERVER_HOST 不能为空"));
        }
        if self.database.database_url.trim().is_empty() {
            return Err(AppError::param_invalid("DATABASE_URL 不能为空"));
        }
        if self.database.max_connections == 0 {
            return Err(AppError::param_invalid(
                "DATABASE_MAX_CONNECTIONS 必须大于 0",
            ));
        }
        if self.database.min_connections > self.database.max_connections {
            return Err(AppError::param_invalid(
                "DATABASE_MIN_CONNECTIONS 不得大于 DATABASE_MAX_CONNECTIONS",
            ));
        }
        validate_url_matches_database_type(
            &self.database.database_type,
            &self.database.database_url,
        )
    }
}

fn read_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

/// 严格解析布尔环境变量。
///
/// 非法值必须在启动期失败，避免生产环境因大小写或拼写错误静默关闭 migration 等关键能力。
fn parse_bool_env(key: &str, value: Option<String>, default: bool) -> Result<bool, AppError> {
    match value {
        None => Ok(default),
        Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(AppError::param_invalid(format!(
                "{key} 必须是布尔值 true/false/1/0/yes/no/on/off"
            ))),
        },
    }
}

/// 校验连接串协议和配置的数据库类型一致。
///
/// 该检查可以提前发现 `DATABASE_TYPE=mysql` 但传入 PostgreSQL URL 的配置错误，
/// 避免 SQLx 在连接阶段才返回较难定位的驱动错误。
fn validate_url_matches_database_type(
    database_type: &DatabaseType,
    database_url: &str,
) -> Result<(), AppError> {
    let lower = database_url.to_ascii_lowercase();
    let matched = match database_type {
        DatabaseType::MySql => lower.starts_with("mysql://"),
        DatabaseType::Postgres => {
            lower.starts_with("postgres://") || lower.starts_with("postgresql://")
        }
    };

    if matched {
        Ok(())
    } else {
        Err(AppError::param_invalid(format!(
            "DATABASE_URL 与 DATABASE_TYPE={} 不匹配",
            database_type.as_str()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_bool_env;

    #[test]
    fn parse_bool_env_accepts_case_insensitive_true_values() {
        // 迁移开关来自部署系统，解析必须兼容常见大小写和布尔别名。
        assert!(
            parse_bool_env("DATABASE_RUN_MIGRATIONS", Some("TRUE".to_string()), false).unwrap()
        );
        assert!(
            parse_bool_env("DATABASE_RUN_MIGRATIONS", Some(" yes ".to_string()), false).unwrap()
        );
        assert!(parse_bool_env("DATABASE_RUN_MIGRATIONS", None, true).unwrap());
    }

    #[test]
    fn parse_bool_env_accepts_false_values_and_rejects_invalid_values() {
        // 非法值必须直接失败，避免关键启动行为被静默改写。
        assert!(!parse_bool_env("DATABASE_RUN_MIGRATIONS", Some("OFF".to_string()), true).unwrap());
        assert!(!parse_bool_env("DATABASE_RUN_MIGRATIONS", Some("0".to_string()), true).unwrap());
        assert!(
            parse_bool_env("DATABASE_RUN_MIGRATIONS", Some("maybe".to_string()), true).is_err()
        );
    }
}

impl FromStr for DatabaseType {
    type Err = AppError;

    /// 支持常见 PostgreSQL 别名，减少环境变量配置差异导致的启动失败。
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "mysql" => Ok(DatabaseType::MySql),
            "postgres" | "postgresql" | "pg" => Ok(DatabaseType::Postgres),
            _ => Err(AppError::param_invalid(
                "DATABASE_TYPE 仅支持 mysql 或 postgres",
            )),
        }
    }
}
