//! 地区化时间上下文。
//!
//! 后端仍以 UTC ISO 时间作为接口和存储标准；本模块只负责解析请求时区、下发前端
//! `Intl.DateTimeFormat` 可消费的格式 profile，并为导出、通知等服务端展示场景提供兜底格式化。

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use http::{HeaderMap, Uri};
use serde::Serialize;

use crate::{
    error::app_error::{AppError, AppResult},
    utils::query::query_param,
};

/// 请求头和响应头使用的时区字段，HTTP header 实际匹配大小写不敏感。
pub const TIME_ZONE_HEADER: &str = "x-time-zone";
/// 文档约定的规范请求头和响应头名称。
pub const TIME_ZONE_HEADER_CANONICAL: &str = "X-Time-Zone";
/// 响应头使用 `Content-*` 前缀表达本次响应采用的展示上下文。
pub const CONTENT_TIME_ZONE_HEADER: &str = "content-time-zone";
/// 文档约定的规范响应头名称。
pub const CONTENT_TIME_ZONE_HEADER_CANONICAL: &str = "Content-Time-Zone";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeZoneResolution {
    /// 最终用于展示、资源响应和响应头回写的 IANA time zone。
    pub time_zone: String,
    /// 调用方原始期望 time zone，可能来自 query、请求头或用户偏好。
    pub requested_time_zone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TimeZoneResolver {
    /// 配置文件指定的默认 time zone，业务代码禁止写死默认时区。
    default_time_zone: String,
    /// 当前系统允许协商和返回的 IANA time zone 列表。
    supported_time_zones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeFormatOptions {
    /// 前端 `Intl.DateTimeFormat` 的 `dateStyle`，例如 `short` 或 `medium`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_style: Option<String>,
    /// 前端 `Intl.DateTimeFormat` 的 `timeStyle`，例如 `short`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DateTimeFormatProfile {
    /// profile key，作为系统资源接口 `datetimeFormats` 的对象 key。
    pub key: &'static str,
    /// 前端 Intl 格式化选项。
    pub options: DateTimeFormatOptions,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimeDisplayContext {
    /// 当前展示语言。
    pub locale: String,
    /// 当前展示时区。
    pub time_zone: String,
    /// 前端可直接传给 `Intl.DateTimeFormat` 的格式 profile。
    pub datetime_formats: BTreeMap<String, DateTimeFormatOptions>,
}

impl TimeZoneResolver {
    /// 创建 time zone 解析器。
    pub fn new(default_time_zone: impl Into<String>, supported_time_zones: Vec<String>) -> Self {
        Self {
            default_time_zone: default_time_zone.into(),
            supported_time_zones,
        }
    }

    /// 按固定优先级解析当前请求时区。
    pub fn resolve(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        user_time_zone: Option<&str>,
    ) -> TimeZoneResolution {
        let candidates = [
            query_time_zone(uri),
            header_time_zone(headers),
            user_time_zone.map(ToOwned::to_owned),
        ];

        for candidate in candidates.into_iter().flatten() {
            if let Some(time_zone) = self.match_supported_time_zone(&candidate) {
                return TimeZoneResolution {
                    time_zone,
                    requested_time_zone: Some(candidate),
                };
            }
        }

        TimeZoneResolution {
            time_zone: self.default_time_zone.clone(),
            requested_time_zone: None,
        }
    }

    /// 判断请求 time zone 是否在配置允许范围内。
    pub fn match_supported_time_zone(&self, value: &str) -> Option<String> {
        let canonical = canonicalize_time_zone(value).ok()?;
        self.supported_time_zones
            .iter()
            .find(|time_zone| *time_zone == &canonical)
            .cloned()
    }

    /// 返回配置默认 time zone。
    pub fn default_time_zone(&self) -> &str {
        &self.default_time_zone
    }
}

/// 校验并规范化 IANA time zone。
pub fn canonicalize_time_zone(value: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || !is_safe_time_zone_value(trimmed) {
        return Err(AppError::param_invalid(format!("非法 time zone: {value}")));
    }

    let time_zone = trimmed
        .parse::<Tz>()
        .map_err(|_| AppError::param_invalid(format!("非法 time zone: {value}")))?;
    Ok(time_zone.name().to_string())
}

/// 构造默认日期时间格式 profile。
pub fn default_datetime_formats() -> BTreeMap<String, DateTimeFormatOptions> {
    datetime_format_profiles()
        .into_iter()
        .map(|profile| (profile.key.to_string(), profile.options))
        .collect()
}

/// 构造请求展示上下文。
pub fn time_display_context(locale: &str, time_zone: &str) -> TimeDisplayContext {
    TimeDisplayContext {
        locale: locale.to_string(),
        time_zone: time_zone.to_string(),
        datetime_formats: default_datetime_formats(),
    }
}

/// 按后端有限 profile 格式化 UTC 时间，服务端导出或通知可使用该兜底能力。
pub fn format_utc_datetime(
    value: DateTime<Utc>,
    locale: &str,
    time_zone: &str,
    profile_key: &str,
) -> AppResult<String> {
    let time_zone = canonicalize_time_zone(time_zone)?;
    let parsed_time_zone = time_zone
        .parse::<Tz>()
        .map_err(|_| AppError::param_invalid(format!("非法 time zone: {time_zone}")))?;
    let local_time = value.with_timezone(&parsed_time_zone);
    let pattern = server_format_pattern(locale, profile_key)?;
    Ok(local_time.format(pattern).to_string())
}

fn query_time_zone(uri: &Uri) -> Option<String> {
    query_param(uri, "timeZone")
}

fn header_time_zone(headers: &HeaderMap) -> Option<String> {
    headers
        .get(TIME_ZONE_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn datetime_format_profiles() -> Vec<DateTimeFormatProfile> {
    vec![
        DateTimeFormatProfile {
            key: "dateShort",
            options: DateTimeFormatOptions {
                date_style: Some("short".to_string()),
                time_style: None,
            },
        },
        DateTimeFormatProfile {
            key: "dateMedium",
            options: DateTimeFormatOptions {
                date_style: Some("medium".to_string()),
                time_style: None,
            },
        },
        DateTimeFormatProfile {
            key: "timeShort",
            options: DateTimeFormatOptions {
                date_style: None,
                time_style: Some("short".to_string()),
            },
        },
        DateTimeFormatProfile {
            key: "dateTimeShort",
            options: DateTimeFormatOptions {
                date_style: Some("short".to_string()),
                time_style: Some("short".to_string()),
            },
        },
        DateTimeFormatProfile {
            key: "dateTimeMedium",
            options: DateTimeFormatOptions {
                date_style: Some("medium".to_string()),
                time_style: Some("short".to_string()),
            },
        },
    ]
}

fn server_format_pattern(locale: &str, profile_key: &str) -> AppResult<&'static str> {
    let zh_locale = locale.to_ascii_lowercase().starts_with("zh");
    match (profile_key, zh_locale) {
        ("dateShort", _) => Ok("%Y-%m-%d"),
        ("dateMedium", true) => Ok("%Y年%-m月%-d日"),
        ("dateMedium", false) => Ok("%b %-d, %Y"),
        ("timeShort", true) => Ok("%H:%M"),
        ("timeShort", false) => Ok("%-I:%M %p"),
        ("dateTimeShort", true) => Ok("%Y-%m-%d %H:%M"),
        ("dateTimeShort", false) => Ok("%m/%d/%y %-I:%M %p"),
        ("dateTimeMedium", true) => Ok("%Y年%-m月%-d日 %H:%M"),
        ("dateTimeMedium", false) => Ok("%b %-d, %Y %-I:%M %p"),
        _ => Err(AppError::param_invalid(format!(
            "不支持的日期时间格式 profile: {profile_key}"
        ))),
    }
}

fn is_safe_time_zone_value(value: &str) -> bool {
    value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'+'))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use http::{HeaderMap, HeaderValue, Uri};

    use super::{
        TIME_ZONE_HEADER, TimeZoneResolver, canonicalize_time_zone, default_datetime_formats,
        format_utc_datetime,
    };

    fn resolver() -> TimeZoneResolver {
        TimeZoneResolver::new(
            "Asia/Shanghai",
            vec![
                "Asia/Shanghai".to_string(),
                "UTC".to_string(),
                "America/New_York".to_string(),
            ],
        )
    }

    #[test]
    fn time_zone_resolver_prefers_query_over_header() {
        // query 显式参数优先级最高，用于用户临时切换展示时区。
        let uri: Uri = "/health/live?timeZone=America%2FNew_York".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(TIME_ZONE_HEADER, HeaderValue::from_static("UTC"));

        let resolved = resolver().resolve(&uri, &headers, None);
        assert_eq!(resolved.time_zone, "America/New_York");
        assert_eq!(
            resolved.requested_time_zone,
            Some("America/New_York".to_string())
        );
    }

    #[test]
    fn time_zone_resolver_falls_back_to_default_when_invalid() {
        // 非法或不支持的请求时区不能进入响应头，必须降级到配置默认时区。
        let uri: Uri = "/health/live?timeZone=Bad/Zone".parse().unwrap();
        let headers = HeaderMap::new();

        let resolved = resolver().resolve(&uri, &headers, None);
        assert_eq!(resolved.time_zone, "Asia/Shanghai");
        assert_eq!(resolved.requested_time_zone, None);
    }

    #[test]
    fn canonicalize_time_zone_accepts_supported_iana_names() {
        // IANA 时区名称必须能被规范化，避免大小写或非法字符污染上下文。
        assert_eq!(
            canonicalize_time_zone("Asia/Shanghai").unwrap(),
            "Asia/Shanghai"
        );
        assert_eq!(canonicalize_time_zone("UTC").unwrap(), "UTC");
        assert!(canonicalize_time_zone("Asia/NotExists").is_err());
    }

    #[test]
    fn default_datetime_formats_are_intl_compatible_profiles() {
        // 下发给前端的 profile 必须使用 camelCase 字段，便于直接映射 Intl.DateTimeFormat。
        let formats = default_datetime_formats();
        assert_eq!(
            formats.get("dateTimeShort").unwrap().date_style.as_deref(),
            Some("short")
        );
        assert_eq!(
            formats.get("dateTimeShort").unwrap().time_style.as_deref(),
            Some("short")
        );
    }

    #[test]
    fn format_utc_datetime_uses_requested_time_zone() {
        // 同一个 UTC 时间在不同时区下格式化结果应不同，服务端导出场景依赖该能力。
        let value = Utc.with_ymd_and_hms(2026, 4, 28, 0, 0, 0).unwrap();
        let shanghai =
            format_utc_datetime(value, "zh-CN", "Asia/Shanghai", "dateTimeShort").unwrap();
        let new_york =
            format_utc_datetime(value, "en-US", "America/New_York", "dateTimeShort").unwrap();

        assert_eq!(shanghai, "2026-04-28 08:00");
        assert_eq!(new_york, "04/27/26 8:00 PM");
    }
}
