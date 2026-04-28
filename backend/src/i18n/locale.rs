//! locale 解析和协商。
//!
//! 语言协商必须集中在这里处理，保证请求参数、`X-Locale`、用户偏好、`Accept-Language`
//! 和配置默认语言的优先级在所有接口中一致。

use http::{HeaderMap, Uri, header::ACCEPT_LANGUAGE};
use serde::{Deserialize, Serialize};

pub const LOCALE_HEADER: &str = "x-locale";
pub const LOCALE_HEADER_CANONICAL: &str = "X-Locale";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Locale {
    /// 规范化后的 BCP 47 风格语言标识，例如 `zh-CN` 或 `en-US`。
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleResolution {
    /// 最终用于响应、翻译和缓存的 locale。
    pub locale: String,
    /// 调用方原始期望 locale，可能来自 query、`X-Locale` 或 `Accept-Language`。
    pub requested_locale: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LocaleResolver {
    /// 配置文件指定的默认 locale，不能在业务逻辑中写死。
    default_locale: String,
    /// 当前系统允许协商的 locale 列表，使用配置顺序作为语言简化匹配的优先级。
    supported_locales: Vec<String>,
}

impl Locale {
    /// 尝试规范化外部传入的 locale。
    ///
    /// 这里只做轻量 BCP 47 兼容校验，避免不可打印字符或过长值进入响应头、日志和缓存 key。
    pub fn normalize(value: &str) -> Option<Self> {
        let trimmed = value.trim().replace('_', "-");
        if !is_valid_locale_tag(&trimmed) {
            return None;
        }

        let parts = trimmed
            .split('-')
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    part.to_ascii_lowercase()
                } else if part.len() == 2 {
                    part.to_ascii_uppercase()
                } else {
                    part.to_string()
                }
            })
            .collect::<Vec<_>>();

        Some(Self {
            value: parts.join("-"),
        })
    }
}

impl LocaleResolver {
    /// 创建 locale 解析器。
    pub fn new(default_locale: impl Into<String>, supported_locales: Vec<String>) -> Self {
        Self {
            default_locale: default_locale.into(),
            supported_locales,
        }
    }

    /// 按固定优先级解析当前请求语言。
    pub fn resolve(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        user_locale: Option<&str>,
    ) -> LocaleResolution {
        let mut candidates = Vec::new();
        if let Some(locale) = query_locale(uri) {
            candidates.push(locale);
        }
        if let Some(locale) = header_locale(headers, LOCALE_HEADER) {
            candidates.push(locale);
        }
        if let Some(locale) = user_locale {
            candidates.push(locale.to_string());
        }
        // `Accept-Language` 可能包含多个候选语言，必须逐个尝试，避免第一个不支持时直接回退默认语言。
        candidates.extend(accept_language_locales(headers));

        for candidate in candidates {
            if let Some(locale) = self.match_supported_locale(&candidate) {
                return LocaleResolution {
                    locale,
                    requested_locale: Some(candidate),
                };
            }
        }

        LocaleResolution {
            locale: self.default_locale.clone(),
            requested_locale: None,
        }
    }

    /// 判断请求 locale 是否受支持，并处理 `en` 匹配 `en-US` 等常见简写。
    pub fn match_supported_locale(&self, value: &str) -> Option<String> {
        let normalized = Locale::normalize(value)?.value;
        self.supported_locales
            .iter()
            .find(|locale| locale.eq_ignore_ascii_case(&normalized))
            .cloned()
            .or_else(|| self.match_language_only(&normalized))
    }

    fn match_language_only(&self, normalized: &str) -> Option<String> {
        let language = normalized.split('-').next()?;
        self.supported_locales
            .iter()
            .find(|locale| {
                locale
                    .split('-')
                    .next()
                    .map(|supported_language| supported_language.eq_ignore_ascii_case(language))
                    .unwrap_or(false)
            })
            .cloned()
    }
}

pub fn is_valid_locale_tag(value: &str) -> bool {
    let len = value.len();
    if !(2..=35).contains(&len) {
        return false;
    }

    value
        .split('-')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphanumeric()))
}

fn query_locale(uri: &Uri) -> Option<String> {
    uri.query().and_then(|query| {
        query.split('&').find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some("locale"), Some(value)) if !value.trim().is_empty() => {
                    Some(value.trim().to_string())
                }
                _ => None,
            }
        })
    })
}

fn header_locale(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers
        .get(header_name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn accept_language_locales(headers: &HeaderMap) -> Vec<String> {
    let Some(value) = headers
        .get(ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
    else {
        return Vec::new();
    };
    let mut candidates = value
        .split(',')
        .enumerate()
        .filter_map(|(order, part)| parse_accept_language_part(order, part))
        .filter(|candidate| candidate.quality > 0)
        .collect::<Vec<_>>();

    // `Accept-Language` 的 q 值才是真正优先级；q 相同时保留请求头原始顺序。
    candidates.sort_by(|left, right| {
        right
            .quality
            .cmp(&left.quality)
            .then_with(|| left.order.cmp(&right.order))
    });

    candidates
        .into_iter()
        .map(|candidate| candidate.locale)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcceptLanguageCandidate {
    /// 请求头中的语言标签，后续仍由 `match_supported_locale` 负责规范化和支持性判断。
    locale: String,
    /// q 权重按千分位保存，避免浮点比较带来边界误差。
    quality: u16,
    /// 原始顺序用于 q 值相同时保持浏览器偏好稳定。
    order: usize,
}

fn parse_accept_language_part(order: usize, part: &str) -> Option<AcceptLanguageCandidate> {
    let mut segments = part.split(';');
    let locale = segments.next()?.trim();
    if locale.is_empty() {
        return None;
    }
    let mut quality = 1000;
    for segment in segments {
        let segment = segment.trim();
        if let Some(value) = segment.strip_prefix("q=") {
            quality = parse_accept_language_quality(value).unwrap_or(0);
        }
    }

    Some(AcceptLanguageCandidate {
        locale: locale.to_string(),
        quality,
        order,
    })
}

fn parse_accept_language_quality(value: &str) -> Option<u16> {
    let trimmed = value.trim();
    if trimmed == "1"
        || trimmed
            .strip_prefix("1.")
            .is_some_and(|rest| rest.bytes().all(|byte| byte == b'0'))
    {
        return Some(1000);
    }
    if trimmed == "0" {
        return Some(0);
    }
    let decimals = trimmed.strip_prefix("0.")?;
    if decimals.is_empty() || !decimals.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut padded = decimals.chars().take(3).collect::<String>();
    while padded.len() < 3 {
        padded.push('0');
    }
    padded.parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    use http::{HeaderMap, HeaderValue, Uri, header::ACCEPT_LANGUAGE};

    use super::{LOCALE_HEADER, Locale, LocaleResolver, accept_language_locales};

    fn resolver() -> LocaleResolver {
        LocaleResolver::new("zh-CN", vec!["zh-CN".to_string(), "en-US".to_string()])
    }

    #[test]
    fn locale_normalize_keeps_stable_bcp47_shape() {
        // 下划线和大小写差异来自浏览器或客户端，进入上下文前必须统一。
        assert_eq!(Locale::normalize("EN_us").unwrap().value, "en-US");
        assert!(Locale::normalize("bad locale").is_none());
    }

    #[test]
    fn resolver_prefers_query_over_header() {
        // query 显式参数优先级最高，用于资源接口或调试场景覆盖请求头。
        let uri: Uri = "/api/v1/i18n/system_resources?locale=en-US"
            .parse()
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(LOCALE_HEADER, HeaderValue::from_static("zh-CN"));

        let resolved = resolver().resolve(&uri, &headers, None);
        assert_eq!(resolved.locale, "en-US");
        assert_eq!(resolved.requested_locale, Some("en-US".to_string()));
    }

    #[test]
    fn resolver_tries_later_accept_language_candidates() {
        // 第一个浏览器语言不受支持时，必须继续匹配后续候选语言。
        let uri: Uri = "/health/live".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("fr-FR,en-US;q=0.9"),
        );

        let resolved = resolver().resolve(&uri, &headers, None);
        assert_eq!(resolved.locale, "en-US");
        assert_eq!(resolved.requested_locale, Some("en-US".to_string()));
    }

    #[test]
    fn accept_language_locales_keep_header_order() {
        // 语言候选列表要保留请求头顺序，供解析器逐个匹配支持语言。
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("fr-FR,en-US;q=0.9,zh-CN;q=0.8"),
        );

        assert_eq!(
            accept_language_locales(&headers),
            vec![
                "fr-FR".to_string(),
                "en-US".to_string(),
                "zh-CN".to_string()
            ]
        );
    }

    #[test]
    fn accept_language_locales_follow_quality_priority() {
        // q 权重高的语言应优先匹配，避免请求头原始顺序和真实偏好不一致。
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US;q=0.8,zh-CN;q=1.0"),
        );

        assert_eq!(
            accept_language_locales(&headers),
            vec!["zh-CN".to_string(), "en-US".to_string()]
        );
    }

    #[test]
    fn resolver_falls_back_to_config_default() {
        // 不支持的语言不能进入响应头和缓存 key，必须降级到配置默认语言。
        let uri: Uri = "/health/live".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(LOCALE_HEADER, HeaderValue::from_static("fr-FR"));

        let resolved = resolver().resolve(&uri, &headers, None);
        assert_eq!(resolved.locale, "zh-CN");
    }
}
