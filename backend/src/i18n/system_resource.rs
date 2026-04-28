//! 系统多语言资源定义。
//!
//! 系统资源由后端统一分发给前端和移动端。这里保存 namespace 到稳定 key 的映射，资源值由
//! `rust-i18n` 根据请求 locale 渲染。

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub struct SystemResourceKey {
    /// 对前端输出的 key，使用 camelCase。
    pub output_key: &'static str,
    /// rust-i18n 资源 key，保持跨端稳定。
    pub message_key: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemResourcesResponse {
    /// 最终返回资源所使用的 locale。
    pub locale: String,
    /// 后续资源缺失时可使用的 fallback locale 链路。
    pub fallback_locales: Vec<String>,
    /// 系统资源版本，用于前端和移动端缓存失效。
    pub version: String,
    /// 按 namespace 分组后的资源。
    pub resources: BTreeMap<String, BTreeMap<String, String>>,
}

pub fn namespace_keys(namespace: &str) -> Option<&'static [SystemResourceKey]> {
    match namespace {
        "backend" => Some(BACKEND_KEYS),
        "common" => Some(COMMON_KEYS),
        "menu" => Some(MENU_KEYS),
        "form" => Some(FORM_KEYS),
        _ => None,
    }
}

pub fn default_namespaces(platform: &str) -> Option<&'static [&'static str]> {
    match platform {
        "backend" => Some(&["backend"]),
        "frontend" | "mobile" => Some(&["common", "menu"]),
        _ => None,
    }
}

const BACKEND_KEYS: &[SystemResourceKey] = &[
    SystemResourceKey {
        output_key: "paramInvalid",
        message_key: "error.param_invalid",
    },
    SystemResourceKey {
        output_key: "resourceNotFound",
        message_key: "error.resource_not_found",
    },
    SystemResourceKey {
        output_key: "systemError",
        message_key: "error.system_error",
    },
];

const COMMON_KEYS: &[SystemResourceKey] = &[
    SystemResourceKey {
        output_key: "confirm",
        message_key: "common.confirm",
    },
    SystemResourceKey {
        output_key: "cancel",
        message_key: "common.cancel",
    },
    SystemResourceKey {
        output_key: "ok",
        message_key: "common.ok",
    },
    SystemResourceKey {
        output_key: "operationSuccess",
        message_key: "common.operation_success",
    },
];

const MENU_KEYS: &[SystemResourceKey] = &[
    SystemResourceKey {
        output_key: "systemSettings",
        message_key: "menu.system_settings",
    },
    SystemResourceKey {
        output_key: "workspace",
        message_key: "menu.workspace",
    },
];

const FORM_KEYS: &[SystemResourceKey] = &[
    SystemResourceKey {
        output_key: "validationRequired",
        message_key: "form.validation.required",
    },
    SystemResourceKey {
        output_key: "validationMaxLength",
        message_key: "form.validation.max_length",
    },
];
