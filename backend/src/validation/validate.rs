//! 参数校验辅助。
//!
//! handler 必须显式校验请求 DTO，禁止依赖数据库约束错误表达业务参数问题。

use crate::error::app_error::{AppError, AppResult};

/// 根据布尔条件返回参数校验结果。
///
/// 业务 handler 可以用该函数把显式校验失败稳定转换为 `-400`。
pub fn ensure(condition: bool, message: impl Into<String>) -> AppResult<()> {
    if condition {
        Ok(())
    } else {
        Err(AppError::param_invalid(message))
    }
}

/// 校验字符串字段不能为空白。
///
/// 该函数只处理通用空值约束，业务语义校验应放在对应 service 或 DTO 校验逻辑中。
pub fn ensure_not_blank(value: &str, field_name: &str) -> AppResult<()> {
    ensure(!value.trim().is_empty(), format!("{field_name} 不能为空"))
}
