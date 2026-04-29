//! 租户解析中间件。
//!
//! 首版支持 `X-Tenant-Id` 和默认租户两种来源，并在存在数据库连接池时校验普通业务请求的租户状态。

use axum::{
    body::Body, extract::Request, extract::State, middleware::Next, response::IntoResponse,
    response::Response,
};
use http::HeaderName;
use tracing::info;

use crate::{
    AppState, context::request_context::RequestContext, error::app_error::AppError,
    modules::tenant::service::TenantService,
};

pub const TENANT_ID_HEADER: &str = "x-tenant-id";
pub const TENANT_ID_HEADER_CANONICAL: &str = "X-Tenant-Id";

pub async fn tenant_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let (tenant_id, tenant_source) = match resolve_tenant(&state, &request) {
        Ok(value) => value,
        Err(err) => return tenant_error_response(&state, &request, err),
    };

    if should_validate_tenant(&path) {
        if let Some(pool) = state.database.as_ref() {
            if let Err(err) = TenantService::ensure_request_tenant_active(pool, &tenant_id).await {
                return tenant_error_response(&state, &request, err);
            }
        }
    }

    if let Some(ctx) = request.extensions_mut().get_mut::<RequestContext>() {
        ctx.set_tenant(tenant_id.clone(), tenant_source.clone());
    }

    info!(
        %tenant_id,
        %tenant_source,
        "请求租户解析完成"
    );

    next.run(request).await
}

fn resolve_tenant(state: &AppState, request: &Request<Body>) -> Result<(String, String), AppError> {
    let header_tenant = request
        .headers()
        .get(HeaderName::from_static(TENANT_ID_HEADER))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(tenant_id) = header_tenant {
        if !state.config.tenant.allow_header_override {
            return Err(AppError::forbidden("当前环境不允许通过请求头指定租户"));
        }
        validate_tenant_id(tenant_id)?;
        return Ok((tenant_id.to_string(), "header".to_string()));
    }

    let default_tenant_id = state.config.tenant.default_tenant_id.trim();
    if default_tenant_id.is_empty() {
        return Err(AppError::param_invalid("租户上下文缺失"));
    }
    validate_tenant_id(default_tenant_id)?;
    Ok((default_tenant_id.to_string(), "default".to_string()))
}

fn should_validate_tenant(path: &str) -> bool {
    !(path.starts_with("/health")
        || path.starts_with("/api/v1/i18n/system_resources")
        || path.starts_with("/api/v1/system/tenants"))
}

fn validate_tenant_id(value: &str) -> Result<(), AppError> {
    let len = value.len();
    let valid = (1..=64).contains(&len)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(AppError::param_invalid(
            "租户 ID 只能包含字母、数字、下划线和中横线",
        ))
    }
}

fn tenant_error_response(state: &AppState, request: &Request<Body>, err: AppError) -> Response {
    let ctx = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext::anonymous("missing-trace-id"));
    err.into_response_with_context(&ctx, &state.i18n)
        .into_response()
}
