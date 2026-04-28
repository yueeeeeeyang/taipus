//! 多语言服务。
//!
//! `I18nService` 是后端系统文本和资源分发的统一入口。handler、service 和错误转换层应通过该
//! 服务获取翻译结果，避免散落 `rust-i18n` 调用和 fallback 逻辑。

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use http::{HeaderMap, Uri};
use rust_i18n::t;
use tracing::{info, warn};

use crate::{
    config::settings::I18nConfig,
    context::request_context::RequestContext,
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    i18n::{
        business_translation::{
            BusinessTranslationFieldMap, BusinessTranslationLocalizeRequest,
            BusinessTranslationReadResponse, BusinessTranslationValue,
            BusinessTranslationWriteCommand, BusinessTranslationWriteResponse, LocalizedText,
        },
        business_translation_registry::{
            BusinessTranslationRegistry, normalize_field_name, normalize_resource_type,
        },
        business_translation_repository::BusinessTranslationRepository,
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
    /// 业务翻译资源注册表，用于限制通用 API 可操作的资源和字段。
    business_registry: BusinessTranslationRegistry,
    /// 业务翻译 repository，用于封装 SQLx 和数据库方言差异。
    business_repository: BusinessTranslationRepository,
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
            business_registry: BusinessTranslationRegistry::with_default_resources(),
            business_repository: BusinessTranslationRepository::new(),
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

    /// 返回业务翻译资源注册表。
    pub fn business_registry(&self) -> &BusinessTranslationRegistry {
        &self.business_registry
    }

    /// 校验业务翻译资源和字段。
    ///
    /// handler 在检查数据库连接前调用该方法，可以让非法资源和非法字段稳定返回参数错误。
    pub fn validate_business_translation_fields(
        &self,
        resource_type: &str,
        fields: Option<&str>,
    ) -> AppResult<Vec<String>> {
        let resource_type = normalize_resource_type(resource_type)?;
        if let Some(fields) = fields.filter(|value| !value.trim().is_empty()) {
            let requested_fields = fields
                .split(',')
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .collect::<Vec<_>>();
            return self
                .business_registry
                .validate_fields(&resource_type, requested_fields);
        }

        let policy = self.business_registry.validate_resource(&resource_type)?;
        Ok(policy.allowed_fields())
    }

    /// 校验并规范化业务翻译写入命令。
    ///
    /// handler 可在检查数据库连接前调用该方法，让资源、字段和 locale 错误稳定返回参数错误。
    pub fn validate_business_translation_write_command(
        &self,
        resource_type: &str,
        command: BusinessTranslationWriteCommand,
    ) -> AppResult<BusinessTranslationWriteCommand> {
        let resource_type = normalize_resource_type(resource_type)?;
        self.normalize_write_command(&resource_type, command)
    }

    /// 读取单个业务资源的翻译集合。
    pub async fn get_business_translations(
        &self,
        database: &DatabasePool,
        resource_type: &str,
        resource_id: &str,
        fields: Option<&str>,
        ctx: &RequestContext,
    ) -> AppResult<BusinessTranslationReadResponse> {
        let resource_type = normalize_resource_type(resource_type)?;
        let resource_id = normalize_resource_id(resource_id)?;
        let field_names = self.validate_business_translation_fields(&resource_type, fields)?;
        let policy = self.business_registry.validate_resource(&resource_type)?;
        policy.resource_exists(&resource_id)?;
        policy.check_read(ctx, &resource_id)?;

        self.business_repository
            .read_resource(database, &resource_type, &resource_id, &field_names)
            .await
    }

    /// 批量覆盖设置业务资源的多语言文本。
    pub async fn set_business_translations(
        &self,
        database: &DatabasePool,
        resource_type: &str,
        resource_id: &str,
        command: BusinessTranslationWriteCommand,
        ctx: &RequestContext,
    ) -> AppResult<BusinessTranslationWriteResponse> {
        let resource_type = normalize_resource_type(resource_type)?;
        let resource_id = normalize_resource_id(resource_id)?;
        let command = self.normalize_write_command(&resource_type, command)?;
        let policy = self.business_registry.validate_resource(&resource_type)?;
        policy.resource_exists(&resource_id)?;
        policy.check_write(ctx, &resource_id)?;
        let operator = ctx
            .user_id
            .clone()
            .unwrap_or_else(|| "anonymous".to_string());

        let response = self
            .business_repository
            .replace_resource_fields(
                database,
                &resource_type,
                &resource_id,
                command.version,
                &command.fields,
                &operator,
            )
            .await?;

        for (field_name, locale_values) in &response.fields {
            for locale in locale_values.keys() {
                info!(
                    trace_id = %ctx.trace_id,
                    resource_type = %response.resource_type,
                    resource_id = %response.resource_id,
                    field_name,
                    locale,
                    operator = %operator,
                    version = response.version,
                    "业务翻译写入成功"
                );
            }
        }

        Ok(BusinessTranslationWriteResponse {
            resource_type: response.resource_type,
            resource_id: response.resource_id,
            version: response.version,
            fields: response.fields,
        })
    }

    /// 本地化单个业务文本字段。
    ///
    /// 读取顺序固定为当前请求 locale、配置默认 locale、业务主表原始值。
    pub async fn localize_text(
        &self,
        database: &DatabasePool,
        resource_type: &str,
        resource_id: &str,
        field_name: &str,
        default_value: impl Into<String>,
        ctx: &RequestContext,
    ) -> AppResult<LocalizedText<String>> {
        let requests = vec![BusinessTranslationLocalizeRequest::new(
            resource_type,
            resource_id,
            field_name,
            default_value,
        )];
        let mut values = self.localize_batch(database, requests, ctx).await?;
        Ok(values.remove(0))
    }

    /// 批量本地化业务文本字段。
    ///
    /// 列表页应优先使用该方法，把同一页数据合并查询，避免每条记录单独读取翻译表。
    pub async fn localize_batch(
        &self,
        database: &DatabasePool,
        requests: Vec<BusinessTranslationLocalizeRequest>,
        ctx: &RequestContext,
    ) -> AppResult<Vec<LocalizedText<String>>> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let target_locale = self.resolve_supported_locale(&ctx.locale)?;
        let default_locale = self.default_locale.clone();
        let locales = unique_locale_candidates(&target_locale, &default_locale);
        let normalized_requests = requests
            .into_iter()
            .map(normalize_localize_request)
            .collect::<AppResult<Vec<_>>>()?;
        let translations = self
            .load_batch_translations(database, &normalized_requests, &locales)
            .await?;

        Ok(normalized_requests
            .into_iter()
            .map(|request| {
                let key = translation_lookup_key(
                    &request.resource_type,
                    &request.resource_id,
                    &request.field_name,
                );
                let locale_values = translations.get(&key);
                resolve_localized_text(
                    &target_locale,
                    &default_locale,
                    &request.default_value,
                    locale_values,
                )
            })
            .collect())
    }

    fn normalize_write_command(
        &self,
        resource_type: &str,
        command: BusinessTranslationWriteCommand,
    ) -> AppResult<BusinessTranslationWriteCommand> {
        if command.version < 0 {
            return Err(AppError::param_invalid("业务翻译版本号不能小于 0"));
        }
        if command.fields.is_empty() {
            return Err(AppError::param_invalid("业务翻译字段不能为空"));
        }

        let mut normalized_fields = BusinessTranslationFieldMap::new();
        for (field_name, locale_values) in command.fields {
            let field_name = normalize_field_name(&field_name)?;
            self.business_registry
                .validate_fields(resource_type, [&field_name])?;
            if locale_values.is_empty() {
                return Err(AppError::param_invalid(format!(
                    "字段 {field_name} 的翻译 locale 不能为空"
                )));
            }

            let mut normalized_locale_values = BTreeMap::new();
            for (locale, text_value) in locale_values {
                let locale = self.resolve_supported_locale(&locale)?;
                if normalized_locale_values
                    .insert(locale.clone(), text_value)
                    .is_some()
                {
                    return Err(AppError::param_invalid(format!(
                        "字段 {field_name} 存在重复 locale: {locale}"
                    )));
                }
            }
            if normalized_fields
                .insert(field_name.clone(), normalized_locale_values)
                .is_some()
            {
                return Err(AppError::param_invalid(format!(
                    "业务翻译存在重复字段: {field_name}"
                )));
            }
        }

        Ok(BusinessTranslationWriteCommand {
            version: command.version,
            fields: normalized_fields,
        })
    }

    async fn load_batch_translations(
        &self,
        database: &DatabasePool,
        requests: &[BusinessTranslationLocalizeRequest],
        locales: &[String],
    ) -> AppResult<BTreeMap<String, BTreeMap<String, String>>> {
        let mut grouped = BTreeMap::<String, (BTreeSet<String>, BTreeSet<String>)>::new();
        for request in requests {
            self.business_registry
                .validate_fields(&request.resource_type, [&request.field_name])?;
            let entry = grouped
                .entry(request.resource_type.clone())
                .or_insert_with(|| (BTreeSet::new(), BTreeSet::new()));
            entry.0.insert(request.resource_id.clone());
            entry.1.insert(request.field_name.clone());
        }

        let mut translations = BTreeMap::new();
        for (resource_type, (resource_ids, field_names)) in grouped {
            let resource_ids = resource_ids.into_iter().collect::<Vec<_>>();
            let field_names = field_names.into_iter().collect::<Vec<_>>();
            let values = self
                .business_repository
                .find_active(
                    database,
                    &resource_type,
                    &resource_ids,
                    &field_names,
                    Some(locales),
                )
                .await?;
            index_translation_values(&mut translations, &resource_type, values);
        }

        Ok(translations)
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

    fn resolve_supported_locale(&self, locale: &str) -> AppResult<String> {
        self.resolver
            .match_supported_locale(locale)
            .ok_or_else(|| AppError::param_invalid(format!("不支持的 locale: {locale}")))
    }
}

fn normalize_resource_id(value: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err(AppError::param_invalid(format!("非法业务资源 ID: {value}")));
    }
    if trimmed
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        Ok(trimmed.to_string())
    } else {
        Err(AppError::param_invalid(format!("非法业务资源 ID: {value}")))
    }
}

fn normalize_localize_request(
    request: BusinessTranslationLocalizeRequest,
) -> AppResult<BusinessTranslationLocalizeRequest> {
    Ok(BusinessTranslationLocalizeRequest {
        resource_type: normalize_resource_type(&request.resource_type)?,
        resource_id: normalize_resource_id(&request.resource_id)?,
        field_name: normalize_field_name(&request.field_name)?,
        default_value: request.default_value,
    })
}

fn unique_locale_candidates(target_locale: &str, default_locale: &str) -> Vec<String> {
    let mut locales = Vec::with_capacity(2);
    locales.push(target_locale.to_string());
    if target_locale != default_locale {
        locales.push(default_locale.to_string());
    }
    locales
}

fn translation_lookup_key(resource_type: &str, resource_id: &str, field_name: &str) -> String {
    format!("{resource_type}\u{1f}{resource_id}\u{1f}{field_name}")
}

fn index_translation_values(
    output: &mut BTreeMap<String, BTreeMap<String, String>>,
    resource_type: &str,
    values: Vec<BusinessTranslationValue>,
) {
    for value in values {
        let key = translation_lookup_key(resource_type, &value.resource_id, &value.field_name);
        output
            .entry(key)
            .or_default()
            .insert(value.locale, value.text_value);
    }
}

fn resolve_localized_text(
    target_locale: &str,
    default_locale: &str,
    default_value: &str,
    locale_values: Option<&BTreeMap<String, String>>,
) -> LocalizedText<String> {
    if let Some(value) = locale_values.and_then(|values| values.get(target_locale)) {
        return LocalizedText {
            value: value.clone(),
            requested_locale: Some(target_locale.to_string()),
            locale: target_locale.to_string(),
            translation_missing: false,
        };
    }

    if let Some(value) = locale_values.and_then(|values| values.get(default_locale)) {
        return LocalizedText {
            value: value.clone(),
            requested_locale: Some(target_locale.to_string()),
            locale: default_locale.to_string(),
            translation_missing: target_locale != default_locale,
        };
    }

    LocalizedText {
        value: default_value.to_string(),
        requested_locale: Some(target_locale.to_string()),
        locale: default_locale.to_string(),
        translation_missing: target_locale != default_locale,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{resolve_localized_text, unique_locale_candidates};

    #[test]
    fn localized_text_prefers_target_locale() {
        // 命中目标语言时不能误标记为缺失翻译。
        let values = BTreeMap::from([
            ("zh-CN".to_string(), "客户登记".to_string()),
            ("en-US".to_string(), "Customer Registration".to_string()),
        ]);

        let text = resolve_localized_text("en-US", "zh-CN", "客户登记", Some(&values));

        assert_eq!(text.value, "Customer Registration");
        assert_eq!(text.locale, "en-US");
        assert!(!text.translation_missing);
    }

    #[test]
    fn localized_text_falls_back_to_default_locale() {
        // 目标语言缺失时应返回默认语言翻译，并明确标记缺失，方便管理端补翻译。
        let values = BTreeMap::from([("zh-CN".to_string(), "客户登记".to_string())]);

        let text = resolve_localized_text("en-US", "zh-CN", "客户登记原始值", Some(&values));

        assert_eq!(text.value, "客户登记");
        assert_eq!(text.locale, "zh-CN");
        assert!(text.translation_missing);
    }

    #[test]
    fn localized_text_falls_back_to_business_original_value() {
        // 默认语言翻译也不存在时，业务主表原始值是最后 fallback，保证业务页面可展示。
        let text = resolve_localized_text("en-US", "zh-CN", "客户登记原始值", None);

        assert_eq!(text.value, "客户登记原始值");
        assert_eq!(text.locale, "zh-CN");
        assert!(text.translation_missing);
    }

    #[test]
    fn locale_candidates_deduplicate_default_locale() {
        // 当前 locale 已经是默认语言时，只需要查一次翻译表。
        assert_eq!(unique_locale_candidates("zh-CN", "zh-CN"), vec!["zh-CN"]);
        assert_eq!(
            unique_locale_candidates("en-US", "zh-CN"),
            vec!["en-US", "zh-CN"]
        );
    }
}
