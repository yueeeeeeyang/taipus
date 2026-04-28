//! 鉴权占位中间件。
//!
//! 首版后端底座只定义鉴权边界，不绑定具体登录态、Token 或权限模型。后续接入真实鉴权时，
//! 应在这里解析身份，并把用户、租户和角色写入 `RequestContext`。

use axum::{body::Body, extract::Request, middleware::Next, response::Response};

pub async fn auth_placeholder_middleware(request: Request<Body>, next: Next) -> Response {
    // 首版不做真实鉴权，只保留统一扩展点；后续接入时必须写入 RequestContext。
    next.run(request).await
}
