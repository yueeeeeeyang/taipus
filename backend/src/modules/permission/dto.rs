//! 权限模块 HTTP DTO。
//!
//! DTO 只表达接口契约，字段使用 camelCase；拼音、审计字段和权限版本由后端生成和维护。

use serde::{Deserialize, Serialize};

use crate::response::page::PageQuery;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionQuery {
    /// 调用方读取到的数据版本号。
    pub version: i64,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePageQuery {
    #[serde(flatten)]
    pub page: PageQuery,
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub platform: Option<String>,
    pub app_id: Option<String>,
    pub menu_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApplicationRequest {
    pub app_code: String,
    pub name: String,
    pub platform: String,
    pub home_path: Option<String>,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApplicationRequest {
    pub version: i64,
    pub app_code: String,
    pub name: String,
    pub platform: String,
    pub home_path: Option<String>,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMenuRequest {
    pub app_id: String,
    pub parent_id: Option<String>,
    pub menu_code: String,
    pub name: String,
    pub platform: String,
    pub route_path: String,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub visible: bool,
    pub keep_alive: bool,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMenuRequest {
    pub version: i64,
    pub app_id: String,
    pub parent_id: Option<String>,
    pub menu_code: String,
    pub name: String,
    pub platform: String,
    pub route_path: String,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub visible: bool,
    pub keep_alive: bool,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateButtonRequest {
    pub app_id: String,
    pub menu_id: String,
    pub button_code: String,
    pub name: String,
    pub action_key: String,
    pub button_type: String,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateButtonRequest {
    pub version: i64,
    pub app_id: String,
    pub menu_id: String,
    pub button_code: String,
    pub name: String,
    pub action_key: String,
    pub button_type: String,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiRequest {
    pub app_id: Option<String>,
    pub api_code: String,
    pub name: String,
    pub http_method: String,
    pub path_pattern: String,
    pub related_menu_id: Option<String>,
    pub related_button_id: Option<String>,
    pub public_access: bool,
    pub auth_required: bool,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiRequest {
    pub version: i64,
    pub app_id: Option<String>,
    pub api_code: String,
    pub name: String,
    pub http_method: String,
    pub path_pattern: String,
    pub related_menu_id: Option<String>,
    pub related_button_id: Option<String>,
    pub public_access: bool,
    pub auth_required: bool,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
    pub role_code: String,
    pub name: String,
    pub role_type: String,
    pub status: String,
    pub sort_no: i64,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
    pub version: i64,
    pub role_code: String,
    pub name: String,
    pub role_type: String,
    pub status: String,
    pub sort_no: i64,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RolePageQuery {
    #[serde(flatten)]
    pub page: PageQuery,
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub role_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRoleParentsRequest {
    pub version: i64,
    pub parent_role_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionGrantItem {
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRolePermissionsRequest {
    pub version: i64,
    pub permissions: Vec<PermissionGrantItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGrantsQuery {
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetResourceGrantsRequest {
    pub resource_type: String,
    pub resource_id: String,
    pub role_ids: Vec<String>,
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAccountRolesRequest {
    pub version: Option<i64>,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MeResourceQuery {
    pub platform: Option<String>,
    pub menu_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSummary {
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGrantSummary {
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionVersionResponse {
    pub tenant_id: String,
    pub version_no: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRoleSummary {
    pub account_id: String,
    pub role_id: String,
    pub status: String,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleTreeItem {
    pub id: String,
    pub role_code: String,
    pub name: String,
    pub role_type: String,
    pub status: String,
    pub sort_no: i64,
    pub version: i64,
    pub parent_role_ids: Vec<String>,
}
