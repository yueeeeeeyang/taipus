//! JWT 与刷新令牌工具。
//!
//! 访问令牌使用 RS256 JWT；刷新令牌使用不透明随机值，数据库只保存 pepper 后的哈希。

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    config::settings::AuthConfig,
    error::app_error::{AppError, AppResult},
};

/// 访问令牌 claims。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub iss: String,
    pub sub: String,
    pub tid: String,
    pub sid: String,
    pub aud: String,
    pub iat: i64,
    pub nbf: i64,
    pub exp: i64,
    pub jti: String,
    pub auth_time: i64,
    pub client_type: String,
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

pub struct AuthTokenService;

impl AuthTokenService {
    /// 签发访问令牌和刷新令牌。
    pub fn issue_pair(
        config: &AuthConfig,
        account_id: &str,
        tenant_id: &str,
        session_id: &str,
        client_type: &str,
    ) -> AppResult<TokenPair> {
        let now = Utc::now();
        let access_expires_at = now + Duration::seconds(config.access_token_ttl_seconds);
        let refresh_expires_at = now + Duration::seconds(config.refresh_token_ttl_seconds);
        let claims = AccessTokenClaims {
            iss: config.jwt_issuer.clone(),
            sub: account_id.to_string(),
            tid: tenant_id.to_string(),
            sid: session_id.to_string(),
            aud: config.jwt_audience.clone(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            exp: access_expires_at.timestamp(),
            jti: crate::utils::id::generate_business_id(),
            auth_time: now.timestamp(),
            client_type: client_type.to_string(),
        };
        let access_token = encode_claims(config, &claims)?;
        Ok(TokenPair {
            access_token,
            refresh_token: generate_refresh_token(),
            access_expires_at,
            refresh_expires_at,
        })
    }

    /// 校验访问令牌并返回 claims。
    pub fn verify_access_token(config: &AuthConfig, token: &str) -> AppResult<AccessTokenClaims> {
        let public_key = config
            .jwt_public_key_pem
            .as_deref()
            .ok_or_else(|| AppError::system("AUTH_JWT_PUBLIC_KEY_PEM 未配置"))?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[config.jwt_issuer.as_str()]);
        validation.set_audience(&[config.jwt_audience.as_str()]);
        let header = jsonwebtoken::decode_header(token)
            .map_err(|_| AppError::unauthorized("访问令牌无效"))?;
        if header.alg != Algorithm::RS256 || header.kid.as_deref() != Some(config.jwt_kid.as_str())
        {
            return Err(AppError::unauthorized("访问令牌无效"));
        }
        decode::<AccessTokenClaims>(
            token,
            &DecodingKey::from_rsa_pem(public_key.as_bytes())
                .map_err(|err| AppError::system(format!("JWT 公钥解析失败: {err}")))?,
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::unauthorized("访问令牌无效或已过期"))
    }
}

fn encode_claims(config: &AuthConfig, claims: &AccessTokenClaims) -> AppResult<String> {
    let private_key = config
        .jwt_private_key_pem
        .as_deref()
        .ok_or_else(|| AppError::system("AUTH_JWT_PRIVATE_KEY_PEM 未配置"))?;
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(config.jwt_kid.clone());
    encode(
        &header,
        claims,
        &EncodingKey::from_rsa_pem(private_key.as_bytes())
            .map_err(|err| AppError::system(format!("JWT 私钥解析失败: {err}")))?,
    )
    .map_err(|err| AppError::system(format!("JWT 签发失败: {err}")))
}

/// 生成 URL 安全的高熵刷新令牌。
pub fn generate_refresh_token() -> String {
    let mut bytes = [0_u8; 48];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 对刷新令牌做 pepper 哈希，避免数据库泄漏后可直接使用明文令牌。
pub fn hash_refresh_token(config: &AuthConfig, refresh_token: &str) -> AppResult<String> {
    let pepper = config
        .refresh_token_pepper
        .as_deref()
        .ok_or_else(|| AppError::system("AUTH_REFRESH_TOKEN_PEPPER 未配置"))?;
    let mut hasher = Sha256::new();
    hasher.update(pepper.as_bytes());
    hasher.update(b":");
    hasher.update(refresh_token.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}
