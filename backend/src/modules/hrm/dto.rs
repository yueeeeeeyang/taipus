//! HRM 接口 DTO。
//!
//! DTO 只表达 HTTP 契约和内部 service 读模型，字段统一使用 camelCase。写入请求不得包含拼音字段，
//! 拼音必须由 service 根据名称生成，避免客户端伪造搜索字段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::response::page::PageQuery;

/// HRM 逻辑删除请求 query。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionQuery {
    /// 调用方读取到的数据版本号，用于乐观锁删除。
    pub version: i64,
}

/// 用户新增请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    /// 工号，租户内未删除数据唯一。
    pub employee_no: String,
    /// 用户姓名。
    pub name: String,
    /// 手机号，首版不作为登录账号。
    pub mobile: Option<String>,
    /// 邮箱，首版不作为登录账号。
    pub email: Option<String>,
    /// 用户排序号。
    pub sort_no: i64,
    /// 用户状态，允许 `enabled` 或 `disabled`。
    pub status: String,
}

/// 用户修改请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    /// 乐观锁版本号。
    pub version: i64,
    /// 工号，租户内未删除数据唯一。
    pub employee_no: String,
    /// 用户姓名。
    pub name: String,
    /// 手机号。
    pub mobile: Option<String>,
    /// 邮箱。
    pub email: Option<String>,
    /// 用户排序号。
    pub sort_no: i64,
    /// 用户状态。
    pub status: String,
}

/// 用户分页查询。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserPageQuery {
    /// 通用分页参数。
    #[serde(flatten)]
    pub page: PageQuery,
    /// 工号模糊查询。
    pub employee_no: Option<String>,
    /// 名称、全拼、简拼模糊查询。
    pub name: Option<String>,
    /// 手机号模糊查询。
    pub mobile: Option<String>,
    /// 邮箱模糊查询。
    pub email: Option<String>,
    /// 排序号精确查询。
    pub sort_no: Option<i64>,
    /// 状态精确查询。
    pub status: Option<String>,
    /// 创建时间起点。
    pub created_time_start: Option<DateTime<Utc>>,
    /// 创建时间终点。
    pub created_time_end: Option<DateTime<Utc>>,
    /// 更新时间起点。
    pub updated_time_start: Option<DateTime<Utc>>,
    /// 更新时间终点。
    pub updated_time_end: Option<DateTime<Utc>>,
}

/// 组织新增请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrgRequest {
    /// 父组织 ID，根分部为空。
    pub parent_id: Option<String>,
    /// 组织编码，租户内未删除数据唯一。
    pub org_code: String,
    /// 组织名称。
    pub name: String,
    /// 组织类型，允许 `branch` 或 `department`。
    pub org_type: String,
    /// 同级排序号。
    pub sort_no: i64,
    /// 组织状态。
    pub status: String,
}

/// 组织修改请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrgRequest {
    /// 乐观锁版本号。
    pub version: i64,
    /// 父组织 ID，根分部为空。
    pub parent_id: Option<String>,
    /// 组织编码。
    pub org_code: String,
    /// 组织名称。
    pub name: String,
    /// 组织类型。
    pub org_type: String,
    /// 同级排序号。
    pub sort_no: i64,
    /// 组织状态。
    pub status: String,
}

/// 组织分页查询。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrgPageQuery {
    /// 通用分页参数。
    #[serde(flatten)]
    pub page: PageQuery,
    /// 父组织 ID 精确查询。
    pub parent_id: Option<String>,
    /// 组织编码模糊查询。
    pub org_code: Option<String>,
    /// 名称、全拼、简拼模糊查询。
    pub name: Option<String>,
    /// 组织类型精确查询。
    pub org_type: Option<String>,
    /// 组织状态精确查询。
    pub status: Option<String>,
    /// 创建时间起点。
    pub created_time_start: Option<DateTime<Utc>>,
    /// 创建时间终点。
    pub created_time_end: Option<DateTime<Utc>>,
}

/// 岗位新增请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostRequest {
    /// 岗位编码，租户内未删除数据唯一。
    pub post_code: String,
    /// 岗位名称。
    pub name: String,
    /// 排序号。
    pub sort_no: i64,
    /// 岗位状态。
    pub status: String,
}

/// 岗位修改请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePostRequest {
    /// 乐观锁版本号。
    pub version: i64,
    /// 岗位编码。
    pub post_code: String,
    /// 岗位名称。
    pub name: String,
    /// 排序号。
    pub sort_no: i64,
    /// 岗位状态。
    pub status: String,
}

/// 岗位分页查询。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PostPageQuery {
    /// 通用分页参数。
    #[serde(flatten)]
    pub page: PageQuery,
    /// 岗位编码模糊查询。
    pub post_code: Option<String>,
    /// 名称、全拼、简拼模糊查询。
    pub name: Option<String>,
    /// 岗位状态精确查询。
    pub status: Option<String>,
    /// 创建时间起点。
    pub created_time_start: Option<DateTime<Utc>>,
    /// 创建时间终点。
    pub created_time_end: Option<DateTime<Utc>>,
}

/// 任职关系新增请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserOrgPostRequest {
    /// 用户 ID。
    pub user_id: String,
    /// 组织 ID。
    pub org_id: String,
    /// 岗位 ID。
    pub post_id: String,
    /// 是否主组织关系。
    pub primary_org: bool,
    /// 是否主岗位关系。
    pub primary_post: bool,
    /// 关系排序号。
    pub sort_no: i64,
}

/// 任职关系修改请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserOrgPostRequest {
    /// 乐观锁版本号。
    pub version: i64,
    /// 用户 ID。
    pub user_id: String,
    /// 组织 ID。
    pub org_id: String,
    /// 岗位 ID。
    pub post_id: String,
    /// 是否主组织关系。
    pub primary_org: bool,
    /// 是否主岗位关系。
    pub primary_post: bool,
    /// 关系排序号。
    pub sort_no: i64,
}

/// 任职关系分页查询。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserOrgPostPageQuery {
    /// 通用分页参数。
    #[serde(flatten)]
    pub page: PageQuery,
    /// 用户 ID。
    pub user_id: Option<String>,
    /// 组织 ID。
    pub org_id: Option<String>,
    /// 岗位 ID。
    pub post_id: Option<String>,
    /// 是否主组织。
    pub primary_org: Option<bool>,
    /// 是否主岗位。
    pub primary_post: Option<bool>,
    /// 创建时间起点。
    pub created_time_start: Option<DateTime<Utc>>,
    /// 创建时间终点。
    pub created_time_end: Option<DateTime<Utc>>,
}

/// 批量 ID 请求，供内部服务测试和后续模块复用。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchIdsRequest {
    /// 待查询 ID 列表，单次最多 500 个。
    pub ids: Vec<String>,
}

/// 组织树节点。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgTreeNode {
    /// 当前组织。
    #[serde(flatten)]
    pub org: crate::modules::hrm::model::HrmOrg,
    /// 下级组织列表。
    pub children: Vec<OrgTreeNode>,
}

/// 用户任职聚合响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAssignmentResponse {
    /// 任职关系。
    pub relation: crate::modules::hrm::model::HrmUserOrgPost,
    /// 关系所属组织。
    pub org: crate::modules::hrm::model::HrmOrg,
    /// 关系对应岗位。
    pub post: crate::modules::hrm::model::HrmPost,
}
