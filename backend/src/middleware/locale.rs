//! locale 中间件。
//!
//! Axum 横切入口统一放在 `middleware` 目录；具体语言协商和系统资源渲染仍由 `i18n` 模块负责。

use axum::{body::Body, extract::Request, extract::State, middleware::Next, response::Response};
use http::{HeaderName, HeaderValue, header::CONTENT_LANGUAGE};
use tracing::info;

use crate::{
    AppState, context::request_context::RequestContext, i18n::time_zone::CONTENT_TIME_ZONE_HEADER,
};

pub async fn locale_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let user_locale = request
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.locale_preference.as_deref());
    let user_time_zone = request
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.time_zone_preference.as_deref());
    let resolution = state
        .i18n
        .resolve_request(request.uri(), request.headers(), user_locale);
    let time_zone_resolution =
        state
            .i18n
            .resolve_time_zone_request(request.uri(), request.headers(), user_time_zone);

    if let Some(ctx) = request.extensions_mut().get_mut::<RequestContext>() {
        ctx.set_locale(
            resolution.locale.clone(),
            resolution.requested_locale.clone(),
        );
        ctx.set_time_zone(
            time_zone_resolution.time_zone.clone(),
            time_zone_resolution.requested_time_zone.clone(),
        );
    }

    info!(
        locale = %resolution.locale,
        requested_locale = ?resolution.requested_locale,
        time_zone = %time_zone_resolution.time_zone,
        requested_time_zone = ?time_zone_resolution.requested_time_zone,
        "请求语言和时区协商完成"
    );

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&resolution.locale) {
        response.headers_mut().insert(CONTENT_LANGUAGE, value);
    }
    if let Ok(value) = HeaderValue::from_str(&time_zone_resolution.time_zone) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(CONTENT_TIME_ZONE_HEADER), value);
    }
    response
}
