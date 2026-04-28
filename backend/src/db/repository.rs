//! repository 通用写入结果约定。
//!
//! 本模块只封装跨业务一致的行数判断和错误映射，不封装通用 CRUD，也不动态拼接任意表名。
//! 业务 repository 仍应显式编写 SQL，保证 SQLx 查询、索引和方言差异都能被清晰审查。

use crate::error::{
    app_error::{AppError, AppResult},
    error_code::ErrorCode,
};

/// 新增操作必须影响的行数。
pub const EXPECTED_SINGLE_ROW_AFFECTED: u64 = 1;

/// 校验新增操作影响行数。
///
/// 插入路径正常只影响 1 行；如果不是 1 行，通常表示 SQL 模板或数据库行为不符合 repository 契约。
pub fn ensure_inserted(rows_affected: u64) -> AppResult<()> {
    ensure_exactly_one_row(rows_affected, "新增")
}

/// 校验更新操作影响行数。
///
/// 更新 0 行通常表示版本号不匹配、资源已被逻辑删除或资源不存在，统一映射为并发冲突。
pub fn ensure_updated(rows_affected: u64) -> AppResult<()> {
    ensure_mutated_one_row_or_conflict(rows_affected, "更新")
}

/// 校验逻辑删除操作影响行数。
///
/// 逻辑删除 0 行通常表示版本号不匹配、资源已删除或资源不存在，统一映射为并发冲突。
pub fn ensure_deleted(rows_affected: u64) -> AppResult<()> {
    ensure_mutated_one_row_or_conflict(rows_affected, "逻辑删除")
}

fn ensure_mutated_one_row_or_conflict(rows_affected: u64, operation: &str) -> AppResult<()> {
    match rows_affected {
        EXPECTED_SINGLE_ROW_AFFECTED => Ok(()),
        0 => Err(
            AppError::new(ErrorCode::Conflict, ErrorCode::Conflict.default_message())
                .with_internal_message(format!(
                    "{operation}未影响任何记录，可能是版本冲突、资源不存在或已被逻辑删除"
                )),
        ),
        value => Err(unexpected_rows_affected(operation, value)),
    }
}

fn ensure_exactly_one_row(rows_affected: u64, operation: &str) -> AppResult<()> {
    if rows_affected == EXPECTED_SINGLE_ROW_AFFECTED {
        Ok(())
    } else {
        Err(unexpected_rows_affected(operation, rows_affected))
    }
}

fn unexpected_rows_affected(operation: &str, rows_affected: u64) -> AppError {
    AppError::system(format!(
        "{operation}影响行数不符合预期: expected=1, actual={rows_affected}"
    ))
}

#[cfg(test)]
mod tests {
    use crate::error::error_code::ErrorCode;

    use super::{ensure_deleted, ensure_inserted, ensure_updated};

    #[test]
    fn ensure_inserted_requires_one_row() {
        // 新增路径必须影响 1 行，避免静默忽略写入失败。
        assert!(ensure_inserted(1).is_ok());
        assert_eq!(ensure_inserted(0).unwrap_err().code, ErrorCode::SystemError);
    }

    #[test]
    fn ensure_updated_zero_rows_returns_conflict() {
        // 乐观锁更新失败必须统一返回 CONFLICT，调用方据此刷新数据后重试。
        assert!(ensure_updated(1).is_ok());
        assert_eq!(ensure_updated(0).unwrap_err().code, ErrorCode::Conflict);
        assert_eq!(ensure_updated(2).unwrap_err().code, ErrorCode::SystemError);
    }

    #[test]
    fn ensure_deleted_zero_rows_returns_conflict() {
        // 逻辑删除失败同样走并发冲突语义，避免各模块返回不一致错误码。
        assert!(ensure_deleted(1).is_ok());
        assert_eq!(ensure_deleted(0).unwrap_err().code, ErrorCode::Conflict);
        assert_eq!(ensure_deleted(2).unwrap_err().code, ErrorCode::SystemError);
    }
}
