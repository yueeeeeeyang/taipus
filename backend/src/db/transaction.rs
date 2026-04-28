//! 事务边界约定。
//!
//! 具体业务模块落地后，应在 service 层编排事务，并把事务执行器传入 repository。
//! 本文件先保留统一入口，避免未来每个模块直接操作连接池导致事务边界不可审计。

use crate::error::app_error::AppResult;

pub async fn run_in_transaction<F, Fut, T>(operation: F) -> AppResult<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = AppResult<T>>,
{
    // 当前底座尚未绑定具体业务事务类型，先保留统一调用形态，后续可替换为真实 SQLx 事务。
    operation().await
}
