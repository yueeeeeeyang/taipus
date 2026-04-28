//! 后端服务启动入口。
//!
//! `main.rs` 只编排启动顺序，不承载业务逻辑。这样可以保证配置、日志、数据库、迁移和路由装配
//! 都有明确的失败边界，便于后续接入部署探针和启动期观测。

use std::{net::SocketAddr, time::Instant};

use taipus_backend::{
    AppConfig, AppError, AppState,
    bootstrap::{
        config::load_startup_config,
        database::create_database_pool,
        migration::run_migrations,
        tracing::{init_tracing, init_tracing_from_env},
    },
    build_router,
};
use tokio::net::TcpListener;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        // 启动失败必须立即退出，避免服务在缺少数据库、迁移失败或配置错误时继续接流量。
        log_startup_error(err.as_ref());
        std::process::exit(1);
    }
}

/// 按固定顺序启动后端服务。
///
/// 该流程与文档保持一致：配置、日志、数据库、迁移、状态、路由和监听必须顺序执行。
async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let startup_started_at = Instant::now();
    let startup_options = load_startup_config()?;
    init_tracing_from_env()?;
    if let Some(config_path) = startup_options.config_path() {
        info!(config_file = %config_path.display(), "已加载启动配置文件");
    }
    let config = AppConfig::from_loaded_env()?;
    init_tracing(&config)?;

    let database = create_database_pool(&config.database).await?;
    if config.database.run_migrations {
        // migration 必须在服务监听前完成，避免新实例使用旧表结构处理请求。
        run_migrations(&config.database).await?;
    }

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    let app = build_router(AppState::new(config, Some(database)));
    let startup_elapsed_ms = startup_started_at.elapsed().as_millis();

    info!(%addr, startup_elapsed_ms, "后端服务开始监听");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// 输出启动失败原因。
///
/// 这里同时写 tracing 和 stderr：tracing 便于结构化日志采集，stderr 兜底保证即使日志系统异常，
/// `cargo run`、容器日志或进程管理器也能看到失败原因。
fn log_startup_error(err: &(dyn std::error::Error + Send + Sync + 'static)) {
    if let Some(app_error) = err.downcast_ref::<AppError>() {
        let internal_message = app_error
            .internal_message
            .as_deref()
            .unwrap_or(app_error.message.as_str());
        error!(
            code = app_error.code.as_i32(),
            public_message = %app_error.message,
            internal_message,
            alert = app_error.alert,
            "后端服务启动失败"
        );
        eprintln!(
            "后端服务启动失败: {}（业务码: {}）",
            app_error.message,
            app_error.code.as_i32()
        );
        if internal_message != app_error.message {
            eprintln!("内部原因: {internal_message}");
        }
    } else {
        error!(error = %err, "后端服务启动失败");
        eprintln!("后端服务启动失败: {err}");
    }
}

/// 监听进程退出信号。
///
/// Ctrl+C 服务本地开发，terminate 服务容器和进程管理器的优雅停止。
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("监听 Ctrl+C 信号失败");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("监听 terminate 信号失败")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
