//! 认证中间件。
//!
//! 中间件校验访问令牌并写入 RequestContext。权限判断不在这里实现，后续由权限模块基于认证上下文处理。

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http::header::AUTHORIZATION;

use crate::{
    AppState,
    context::request_context::{AuthType, RequestContext},
    error::app_error::AppError,
    modules::auth::{service::AuthService, token::AuthTokenService},
};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if is_public_path(&path) || state.database.is_none() {
        return next.run(request).await;
    }
    let token = match bearer_token(&request) {
        Ok(token) => token,
        Err(err) => return auth_error_response(&state, &request, err),
    };
    let claims = match AuthTokenService::verify_access_token(&state.config.auth, token) {
        Ok(claims) => claims,
        Err(err) => return auth_error_response(&state, &request, err),
    };
    let Some(pool) = state.database.as_ref() else {
        return next.run(request).await;
    };
    if let Err(err) =
        AuthService::ensure_authenticated_request(pool, &claims.sub, &claims.tid, &claims.sid).await
    {
        return auth_error_response(&state, &request, err);
    }
    if let Some(header_tenant) = request
        .headers()
        .get(crate::middleware::tenant::TENANT_ID_HEADER_CANONICAL)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if header_tenant != claims.tid {
            return auth_error_response(
                &state,
                &request,
                AppError::forbidden("token 租户与请求租户冲突"),
            );
        }
    }
    if let Some(ctx) = request.extensions_mut().get_mut::<RequestContext>() {
        ctx.user_id = Some(claims.sub);
        ctx.set_tenant(claims.tid, "token");
        ctx.auth_type = AuthType::User;
        ctx.is_authenticated = true;
    }
    next.run(request).await
}

fn bearer_token(request: &Request<Body>) -> Result<&str, AppError> {
    let value = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("缺少访问令牌"))?;
    value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| AppError::unauthorized("访问令牌格式不合法"))
}

fn is_public_path(path: &str) -> bool {
    path.starts_with("/health")
        || path.starts_with("/api/v1/i18n/system_resources")
        || path == "/api/v1/auth/login"
        || path == "/api/v1/auth/refresh"
}

fn auth_error_response(state: &AppState, request: &Request<Body>, err: AppError) -> Response {
    let ctx = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::anonymous("missing-trace-id"));
    err.into_response_with_context(&ctx, &state.i18n)
        .into_response()
}
