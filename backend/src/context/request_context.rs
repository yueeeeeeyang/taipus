//! 请求上下文。
//!
//! 上下文保存 traceId、用户、租户、客户端等横切信息。handler、service、repository 和审计日志
//! 必须通过该结构传递链路信息，避免每层重复解析请求头。

use std::{convert::Infallible, time::Instant};

use axum::extract::FromRequestParts;
use http::request::Parts;
use serde::{Deserialize, Serialize};

use crate::utils::id::generate_trace_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestContext {
    /// 请求进入后端时的单调时钟时间点，仅用于计算后端处理耗时，不参与序列化。
    #[serde(skip, default = "Instant::now")]
    pub request_started_at: Instant,
    /// 请求链路唯一标识，必须与响应体和 `X-Trace-Id` 响应头一致。
    pub trace_id: String,
    /// 租户标识，首版可以为空，后续多租户能力会使用该字段做数据隔离。
    pub tenant_id: Option<String>,
    /// 当前用户标识，匿名接口为空。
    pub user_id: Option<String>,
    /// 当前用户角色集合，供 service 层权限检查使用。
    pub roles: Vec<String>,
    /// 客户端 IP，供审计、风控和限流扩展使用。
    pub client_ip: Option<String>,
    /// 客户端 User-Agent，供审计和问题排查使用。
    pub user_agent: Option<String>,
    /// 最终采用的 locale，由多语言中间件按固定优先级解析。
    pub locale: String,
    /// 调用方原始请求的 locale，可能来自 query、请求头或 `Accept-Language`。
    pub requested_locale: Option<String>,
    /// 用户偏好 locale，后续真实鉴权接入后由用户设置写入。
    pub locale_preference: Option<String>,
    /// 最终采用的 IANA time zone，由多语言中间件按固定优先级解析。
    pub time_zone: String,
    /// 调用方原始请求的 time zone，可能来自 query 或 `X-Time-Zone`。
    pub requested_time_zone: Option<String>,
    /// 用户偏好 time zone，后续真实鉴权接入后由用户设置写入。
    pub time_zone_preference: Option<String>,
    /// 鉴权类型，用于区分匿名、用户和系统任务调用。
    pub auth_type: AuthType,
    /// 是否已认证，避免调用方只依赖 `user_id` 是否为空做权限判断。
    pub is_authenticated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuthType {
    /// 匿名请求，通常只允许访问显式开放接口。
    Anonymous,
    /// 终端用户请求。
    User,
    /// 系统任务或内部服务调用。
    System,
}

impl RequestContext {
    /// 构造匿名请求上下文。
    ///
    /// traceId 仍然必须存在，因为匿名接口同样需要日志、错误和响应链路关联。
    pub fn anonymous(trace_id: impl Into<String>) -> Self {
        Self {
            request_started_at: Instant::now(),
            trace_id: trace_id.into(),
            tenant_id: None,
            user_id: None,
            roles: Vec::new(),
            client_ip: None,
            user_agent: None,
            locale: "und".to_string(),
            requested_locale: None,
            locale_preference: None,
            time_zone: "UTC".to_string(),
            requested_time_zone: None,
            time_zone_preference: None,
            auth_type: AuthType::Anonymous,
            is_authenticated: false,
        }
    }

    /// 追加客户端信息。
    ///
    /// 中间件解析客户端 IP 和 User-Agent 后写入上下文，后续审计能力可以直接复用。
    pub fn with_client_info(
        mut self,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        self.client_ip = client_ip;
        self.user_agent = user_agent;
        self
    }

    /// 写入语言协商结果。
    ///
    /// locale 中间件必须在 handler 执行前调用该方法，保证响应体、响应头和日志使用同一语言。
    pub fn set_locale(&mut self, locale: impl Into<String>, requested_locale: Option<String>) {
        self.locale = locale.into();
        self.requested_locale = requested_locale;
    }

    /// 写入时区协商结果。
    ///
    /// locale 中间件必须在 handler 执行前调用该方法，保证响应头、资源接口和日志使用同一时区。
    pub fn set_time_zone(
        &mut self,
        time_zone: impl Into<String>,
        requested_time_zone: Option<String>,
    ) {
        self.time_zone = time_zone.into();
        self.requested_time_zone = requested_time_zone;
    }

    /// 返回当前请求在后端已消耗的毫秒数。
    ///
    /// 该值使用单调时钟计算，避免系统时间回拨影响接口响应中的耗时字段。
    pub fn elapsed_ms(&self) -> u64 {
        self.request_started_at
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    /// 从请求扩展中提取上下文。
    ///
    /// 如果上游中间件未写入上下文，则生成兜底 traceId，保证 handler 不需要处理缺失上下文。
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_else(|| RequestContext::anonymous(generate_trace_id())))
    }
}
