//! 密码哈希工具。
//!
//! 密码只在认证模块内部处理，禁止把明文密码写入日志、数据库或响应。

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

use crate::error::app_error::{AppError, AppResult};

pub const PASSWORD_ALGO_ARGON2ID: &str = "argon2id";

/// 使用 Argon2id 生成密码哈希。
pub fn hash_password(password: &str) -> AppResult<String> {
    validate_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::system(format!("密码哈希生成失败: {err}")))
}

/// 校验明文密码是否匹配哈希。
pub fn verify_password(password: &str, password_hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(password_hash)
        .map_err(|err| AppError::system(format!("密码哈希解析失败: {err}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// 首版密码策略只校验最小长度，复杂度策略后续通过配置扩展。
pub fn validate_password(password: &str) -> AppResult<()> {
    if password.len() < 8 {
        return Err(AppError::business_error("密码长度不能少于 8 位"));
    }
    Ok(())
}
