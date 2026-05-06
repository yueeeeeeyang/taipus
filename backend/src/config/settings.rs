//! 配置加载与校验。
//!
//! 配置只在启动期集中读取，业务模块不得直接访问环境变量。这样可以把缺失配置、非法端口、
//! 数据库方言不匹配等问题前置到启动阶段，避免运行过程中才暴露为不稳定错误。

use std::{env, str::FromStr, time::Duration};

use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::{
    db::executor::DatabaseType, error::app_error::AppError,
    i18n::time_zone::canonicalize_time_zone, modules::auth::token::normalize_pem_value,
};

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
    /// 多语言配置，包含默认语言、支持语言和系统资源版本。
    pub i18n: I18nConfig,
    /// 租户配置，控制默认租户和早期联调阶段的 header 租户覆盖能力。
    pub tenant: TenantConfig,
    /// 认证配置，控制 JWT、刷新令牌和默认管理员初始化。
    pub auth: AuthConfig,
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

#[derive(Debug, Clone)]
pub struct I18nConfig {
    /// 配置文件指定的默认语言，业务代码禁止写死默认 locale。
    pub default_locale: String,
    /// 当前系统允许协商和返回的语言列表。
    pub supported_locales: Vec<String>,
    /// 配置文件指定的默认 IANA time zone，业务代码禁止写死默认时区。
    pub default_time_zone: String,
    /// 当前系统允许协商和返回的 IANA time zone 列表。
    pub supported_time_zones: Vec<String>,
    /// 系统多语言资源版本，用于前端和移动端缓存失效。
    pub system_resource_version: String,
}

#[derive(Debug, Clone)]
pub struct TenantConfig {
    /// 默认租户 ID，用于开发、测试和单租户部署。
    pub default_tenant_id: String,
    /// 是否允许通过 X-Tenant-Id 请求头指定租户，首版默认开启便于联调。
    pub allow_header_override: bool,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT 签发方，访问令牌校验必须匹配。
    pub jwt_issuer: String,
    /// JWT 受众，首版 PC 和移动端统一使用同一 API 受众。
    pub jwt_audience: String,
    /// 当前 RS256 密钥 ID，用于后续密钥轮换。
    pub jwt_kid: String,
    /// RS256 私钥 PEM，用于签发访问令牌。
    pub jwt_private_key_pem: Option<String>,
    /// RS256 公钥 PEM，用于校验访问令牌。
    pub jwt_public_key_pem: Option<String>,
    /// 访问令牌有效期秒数。
    pub access_token_ttl_seconds: i64,
    /// 刷新令牌有效期秒数。
    pub refresh_token_ttl_seconds: i64,
    /// 刷新令牌哈希 pepper，必须由部署环境提供。
    pub refresh_token_pepper: Option<String>,
    /// 首次启动管理员用户名。
    pub bootstrap_admin_username: Option<String>,
    /// 首次启动管理员明文密码，仅启动初始化使用，不写入日志和响应。
    pub bootstrap_admin_password: Option<String>,
    /// 首次启动管理员展示名。
    pub bootstrap_admin_display_name: Option<String>,
    /// 首次启动管理员所属租户。
    pub bootstrap_admin_tenant_id: Option<String>,
}

impl AppConfig {
    /// 从默认 `.env` 和环境变量加载配置，并立即执行完整校验。
    ///
    /// 该入口用于测试、工具或未经过 `main.rs` 启动流程的调用方；正式进程启动会先由
    /// `bootstrap::config` 处理显式配置文件，然后调用 `from_loaded_env` 避免重复加载默认 `.env`。
    pub fn from_env() -> Result<Self, AppError> {
        dotenvy::dotenv().ok();
        Self::from_loaded_env()
    }

    /// 从已经加载到进程环境中的配置项构造应用配置。
    ///
    /// 启动配置必须前置失败，不能把缺失数据库连接串、方言不匹配等问题延迟到请求处理阶段。
    pub fn from_loaded_env() -> Result<Self, AppError> {
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
        // 默认语言必须来自配置，避免业务代码或接口逻辑写死某个 locale。
        let default_locale = read_env("I18N_DEFAULT_LOCALE")
            .ok_or_else(|| AppError::param_invalid("I18N_DEFAULT_LOCALE 是必填配置"))?;
        let supported_locales =
            parse_locale_list(read_env("I18N_SUPPORTED_LOCALES"), &default_locale)?;
        // 默认时区同样必须来自配置，保证跨地区部署和用户展示行为可控。
        let default_time_zone = read_env("I18N_DEFAULT_TIME_ZONE")
            .ok_or_else(|| AppError::param_invalid("I18N_DEFAULT_TIME_ZONE 是必填配置"))?;
        let supported_time_zones =
            parse_time_zone_list(read_env("I18N_SUPPORTED_TIME_ZONES"), &default_time_zone)?;
        let system_resource_version =
            read_env("I18N_SYSTEM_RESOURCE_VERSION").unwrap_or_else(|| "202604280001".to_string());
        let tenant = TenantConfig {
            default_tenant_id: read_env("TENANT_DEFAULT_ID")
                .unwrap_or_else(|| "default".to_string()),
            allow_header_override: parse_bool_env(
                "TENANT_ALLOW_HEADER_OVERRIDE",
                read_env("TENANT_ALLOW_HEADER_OVERRIDE"),
                true,
            )?,
        };
        let auth = AuthConfig {
            jwt_issuer: read_env("AUTH_JWT_ISSUER").unwrap_or_else(|| "taipus-api".to_string()),
            jwt_audience: read_env("AUTH_JWT_AUDIENCE").unwrap_or_else(|| "api".to_string()),
            jwt_kid: read_env("AUTH_JWT_KID").unwrap_or_else(|| "default".to_string()),
            jwt_private_key_pem: read_env("AUTH_JWT_PRIVATE_KEY_PEM"),
            jwt_public_key_pem: read_env("AUTH_JWT_PUBLIC_KEY_PEM"),
            access_token_ttl_seconds: read_env("AUTH_ACCESS_TOKEN_TTL_SECONDS")
                .unwrap_or_else(|| "900".to_string())
                .parse::<i64>()
                .map_err(|_| {
                    AppError::param_invalid("AUTH_ACCESS_TOKEN_TTL_SECONDS 必须是正整数")
                })?,
            refresh_token_ttl_seconds: read_env("AUTH_REFRESH_TOKEN_TTL_SECONDS")
                .unwrap_or_else(|| "2592000".to_string())
                .parse::<i64>()
                .map_err(|_| {
                    AppError::param_invalid("AUTH_REFRESH_TOKEN_TTL_SECONDS 必须是正整数")
                })?,
            refresh_token_pepper: read_env("AUTH_REFRESH_TOKEN_PEPPER"),
            bootstrap_admin_username: read_env("AUTH_BOOTSTRAP_ADMIN_USERNAME"),
            bootstrap_admin_password: read_env("AUTH_BOOTSTRAP_ADMIN_PASSWORD"),
            bootstrap_admin_display_name: read_env("AUTH_BOOTSTRAP_ADMIN_DISPLAY_NAME"),
            bootstrap_admin_tenant_id: read_env("AUTH_BOOTSTRAP_ADMIN_TENANT_ID"),
        };
        let mut i18n = I18nConfig {
            default_locale,
            supported_locales,
            default_time_zone,
            supported_time_zones,
            system_resource_version,
        };
        canonicalize_i18n_config(&mut i18n)?;

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
            i18n,
            tenant,
            auth,
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
            i18n: I18nConfig {
                default_locale: "zh-CN".to_string(),
                supported_locales: vec!["zh-CN".to_string(), "en-US".to_string()],
                default_time_zone: "Asia/Shanghai".to_string(),
                supported_time_zones: vec![
                    "Asia/Shanghai".to_string(),
                    "UTC".to_string(),
                    "America/New_York".to_string(),
                ],
                system_resource_version: "test-version".to_string(),
            },
            tenant: TenantConfig {
                default_tenant_id: "default".to_string(),
                allow_header_override: true,
            },
            auth: AuthConfig {
                jwt_issuer: "taipus-api".to_string(),
                jwt_audience: "api".to_string(),
                jwt_kid: "test".to_string(),
                jwt_private_key_pem: None,
                jwt_public_key_pem: None,
                access_token_ttl_seconds: 900,
                refresh_token_ttl_seconds: 2_592_000,
                refresh_token_pepper: Some("test-pepper".to_string()),
                bootstrap_admin_username: None,
                bootstrap_admin_password: None,
                bootstrap_admin_display_name: None,
                bootstrap_admin_tenant_id: None,
            },
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
        )?;
        validate_i18n_config(&self.i18n)?;
        validate_tenant_config(&self.tenant)?;
        validate_auth_config(&self.auth)
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

/// 解析逗号分隔的 locale 列表。
///
/// 未配置支持语言时，只启用配置默认语言，避免隐式把某个固定语言加入可用范围。
fn parse_locale_list(value: Option<String>, default_locale: &str) -> Result<Vec<String>, AppError> {
    let locales = value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|locale| !locale.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|locales| !locales.is_empty())
        .unwrap_or_else(|| vec![default_locale.to_string()]);

    for locale in &locales {
        if !is_valid_locale_tag(locale) {
            return Err(AppError::param_invalid(format!(
                "I18N_SUPPORTED_LOCALES 包含非法 locale: {locale}"
            )));
        }
    }

    Ok(locales)
}

/// 解析逗号分隔的 time zone 列表。
///
/// 未配置支持时区时，只启用配置默认时区，避免隐式把某个固定地区加入可用范围。
fn parse_time_zone_list(
    value: Option<String>,
    default_time_zone: &str,
) -> Result<Vec<String>, AppError> {
    let time_zones = value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|time_zone| !time_zone.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|time_zones| !time_zones.is_empty())
        .unwrap_or_else(|| vec![default_time_zone.to_string()]);

    for time_zone in &time_zones {
        canonicalize_time_zone(time_zone)?;
    }

    Ok(time_zones)
}

fn validate_i18n_config(config: &I18nConfig) -> Result<(), AppError> {
    if !is_valid_locale_tag(&config.default_locale) {
        return Err(AppError::param_invalid(
            "I18N_DEFAULT_LOCALE 必须是合法 locale",
        ));
    }
    let available_locales = available_resource_locales();
    validate_locale_has_resource(
        "I18N_DEFAULT_LOCALE",
        &config.default_locale,
        &available_locales,
    )?;
    for locale in &config.supported_locales {
        if !is_valid_locale_tag(locale) {
            return Err(AppError::param_invalid(format!(
                "I18N_SUPPORTED_LOCALES 包含非法 locale: {locale}"
            )));
        }
        validate_locale_has_resource("I18N_SUPPORTED_LOCALES", locale, &available_locales)?;
    }
    if !config
        .supported_locales
        .iter()
        .any(|locale| locale == &config.default_locale)
    {
        return Err(AppError::param_invalid(
            "I18N_SUPPORTED_LOCALES 必须包含 I18N_DEFAULT_LOCALE",
        ));
    }
    canonicalize_time_zone(&config.default_time_zone)?;
    for time_zone in &config.supported_time_zones {
        canonicalize_time_zone(time_zone)?;
    }
    if !config
        .supported_time_zones
        .iter()
        .any(|time_zone| time_zone == &config.default_time_zone)
    {
        return Err(AppError::param_invalid(
            "I18N_SUPPORTED_TIME_ZONES 必须包含 I18N_DEFAULT_TIME_ZONE",
        ));
    }
    if config.system_resource_version.trim().is_empty() {
        return Err(AppError::param_invalid(
            "I18N_SYSTEM_RESOURCE_VERSION 不能为空",
        ));
    }
    Ok(())
}

fn validate_tenant_config(config: &TenantConfig) -> Result<(), AppError> {
    if config.default_tenant_id.trim().is_empty() {
        return Err(AppError::param_invalid("TENANT_DEFAULT_ID 不能为空"));
    }
    if !is_safe_identifier(config.default_tenant_id.trim()) {
        return Err(AppError::param_invalid(
            "TENANT_DEFAULT_ID 只能包含字母、数字、下划线和中横线",
        ));
    }
    Ok(())
}

fn validate_auth_config(config: &AuthConfig) -> Result<(), AppError> {
    if config.jwt_issuer.trim().is_empty() {
        return Err(AppError::param_invalid("AUTH_JWT_ISSUER 不能为空"));
    }
    if config.jwt_audience.trim().is_empty() {
        return Err(AppError::param_invalid("AUTH_JWT_AUDIENCE 不能为空"));
    }
    if config.jwt_kid.trim().is_empty() {
        return Err(AppError::param_invalid("AUTH_JWT_KID 不能为空"));
    }
    if config.access_token_ttl_seconds <= 0 {
        return Err(AppError::param_invalid(
            "AUTH_ACCESS_TOKEN_TTL_SECONDS 必须大于 0",
        ));
    }
    if config.refresh_token_ttl_seconds <= 0 {
        return Err(AppError::param_invalid(
            "AUTH_REFRESH_TOKEN_TTL_SECONDS 必须大于 0",
        ));
    }
    if config
        .jwt_private_key_pem
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AppError::param_invalid(
            "AUTH_JWT_PRIVATE_KEY_PEM 是认证模块必填配置",
        ));
    }
    if let Some(private_key) = config.jwt_private_key_pem.as_deref() {
        // JWT 密钥属于认证链路硬依赖，必须在启动期解析失败，而不是等到登录签发令牌时返回系统错误。
        EncodingKey::from_rsa_pem(normalize_pem_value(private_key).as_bytes()).map_err(|err| {
            AppError::param_invalid(format!("AUTH_JWT_PRIVATE_KEY_PEM 不是合法 RSA 私钥: {err}"))
        })?;
    }
    if config
        .jwt_public_key_pem
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AppError::param_invalid(
            "AUTH_JWT_PUBLIC_KEY_PEM 是认证模块必填配置",
        ));
    }
    if let Some(public_key) = config.jwt_public_key_pem.as_deref() {
        // 公钥同样前置校验，避免服务启动后才在受保护接口鉴权时暴露配置错误。
        DecodingKey::from_rsa_pem(normalize_pem_value(public_key).as_bytes()).map_err(|err| {
            AppError::param_invalid(format!("AUTH_JWT_PUBLIC_KEY_PEM 不是合法 RSA 公钥: {err}"))
        })?;
    }
    if config
        .refresh_token_pepper
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AppError::param_invalid(
            "AUTH_REFRESH_TOKEN_PEPPER 是认证模块必填配置",
        ));
    }
    let bootstrap_values = [
        config.bootstrap_admin_username.as_ref(),
        config.bootstrap_admin_password.as_ref(),
        config.bootstrap_admin_display_name.as_ref(),
        config.bootstrap_admin_tenant_id.as_ref(),
    ];
    let configured_count = bootstrap_values
        .iter()
        .filter(|value| value.is_some_and(|value| !value.trim().is_empty()))
        .count();
    if configured_count != 0 && configured_count != bootstrap_values.len() {
        return Err(AppError::param_invalid(
            "AUTH_BOOTSTRAP_ADMIN_* 必须同时配置 username、password、display name 和 tenant id",
        ));
    }
    if let Some(tenant_id) = config.bootstrap_admin_tenant_id.as_ref() {
        if !is_safe_identifier(tenant_id.trim()) {
            return Err(AppError::param_invalid(
                "AUTH_BOOTSTRAP_ADMIN_TENANT_ID 只能包含字母、数字、下划线和中横线",
            ));
        }
    }
    Ok(())
}

fn canonicalize_i18n_config(config: &mut I18nConfig) -> Result<(), AppError> {
    let available_locales = available_resource_locales();
    config.default_locale = canonicalize_resource_locale(
        "I18N_DEFAULT_LOCALE",
        &config.default_locale,
        &available_locales,
    )?;

    let mut canonical_supported_locales = Vec::new();
    for locale in &config.supported_locales {
        let canonical =
            canonicalize_resource_locale("I18N_SUPPORTED_LOCALES", locale, &available_locales)?;
        if !canonical_supported_locales.contains(&canonical) {
            canonical_supported_locales.push(canonical);
        }
    }
    config.supported_locales = canonical_supported_locales;

    config.default_time_zone = canonicalize_time_zone(&config.default_time_zone)?;
    let mut canonical_supported_time_zones = Vec::new();
    for time_zone in &config.supported_time_zones {
        let canonical = canonicalize_time_zone(time_zone)?;
        if !canonical_supported_time_zones.contains(&canonical) {
            canonical_supported_time_zones.push(canonical);
        }
    }
    config.supported_time_zones = canonical_supported_time_zones;
    Ok(())
}

fn available_resource_locales() -> Vec<String> {
    rust_i18n::available_locales!()
        .into_iter()
        .map(|locale| locale.into_owned())
        .collect()
}

fn canonicalize_resource_locale(
    field_name: &str,
    locale: &str,
    available_locales: &[String],
) -> Result<String, AppError> {
    if !is_valid_locale_tag(locale) {
        return Err(AppError::param_invalid(format!(
            "{field_name} 包含非法 locale: {locale}"
        )));
    }
    available_locales
        .iter()
        .find(|available_locale| available_locale.eq_ignore_ascii_case(locale))
        .cloned()
        .ok_or_else(|| {
            AppError::param_invalid(format!(
                "{field_name}={locale} 未找到对应系统多语言资源文件"
            ))
        })
}

fn validate_locale_has_resource(
    field_name: &str,
    locale: &str,
    available_locales: &[String],
) -> Result<(), AppError> {
    let exists = available_locales
        .iter()
        .any(|available_locale| available_locale == locale);
    if exists {
        Ok(())
    } else {
        Err(AppError::param_invalid(format!(
            "{field_name}={locale} 未找到对应系统多语言资源文件"
        )))
    }
}

fn is_valid_locale_tag(value: &str) -> bool {
    let len = value.len();
    if !(2..=35).contains(&len) {
        return false;
    }

    value
        .split('-')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphanumeric()))
}

fn is_safe_identifier(value: &str) -> bool {
    let len = value.len();
    (1..=64).contains(&len)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
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
    use super::{
        I18nConfig, TenantConfig, canonicalize_i18n_config, parse_bool_env, parse_locale_list,
        parse_time_zone_list, validate_i18n_config, validate_tenant_config,
    };

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

    #[test]
    fn parse_locale_list_defaults_to_config_default_locale() {
        // 未显式配置支持语言时，只启用默认语言，保证默认语言来源仍然是配置项。
        let locales = parse_locale_list(None, "en-US").unwrap();
        assert_eq!(locales, vec!["en-US"]);
    }

    #[test]
    fn parse_locale_list_rejects_invalid_locale() {
        // locale 会进入响应头和缓存 key，非法字符必须在启动期拦截。
        assert!(parse_locale_list(Some("zh-CN,bad locale".to_string()), "zh-CN").is_err());
    }

    #[test]
    fn parse_time_zone_list_defaults_to_config_default_time_zone() {
        // 未显式配置支持时区时，只启用默认时区，保证默认时区来源仍然是配置项。
        let time_zones = parse_time_zone_list(None, "Asia/Shanghai").unwrap();
        assert_eq!(time_zones, vec!["Asia/Shanghai"]);
    }

    #[test]
    fn parse_time_zone_list_rejects_invalid_time_zone() {
        // time zone 会进入响应头和前端缓存 key，非法值必须在启动期拦截。
        assert!(
            parse_time_zone_list(Some("Asia/Shanghai,Bad/Zone".to_string()), "Asia/Shanghai")
                .is_err()
        );
    }

    #[test]
    fn validate_tenant_config_rejects_invalid_default_tenant() {
        // 默认租户会进入请求上下文和 SQL 条件，必须限制为稳定安全标识符。
        let config = TenantConfig {
            default_tenant_id: "bad tenant".to_string(),
            allow_header_override: true,
        };

        assert!(validate_tenant_config(&config).is_err());
    }

    #[test]
    fn validate_i18n_config_accepts_loaded_resource_locales() {
        // 配置语言必须和已加载资源文件一致，合法资源语言应正常通过启动校验。
        let config = I18nConfig {
            default_locale: "zh-CN".to_string(),
            supported_locales: vec!["zh-CN".to_string(), "en-US".to_string()],
            default_time_zone: "Asia/Shanghai".to_string(),
            supported_time_zones: vec!["Asia/Shanghai".to_string(), "UTC".to_string()],
            system_resource_version: "test-version".to_string(),
        };

        assert!(validate_i18n_config(&config).is_ok());
    }

    #[test]
    fn validate_i18n_config_rejects_locale_without_resource_file() {
        // 只声明配置但没有对应 yml 文件时必须启动失败，避免返回伪支持语言。
        let config = I18nConfig {
            default_locale: "fr-FR".to_string(),
            supported_locales: vec!["fr-FR".to_string()],
            default_time_zone: "Asia/Shanghai".to_string(),
            supported_time_zones: vec!["Asia/Shanghai".to_string()],
            system_resource_version: "test-version".to_string(),
        };

        assert!(validate_i18n_config(&config).is_err());
    }

    #[test]
    fn validate_i18n_config_rejects_time_zone_without_default() {
        // 支持时区列表必须包含默认时区，否则 fallback 结果可能不是允许返回值。
        let config = I18nConfig {
            default_locale: "zh-CN".to_string(),
            supported_locales: vec!["zh-CN".to_string()],
            default_time_zone: "Asia/Shanghai".to_string(),
            supported_time_zones: vec!["UTC".to_string()],
            system_resource_version: "test-version".to_string(),
        };

        assert!(validate_i18n_config(&config).is_err());
    }

    #[test]
    fn canonicalize_i18n_config_rewrites_locale_case_to_resource_locale() {
        // 环境变量常见大小写差异必须收敛为资源文件中的 canonical locale，避免 rust-i18n 查找失败。
        let mut config = I18nConfig {
            default_locale: "en-us".to_string(),
            supported_locales: vec!["zh-cn".to_string(), "en-us".to_string()],
            default_time_zone: "Asia/Shanghai".to_string(),
            supported_time_zones: vec!["Asia/Shanghai".to_string(), "UTC".to_string()],
            system_resource_version: "test-version".to_string(),
        };

        canonicalize_i18n_config(&mut config).unwrap();

        assert_eq!(config.default_locale, "en-US");
        assert_eq!(config.supported_locales, vec!["zh-CN", "en-US"]);
        assert!(validate_i18n_config(&config).is_ok());
    }

    #[test]
    fn validate_i18n_config_rejects_non_canonical_locale_case() {
        // 绕过环境变量解析直接构造配置时，非资源文件精确写法必须被拒绝。
        let config = I18nConfig {
            default_locale: "en-us".to_string(),
            supported_locales: vec!["en-us".to_string()],
            default_time_zone: "Asia/Shanghai".to_string(),
            supported_time_zones: vec!["Asia/Shanghai".to_string()],
            system_resource_version: "test-version".to_string(),
        };

        assert!(validate_i18n_config(&config).is_err());
    }

    #[test]
    fn canonicalize_i18n_config_deduplicates_time_zones() {
        // 时区列表规范化后要去重，避免响应缓存 key 和配置展示出现重复项。
        let mut config = I18nConfig {
            default_locale: "zh-CN".to_string(),
            supported_locales: vec!["zh-CN".to_string()],
            default_time_zone: "Asia/Shanghai".to_string(),
            supported_time_zones: vec!["Asia/Shanghai".to_string(), "Asia/Shanghai".to_string()],
            system_resource_version: "test-version".to_string(),
        };

        canonicalize_i18n_config(&mut config).unwrap();

        assert_eq!(config.supported_time_zones, vec!["Asia/Shanghai"]);
        assert!(validate_i18n_config(&config).is_ok());
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
