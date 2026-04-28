//! 启动配置文件加载。
//!
//! 该模块只负责启动早期的命令行参数解析和 dotenv 文件加载。应用配置结构仍由
//! `config::settings::AppConfig` 统一解析，避免启动参数、环境变量和业务配置分散在多个入口。

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::error::app_error::{AppError, AppResult};

/// 启动期命令行选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupOptions {
    /// 显式指定的 dotenv 配置文件路径；为空时按默认 `.env` 兼容路径加载。
    config_path: Option<PathBuf>,
}

impl StartupOptions {
    /// 返回显式指定的配置文件路径。
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }
}

/// 解析当前进程启动参数并加载对应配置文件。
///
/// 支持 `--config <path>`、`--config=<path>`、`-c <path>` 和 `-c=<path>`。未指定时继续
/// 尝试加载当前工作目录下的默认 `.env`，保持本地开发兼容性。
pub fn load_startup_config() -> AppResult<StartupOptions> {
    let options = parse_startup_options(env::args_os().skip(1))?;
    load_dotenv_file(&options)?;
    Ok(options)
}

/// 从参数列表解析启动选项。
///
/// 该函数不读取文件系统，方便单元测试只验证命令行契约。
pub fn parse_startup_options<I, S>(args: I) -> AppResult<StartupOptions>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut args = args.into_iter().map(Into::into).peekable();
    let mut config_path = None;

    while let Some(arg) = args.next() {
        let arg_text = arg.to_string_lossy();
        match arg_text.as_ref() {
            "--config" | "-c" => {
                let value = args
                    .next()
                    .ok_or_else(|| AppError::param_invalid("启动参数 --config 缺少配置文件路径"))?;
                set_config_path(&mut config_path, value)?;
            }
            value if value.starts_with("--config=") => {
                set_config_path(&mut config_path, value.trim_start_matches("--config="))?;
            }
            value if value.starts_with("-c=") => {
                set_config_path(&mut config_path, value.trim_start_matches("-c="))?;
            }
            "--help" | "-h" => {
                return Err(AppError::param_invalid(
                    "用法: taipus-backend [--config <path>]",
                ));
            }
            value => {
                return Err(AppError::param_invalid(format!(
                    "不支持的启动参数: {value}"
                )));
            }
        }
    }

    Ok(StartupOptions { config_path })
}

/// 依据启动选项加载 dotenv 配置。
fn load_dotenv_file(options: &StartupOptions) -> AppResult<()> {
    match options.config_path() {
        Some(path) => dotenvy::from_path(path).map(|_| ()).map_err(|err| {
            AppError::param_invalid(format!("配置文件加载失败: {}", path.display()))
                .with_internal_message(format!("dotenv 文件加载失败: {err}"))
        }),
        None => {
            // 默认 `.env` 不存在是允许的，容器和生产环境通常直接通过真实环境变量注入配置。
            dotenvy::dotenv().ok();
            Ok(())
        }
    }
}

/// 设置显式配置文件路径，并拒绝重复声明。
fn set_config_path(target: &mut Option<PathBuf>, value: impl Into<OsString>) -> AppResult<()> {
    if target.is_some() {
        return Err(AppError::param_invalid("启动参数 --config 不能重复指定"));
    }
    let value = value.into();
    if value.is_empty() {
        return Err(AppError::param_invalid(
            "启动参数 --config 缺少配置文件路径",
        ));
    }
    *target = Some(PathBuf::from(value));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_startup_options;

    #[test]
    fn parse_startup_options_accepts_config_flag() {
        // 分离形式和等号形式都属于稳定启动契约，发布脚本可按习惯选择其一。
        let separated = parse_startup_options(["--config", "config/local.env"]).unwrap();
        let assigned = parse_startup_options(["-c=config/local.env"]).unwrap();

        assert_eq!(
            separated.config_path().unwrap().to_string_lossy(),
            "config/local.env"
        );
        assert_eq!(
            assigned.config_path().unwrap().to_string_lossy(),
            "config/local.env"
        );
    }

    #[test]
    fn parse_startup_options_rejects_invalid_args() {
        // 未知参数、缺少路径和重复路径都会改变启动语义，必须启动期直接失败。
        assert!(parse_startup_options(["--unknown"]).is_err());
        assert!(parse_startup_options(["--config"]).is_err());
        assert!(parse_startup_options(["--config", "a.env", "-c", "b.env"]).is_err());
    }
}
