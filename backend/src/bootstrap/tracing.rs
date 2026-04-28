//! tracing 初始化。
//!
//! 日志初始化必须在数据库连接前完成，确保启动期配置错误、迁移失败和连接失败都能被记录。

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{config::settings::AppConfig, error::app_error::AppResult};

/// 在完整配置加载前初始化日志。
///
/// 启动早期可能因为缺少 `DATABASE_URL`、端口非法等配置问题失败；如果等配置加载成功后
/// 才初始化 tracing，这类错误就只能静默退出。因此这里使用已加载配置中的 `RUST_LOG`，
/// 并用保守默认值建立日志输出通道。
pub fn init_tracing_from_env() -> AppResult<()> {
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    init_tracing_with_filter(&log_level);
    Ok(())
}

/// 根据完整应用配置初始化日志。
///
/// 如果启动早期已经初始化过 tracing，`try_init` 会失败；该失败表示全局 subscriber 已存在，
/// 不影响服务继续运行。
pub fn init_tracing(config: &AppConfig) -> AppResult<()> {
    init_tracing_with_filter(&config.log_level);
    Ok(())
}

fn init_tracing_with_filter(log_level: &str) {
    // 非法 RUST_LOG 不应导致服务启动失败，回退到 info 便于生产环境保守运行。
    let env_filter = EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    // try_init 失败通常意味着测试中已初始化全局 subscriber；这里忽略重复初始化。
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().json())
        .try_init()
        .ok();
}
