//! 多语言服务。
//!
//! `I18nService` 是后端系统文本和资源分发的统一入口。handler、service 和错误转换层应通过该
//! 服务获取翻译结果，避免散落 `rust-i18n` 调用和 fallback 逻辑。

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use http::{HeaderMap, Uri};
use rust_i18n::t;
use tracing::warn;

use crate::{
    config::settings::I18nConfig,
    error::app_error::{AppError, AppResult},
    i18n::{
        locale::{LocaleResolution, LocaleResolver},
        system_resource::{SystemResourcesResponse, default_namespaces, namespace_keys},
        time_zone::{
            TimeDisplayContext, TimeZoneResolution, TimeZoneResolver, format_utc_datetime,
            time_display_context,
        },
    },
};

#[derive(Debug, Clone)]
pub struct I18nService {
    /// 配置文件指定的默认 locale。
    default_locale: String,
    /// 允许被请求协商命中的 locale 列表。
    supported_locales: Vec<String>,
    /// 配置文件指定的默认 time zone。
    default_time_zone: String,
    /// 允许被请求协商命中的 time zone 列表。
    supported_time_zones: Vec<String>,
    /// 系统资源版本，用于前端和移动端缓存失效。
    resource_version: String,
    /// 语言协商器，统一实现请求语言优先级。
    resolver: LocaleResolver,
    /// 时区协商器，统一实现请求时区优先级。
    time_zone_resolver: TimeZoneResolver,
}

impl I18nService {
    /// 根据应用配置创建多语言服务。
    pub fn new(config: &I18nConfig) -> Self {
        Self {
            default_locale: config.default_locale.clone(),
            supported_locales: config.supported_locales.clone(),
            default_time_zone: config.default_time_zone.clone(),
            supported_time_zones: config.supported_time_zones.clone(),
            resource_version: config.system_resource_version.clone(),
            resolver: LocaleResolver::new(
                config.default_locale.clone(),
                config.supported_locales.clone(),
            ),
            time_zone_resolver: TimeZoneResolver::new(
                config.default_time_zone.clone(),
                config.supported_time_zones.clone(),
            ),
        }
    }

    /// 解析当前请求最终使用的 locale。
    pub fn resolve_request(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        user_locale: Option<&str>,
    ) -> LocaleResolution {
        self.resolver.resolve(uri, headers, user_locale)
    }

    /// 解析当前请求最终使用的 time zone。
    pub fn resolve_time_zone_request(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        user_time_zone: Option<&str>,
    ) -> TimeZoneResolution {
        self.time_zone_resolver
            .resolve(uri, headers, user_time_zone)
    }

    /// 返回配置默认 locale。
    pub fn default_locale(&self) -> &str {
        &self.default_locale
    }

    /// 返回配置默认 time zone。
    pub fn default_time_zone(&self) -> &str {
        &self.default_time_zone
    }

    /// 构造前端展示所需的 locale、time zone 和日期时间格式 profile。
    pub fn time_display_context(&self, locale: &str, time_zone: &str) -> TimeDisplayContext {
        time_display_context(locale, time_zone)
    }

    /// 服务端兜底格式化 UTC 时间，主要用于导出、通知或审计展示。
    pub fn format_datetime_for_display(
        &self,
        value: DateTime<Utc>,
        locale: &str,
        time_zone: &str,
        profile_key: &str,
    ) -> AppResult<String> {
        let resolved_locale = self
            .resolver
            .match_supported_locale(locale)
            .unwrap_or_else(|| self.default_locale.clone());
        let resolved_time_zone = self
            .time_zone_resolver
            .match_supported_time_zone(time_zone)
            .unwrap_or_else(|| self.default_time_zone.clone());
        format_utc_datetime(value, &resolved_locale, &resolved_time_zone, profile_key)
    }

    /// 渲染系统文本。
    ///
    /// 资源 key 缺失时返回 key 本身并记录告警，避免响应构造失败。
    pub fn system_text(&self, key: &str, locale: &str) -> String {
        let resolved_locale = self
            .resolver
            .match_supported_locale(locale)
            .unwrap_or_else(|| self.default_locale.clone());
        let translated = t!(key, locale = resolved_locale.as_str()).to_string();

        if translated == key {
            warn!(
                %key,
                locale = %resolved_locale,
                "系统多语言资源缺失"
            );
        }

        translated
    }

    /// 按 platform 和 namespace 构造系统资源响应。
    pub fn system_resources(
        &self,
        locale: &str,
        time_zone: &str,
        platform: Option<&str>,
        namespaces: Option<&str>,
    ) -> AppResult<SystemResourcesResponse> {
        let platform = platform.unwrap_or("frontend").trim();
        let namespace_list = self.resolve_namespaces(platform, namespaces)?;
        let mut resources = BTreeMap::new();

        for namespace in namespace_list {
            let keys = namespace_keys(namespace.as_str()).ok_or_else(|| {
                AppError::param_invalid(format!("不支持的多语言 namespace: {namespace}"))
            })?;
            let mut namespace_resources = BTreeMap::new();
            for key in keys {
                namespace_resources.insert(
                    key.output_key.to_string(),
                    self.system_text(key.message_key, locale),
                );
            }
            resources.insert(namespace, namespace_resources);
        }

        Ok(SystemResourcesResponse {
            locale: locale.to_string(),
            time_zone: time_zone.to_string(),
            fallback_locales: vec![self.default_locale.clone()],
            version: self.resource_version.clone(),
            datetime_formats: self
                .time_display_context(locale, time_zone)
                .datetime_formats,
            resources,
        })
    }

    fn resolve_namespaces(
        &self,
        platform: &str,
        namespaces: Option<&str>,
    ) -> AppResult<Vec<String>> {
        if let Some(namespaces) = namespaces.filter(|value| !value.trim().is_empty()) {
            return namespaces
                .split(',')
                .map(str::trim)
                .filter(|namespace| !namespace.is_empty())
                .map(|namespace| {
                    if namespace_keys(namespace).is_some() {
                        Ok(namespace.to_string())
                    } else {
                        Err(AppError::param_invalid(format!(
                            "不支持的多语言 namespace: {namespace}"
                        )))
                    }
                })
                .collect();
        }

        let defaults = default_namespaces(platform).ok_or_else(|| {
            AppError::param_invalid(format!("不支持的多语言 platform: {platform}"))
        })?;
        Ok(defaults
            .iter()
            .map(|namespace| (*namespace).to_string())
            .collect())
    }

    /// 判断 locale 是否在配置允许范围内。
    pub fn is_supported_locale(&self, locale: &str) -> bool {
        self.supported_locales
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(locale))
    }

    /// 判断 time zone 是否在配置允许范围内。
    pub fn is_supported_time_zone(&self, time_zone: &str) -> bool {
        self.supported_time_zones
            .iter()
            .any(|supported| supported == time_zone)
    }
}
