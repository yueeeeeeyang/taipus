//! 权限鉴权中间件。
//!
//! 中间件在认证上下文写入后执行，按 method + normalized path 匹配 API 资源并判断 api.call 权限。

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    AppState, context::request_context::RequestContext, error::app_error::AppError,
    modules::permission::service::PermissionService,
};

pub async fn permission_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if is_code_public_path(&path) || state.database.is_none() {
        return next.run(request).await;
    }
    let method = request.method().as_str().to_ascii_uppercase();
    let ctx = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::anonymous("missing-trace-id"));
    let Some(pool) = state.database.as_ref() else {
        return next.run(request).await;
    };
    if let Err(err) = PermissionService::authorize_api(pool, &ctx, &method, &path).await {
        return permission_error_response(&state, &ctx, err);
    }
    next.run(request).await
}

fn is_code_public_path(path: &str) -> bool {
    path.starts_with("/health")
        || path.starts_with("/api/v1/i18n/system_resources")
        || path == "/api/v1/auth/login"
        || path == "/api/v1/auth/refresh"
}

fn permission_error_response(state: &AppState, ctx: &RequestContext, err: AppError) -> Response {
    err.into_response_with_context(ctx, &state.i18n)
        .into_response()
}
