//! 权限模块业务服务。
//!
//! service 负责权限动作白名单、租户上下文、角色授权合并、权限缓存和 API 鉴权判断。

use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

use crate::{
    context::request_context::RequestContext,
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::permission::{
        dto::*,
        model::{
            GrantEffect, PermissionApi, PermissionButton, PermissionMenu, PermissionResourceType,
            PermissionRole, PermissionStatus,
        },
        repository::{
            ApiWrite, ApplicationWrite, ButtonWrite, GrantWrite, MenuWrite, PermissionRepository,
            RoleWrite,
        },
    },
    response::page::PageResult,
    utils::{id::generate_business_id, pinyin::to_pinyin_text, time::now_utc},
};

const SYSTEM_OPERATOR: &str = "system";
const SUBJECT_ROLE: &str = "role";

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct PermissionKey {
    resource_type: String,
    resource_id: String,
    action: String,
}

#[derive(Debug, Clone)]
struct CachedPermissions {
    version_no: i64,
    allowed: HashSet<PermissionKey>,
    denied: HashSet<PermissionKey>,
}

static PERMISSION_CACHE: OnceLock<Mutex<HashMap<String, CachedPermissions>>> = OnceLock::new();

pub struct PermissionService;

impl PermissionService {
    pub async fn create_application(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateApplicationRequest,
    ) -> AppResult<crate::modules::permission::model::PermissionApplication> {
        validate_status(&request.status)?;
        validate_required(&request.app_code, "appCode")?;
        validate_required(&request.name, "name")?;
        let pinyin = to_pinyin_text(&request.name);
        let tenant_id = ctx.tenant_id.clone();
        let data = ApplicationWrite {
            id: generate_business_id(),
            tenant_id,
            app_code: request.app_code.trim().to_string(),
            name: request.name.trim().to_string(),
            name_full_pinyin: pinyin.full,
            name_simple_pinyin: pinyin.simple,
            platform: request.platform.trim().to_string(),
            home_path: trim_optional(request.home_path),
            icon: trim_optional(request.icon),
            sort_no: request.sort_no,
            status: request.status,
            remark: trim_optional(request.remark),
            operator: operator(ctx),
            now: now_utc(),
        };
        PermissionRepository::insert_application(pool, &data).await?;
        bump(pool, ctx, "新增应用资源").await?;
        Self::get_application(pool, &data.id).await
    }

    pub async fn get_application(
        pool: &DatabasePool,
        id: &str,
    ) -> AppResult<crate::modules::permission::model::PermissionApplication> {
        PermissionRepository::get_application(pool, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("应用资源不存在或已删除"))
    }

    pub async fn page_applications(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: ResourcePageQuery,
    ) -> AppResult<PageResult<crate::modules::permission::model::PermissionApplication>> {
        let tenant_id = require_tenant(ctx)?;
        let page = query.page.validate_and_normalize()?;
        let (records, total) =
            PermissionRepository::page_applications(pool, &tenant_id, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn create_menu(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateMenuRequest,
    ) -> AppResult<PermissionMenu> {
        validate_status(&request.status)?;
        validate_required(&request.menu_code, "menuCode")?;
        validate_required(&request.name, "name")?;
        let pinyin = to_pinyin_text(&request.name);
        let data = MenuWrite {
            id: generate_business_id(),
            tenant_id: ctx.tenant_id.clone(),
            app_id: request.app_id,
            parent_id: trim_optional(request.parent_id),
            menu_code: request.menu_code.trim().to_string(),
            name: request.name.trim().to_string(),
            name_full_pinyin: pinyin.full,
            name_simple_pinyin: pinyin.simple,
            platform: request.platform,
            route_path: request.route_path,
            component: trim_optional(request.component),
            icon: trim_optional(request.icon),
            visible: request.visible,
            keep_alive: request.keep_alive,
            sort_no: request.sort_no,
            status: request.status,
            remark: trim_optional(request.remark),
            operator: operator(ctx),
            now: now_utc(),
        };
        PermissionRepository::insert_menu(pool, &data).await?;
        bump(pool, ctx, "新增菜单资源").await?;
        Ok(
            PermissionRepository::page_menus(pool, &require_tenant(ctx)?)
                .await?
                .into_iter()
                .find(|menu| menu.id == data.id)
                .ok_or_else(|| AppError::resource_not_found("菜单资源不存在或已删除"))?,
        )
    }

    pub async fn menu_tree(
        pool: &DatabasePool,
        ctx: &RequestContext,
    ) -> AppResult<Vec<PermissionMenu>> {
        PermissionRepository::page_menus(pool, &require_tenant(ctx)?).await
    }

    pub async fn create_button(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateButtonRequest,
    ) -> AppResult<PermissionButton> {
        validate_status(&request.status)?;
        validate_required(&request.button_code, "buttonCode")?;
        validate_required(&request.name, "name")?;
        let data = ButtonWrite {
            id: generate_business_id(),
            tenant_id: ctx.tenant_id.clone(),
            app_id: request.app_id,
            menu_id: request.menu_id,
            button_code: request.button_code.trim().to_string(),
            name: request.name.trim().to_string(),
            action_key: request.action_key,
            button_type: request.button_type,
            icon: trim_optional(request.icon),
            sort_no: request.sort_no,
            status: request.status,
            remark: trim_optional(request.remark),
            operator: operator(ctx),
            now: now_utc(),
        };
        PermissionRepository::insert_button(pool, &data).await?;
        bump(pool, ctx, "新增按钮资源").await?;
        Ok(
            PermissionRepository::page_buttons(pool, &require_tenant(ctx)?, None)
                .await?
                .into_iter()
                .find(|button| button.id == data.id)
                .ok_or_else(|| AppError::resource_not_found("按钮资源不存在或已删除"))?,
        )
    }

    pub async fn page_buttons(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: MeResourceQuery,
    ) -> AppResult<Vec<PermissionButton>> {
        PermissionRepository::page_buttons(pool, &require_tenant(ctx)?, query.menu_id.as_deref())
            .await
    }

    pub async fn me_menus(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: MeResourceQuery,
    ) -> AppResult<Vec<PermissionMenu>> {
        let tenant_id = require_tenant(ctx)?;
        let permissions = effective_permissions(pool, ctx).await?;
        let menus = PermissionRepository::page_menus(pool, &tenant_id).await?;
        let by_id = menus
            .iter()
            .map(|menu| (menu.id.clone(), menu))
            .collect::<HashMap<_, _>>();
        let mut visible_ids = HashSet::new();

        for menu in &menus {
            if query
                .platform
                .as_deref()
                .is_some_and(|platform| menu.platform != platform)
            {
                continue;
            }
            if !is_resource_allowed(
                &permissions,
                PermissionResourceType::Menu,
                &menu.id,
                &["view", "manage"],
            ) {
                continue;
            }
            // 当前账号菜单需要补齐祖先节点，否则前端无法稳定渲染树形导航。
            let mut current = Some(menu);
            while let Some(item) = current {
                visible_ids.insert(item.id.clone());
                current = item
                    .parent_id
                    .as_ref()
                    .and_then(|parent_id| by_id.get(parent_id).copied());
            }
        }

        Ok(menus
            .into_iter()
            .filter(|menu| visible_ids.contains(&menu.id))
            .collect())
    }

    pub async fn me_buttons(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: MeResourceQuery,
    ) -> AppResult<Vec<PermissionButton>> {
        let permissions = effective_permissions(pool, ctx).await?;
        Ok(PermissionRepository::page_buttons(
            pool,
            &require_tenant(ctx)?,
            query.menu_id.as_deref(),
        )
        .await?
        .into_iter()
        .filter(|button| {
            is_resource_allowed(
                &permissions,
                PermissionResourceType::Button,
                &button.id,
                &["click", "manage"],
            )
        })
        .collect())
    }

    pub async fn create_api(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateApiRequest,
    ) -> AppResult<PermissionApi> {
        let data = api_write(generate_business_id(), ctx, request)?;
        PermissionRepository::insert_api(pool, &data).await?;
        bump(pool, ctx, "新增接口资源").await?;
        PermissionRepository::find_api_by_route(pool, &data.http_method, &data.normalized_path)
            .await?
            .ok_or_else(|| AppError::resource_not_found("接口资源不存在或已删除"))
    }

    pub async fn page_apis(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: ResourcePageQuery,
    ) -> AppResult<PageResult<PermissionApi>> {
        let tenant_id = require_tenant(ctx)?;
        let page = query.page.validate_and_normalize()?;
        let (records, total) = PermissionRepository::page_apis(pool, &tenant_id, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn import_apis(pool: &DatabasePool, ctx: &RequestContext) -> AppResult<()> {
        // 首版内置 API 由 migration 初始化；导入接口保留为后续扫描路由或导入清单的稳定入口。
        bump(pool, ctx, "同步接口资源").await
    }

    pub async fn create_role(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateRoleRequest,
    ) -> AppResult<PermissionRole> {
        validate_status(&request.status)?;
        validate_required(&request.role_code, "roleCode")?;
        validate_required(&request.name, "name")?;
        let pinyin = to_pinyin_text(&request.name);
        let data = RoleWrite {
            id: generate_business_id(),
            tenant_id: require_tenant(ctx)?,
            role_code: request.role_code.trim().to_string(),
            name: request.name.trim().to_string(),
            name_full_pinyin: pinyin.full,
            name_simple_pinyin: pinyin.simple,
            role_type: request.role_type,
            status: request.status,
            sort_no: request.sort_no,
            remark: trim_optional(request.remark),
            operator: operator(ctx),
            now: now_utc(),
        };
        PermissionRepository::insert_role(pool, &data).await?;
        bump(pool, ctx, "新增角色").await?;
        Self::get_role(pool, &data.id).await
    }

    pub async fn get_role(pool: &DatabasePool, id: &str) -> AppResult<PermissionRole> {
        PermissionRepository::get_role(pool, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("角色不存在或已删除"))
    }

    pub async fn page_roles(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: RolePageQuery,
    ) -> AppResult<PageResult<PermissionRole>> {
        let page = query.page.validate_and_normalize()?;
        let (records, total) =
            PermissionRepository::page_roles(pool, &require_tenant(ctx)?, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn role_tree(
        pool: &DatabasePool,
        ctx: &RequestContext,
    ) -> AppResult<Vec<RoleTreeItem>> {
        let tenant_id = require_tenant(ctx)?;
        let roles = PermissionRepository::list_roles(pool, &tenant_id).await?;
        let mut parent_map: HashMap<String, Vec<String>> = HashMap::new();
        for (child_id, parent_id) in
            PermissionRepository::list_role_parent_pairs(pool, &tenant_id).await?
        {
            parent_map.entry(child_id).or_default().push(parent_id);
        }
        Ok(roles
            .into_iter()
            .map(|role| RoleTreeItem {
                id: role.id.clone(),
                role_code: role.role_code,
                name: role.name,
                role_type: role.role_type,
                status: role.status,
                sort_no: role.sort_no,
                version: role.version,
                parent_role_ids: parent_map.remove(&role.id).unwrap_or_default(),
            })
            .collect())
    }

    pub async fn set_role_permissions(
        pool: &DatabasePool,
        ctx: &RequestContext,
        role_id: &str,
        request: SetRolePermissionsRequest,
    ) -> AppResult<Vec<PermissionSummary>> {
        let role_version = PermissionRepository::role_version(pool, role_id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("角色不存在或已删除"))?;
        if role_version != request.version {
            return Err(AppError::conflict("角色权限已被修改，请刷新后重试"));
        }
        let tenant_id = require_tenant(ctx)?;
        let mut grants = Vec::with_capacity(request.permissions.len());
        for item in request.permissions {
            let resource_type = PermissionResourceType::try_from(item.resource_type.as_str())?;
            validate_action(resource_type, &item.action)?;
            validate_resource_reference(pool, &tenant_id, resource_type, &item.resource_id).await?;
            let effect = item
                .effect
                .as_deref()
                .unwrap_or(GrantEffect::Allow.as_str());
            GrantEffect::try_from(effect)?;
            grants.push(GrantWrite {
                id: generate_business_id(),
                tenant_id: tenant_id.clone(),
                subject_type: SUBJECT_ROLE.to_string(),
                subject_id: role_id.to_string(),
                resource_type: resource_type.as_str().to_string(),
                resource_id: item.resource_id,
                action: item.action,
                effect: effect.to_string(),
                operator: operator(ctx),
                now: now_utc(),
            });
        }
        PermissionRepository::replace_role_grants(
            pool,
            &tenant_id,
            role_id,
            request.version,
            &grants,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        bump(pool, ctx, "保存角色授权").await?;
        Self::role_permissions(pool, ctx, role_id).await
    }

    pub async fn set_role_parents(
        pool: &DatabasePool,
        ctx: &RequestContext,
        role_id: &str,
        request: SetRoleParentsRequest,
    ) -> AppResult<()> {
        let tenant_id = require_tenant(ctx)?;
        if !PermissionRepository::role_exists(pool, &tenant_id, role_id).await? {
            return Err(AppError::resource_not_found("角色不存在或已删除"));
        }
        let mut parent_ids = Vec::new();
        for parent_id in request.parent_role_ids {
            if parent_id == role_id {
                return Err(AppError::business_error("角色不能继承自身"));
            }
            if !PermissionRepository::role_exists(pool, &tenant_id, &parent_id).await? {
                return Err(AppError::resource_not_found("父角色不存在或已删除"));
            }
            if PermissionRepository::role_is_descendant(pool, &tenant_id, role_id, &parent_id)
                .await?
            {
                return Err(AppError::business_error("角色继承不能形成循环"));
            }
            if !parent_ids.contains(&parent_id) {
                parent_ids.push(parent_id);
            }
        }
        PermissionRepository::replace_role_parents(
            pool,
            &tenant_id,
            role_id,
            request.version,
            &parent_ids,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        bump(pool, ctx, "保存角色父角色").await
    }

    pub async fn resource_grants(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: ResourceGrantsQuery,
    ) -> AppResult<Vec<ResourceGrantSummary>> {
        let tenant_id = require_tenant(ctx)?;
        let resource_type = PermissionResourceType::try_from(query.resource_type.as_str())?;
        validate_resource_reference(pool, &tenant_id, resource_type, &query.resource_id).await?;
        Ok(PermissionRepository::list_resource_grants(
            pool,
            &tenant_id,
            resource_type.as_str(),
            &query.resource_id,
        )
        .await?
        .into_iter()
        .map(|grant| ResourceGrantSummary {
            subject_type: grant.subject_type,
            subject_id: grant.subject_id,
            resource_type: grant.resource_type,
            resource_id: grant.resource_id,
            action: grant.action,
            effect: grant.effect,
        })
        .collect())
    }

    pub async fn set_resource_grants(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: SetResourceGrantsRequest,
    ) -> AppResult<Vec<ResourceGrantSummary>> {
        let tenant_id = require_tenant(ctx)?;
        let resource_type = PermissionResourceType::try_from(request.resource_type.as_str())?;
        validate_action(resource_type, &request.action)?;
        validate_resource_reference(pool, &tenant_id, resource_type, &request.resource_id).await?;
        let mut grants = Vec::new();
        for role_id in request.role_ids {
            if !PermissionRepository::role_exists(pool, &tenant_id, &role_id).await? {
                return Err(AppError::resource_not_found("授权角色不存在或已删除"));
            }
            grants.push(GrantWrite {
                id: generate_business_id(),
                tenant_id: tenant_id.clone(),
                subject_type: SUBJECT_ROLE.to_string(),
                subject_id: role_id,
                resource_type: resource_type.as_str().to_string(),
                resource_id: request.resource_id.clone(),
                action: request.action.clone(),
                effect: GrantEffect::Allow.as_str().to_string(),
                operator: operator(ctx),
                now: now_utc(),
            });
        }
        PermissionRepository::replace_resource_role_grants(
            pool,
            &tenant_id,
            resource_type.as_str(),
            &request.resource_id,
            &request.action,
            &grants,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        bump(pool, ctx, "保存资源授权").await?;
        Self::resource_grants(
            pool,
            ctx,
            ResourceGrantsQuery {
                resource_type: resource_type.as_str().to_string(),
                resource_id: request.resource_id,
            },
        )
        .await
    }

    pub async fn account_roles(
        pool: &DatabasePool,
        ctx: &RequestContext,
        account_id: &str,
    ) -> AppResult<Vec<AccountRoleSummary>> {
        let tenant_id = require_tenant(ctx)?;
        Ok(
            PermissionRepository::list_account_roles(pool, &tenant_id, account_id)
                .await?
                .into_iter()
                .map(|item| AccountRoleSummary {
                    account_id: item.account_id,
                    role_id: item.role_id,
                    status: item.status,
                    version: item.version,
                })
                .collect(),
        )
    }

    pub async fn set_account_roles(
        pool: &DatabasePool,
        ctx: &RequestContext,
        account_id: &str,
        request: SetAccountRolesRequest,
    ) -> AppResult<Vec<AccountRoleSummary>> {
        let tenant_id = require_tenant(ctx)?;
        if !PermissionRepository::account_exists_in_tenant(pool, &tenant_id, account_id).await? {
            return Err(AppError::resource_not_found("账号不存在或不属于当前租户"));
        }
        let mut role_ids = Vec::new();
        for role_id in request.role_ids {
            if !PermissionRepository::role_exists(pool, &tenant_id, &role_id).await? {
                return Err(AppError::resource_not_found("账号角色不存在或已删除"));
            }
            if !role_ids.contains(&role_id) {
                role_ids.push(role_id);
            }
        }
        PermissionRepository::replace_account_roles(
            pool,
            &tenant_id,
            account_id,
            request.version,
            &role_ids,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        bump(pool, ctx, "保存账号角色").await?;
        Self::account_roles(pool, ctx, account_id).await
    }

    pub async fn inherited_role_permissions(
        pool: &DatabasePool,
        ctx: &RequestContext,
        role_id: &str,
    ) -> AppResult<Vec<PermissionSummary>> {
        let tenant_id = require_tenant(ctx)?;
        if !PermissionRepository::role_exists(pool, &tenant_id, role_id).await? {
            return Err(AppError::resource_not_found("角色不存在或已删除"));
        }
        Ok(
            PermissionRepository::list_inherited_role_grants(pool, &tenant_id, role_id)
                .await?
                .into_iter()
                .map(|grant| PermissionSummary {
                    resource_type: grant.resource_type,
                    resource_id: grant.resource_id,
                    action: grant.action,
                    effect: grant.effect,
                })
                .collect(),
        )
    }

    pub async fn role_permissions(
        pool: &DatabasePool,
        ctx: &RequestContext,
        role_id: &str,
    ) -> AppResult<Vec<PermissionSummary>> {
        Ok(
            PermissionRepository::list_role_grants(pool, &require_tenant(ctx)?, role_id)
                .await?
                .into_iter()
                .map(|grant| PermissionSummary {
                    resource_type: grant.resource_type,
                    resource_id: grant.resource_id,
                    action: grant.action,
                    effect: grant.effect,
                })
                .collect(),
        )
    }

    pub async fn permission_version(
        pool: &DatabasePool,
        ctx: &RequestContext,
    ) -> AppResult<PermissionVersionResponse> {
        let tenant_id = require_tenant(ctx)?;
        let version_no = PermissionRepository::permission_version(pool, &tenant_id).await?;
        Ok(PermissionVersionResponse {
            tenant_id,
            version_no,
        })
    }

    pub async fn me_permissions(
        pool: &DatabasePool,
        ctx: &RequestContext,
    ) -> AppResult<Vec<PermissionSummary>> {
        let permissions = effective_permissions(pool, ctx).await?;
        Ok(permissions
            .allowed
            .into_iter()
            .filter(|item| !permissions.denied.contains(item))
            .map(|item| PermissionSummary {
                resource_type: item.resource_type,
                resource_id: item.resource_id,
                action: item.action,
                effect: "allow".to_string(),
            })
            .collect())
    }

    pub async fn authorize_api(
        pool: &DatabasePool,
        ctx: &RequestContext,
        method: &str,
        path: &str,
    ) -> AppResult<()> {
        let normalized_path = normalize_api_path(path);
        let api = find_api_for_request(pool, method, path, &normalized_path)
            .await?
            .ok_or_else(|| AppError::forbidden("接口资源未注册，拒绝访问"))?;
        if api.status != PermissionStatus::Enabled.as_str() {
            return Err(AppError::forbidden("接口资源已禁用"));
        }
        if api.public_access || !api.auth_required {
            return Ok(());
        }
        if !ctx.is_authenticated {
            return Err(AppError::unauthorized("未认证或登录已过期"));
        }
        let permissions = effective_permissions(pool, ctx).await?;
        let key = PermissionKey {
            resource_type: PermissionResourceType::Api.as_str().to_string(),
            resource_id: api.id,
            action: "call".to_string(),
        };
        if permissions.denied.contains(&key) {
            return Err(AppError::forbidden("无权限访问该接口"));
        }
        if permissions.allowed.contains(&key) {
            return Ok(());
        }
        Err(AppError::forbidden("无权限访问该接口"))
    }

    pub async fn is_public_api(pool: &DatabasePool, method: &str, path: &str) -> AppResult<bool> {
        let normalized_path = normalize_api_path(path);
        let Some(api) = find_api_for_request(pool, method, path, &normalized_path).await? else {
            return Ok(false);
        };
        Ok(api.status == PermissionStatus::Enabled.as_str()
            && (api.public_access || !api.auth_required))
    }
}

async fn effective_permissions(
    pool: &DatabasePool,
    ctx: &RequestContext,
) -> AppResult<CachedPermissions> {
    let tenant_id = require_tenant(ctx)?;
    let account_id = ctx
        .user_id
        .as_deref()
        .ok_or_else(|| AppError::unauthorized("未认证或登录已过期"))?;
    let version_no = PermissionRepository::permission_version(pool, &tenant_id).await?;
    let cache_key = format!("{tenant_id}#{account_id}");
    if let Some(cached) = PERMISSION_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("权限缓存锁不得中毒")
        .get(&cache_key)
        .filter(|cached| cached.version_no == version_no)
        .cloned()
    {
        return Ok(cached);
    }

    let mut allowed = HashSet::new();
    let mut denied = HashSet::new();
    for grant in
        PermissionRepository::list_effective_permissions(pool, &tenant_id, account_id).await?
    {
        let key = PermissionKey {
            resource_type: grant.resource_type,
            resource_id: grant.resource_id,
            action: grant.action,
        };
        if grant.effect == GrantEffect::Deny.as_str() {
            denied.insert(key);
        } else {
            allowed.insert(key);
        }
    }
    let cached = CachedPermissions {
        version_no,
        allowed,
        denied,
    };
    PERMISSION_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("权限缓存锁不得中毒")
        .insert(cache_key, cached.clone());
    Ok(cached)
}

async fn find_api_for_request(
    pool: &DatabasePool,
    method: &str,
    raw_path: &str,
    normalized_path: &str,
) -> AppResult<Option<PermissionApi>> {
    if let Some(api) =
        PermissionRepository::find_api_by_route(pool, method, normalized_path).await?
    {
        return Ok(Some(api));
    }
    // 精确归一化未命中时，退回到已注册路径模板逐段匹配，支持短字符串业务主键。
    Ok(PermissionRepository::list_apis_by_method(pool, method)
        .await?
        .into_iter()
        .find(|api| path_pattern_matches(&api.normalized_path, raw_path)))
}

fn path_pattern_matches(pattern: &str, raw_path: &str) -> bool {
    let path = raw_path.split('?').next().unwrap_or(raw_path).trim();
    let pattern_parts = pattern
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let path_parts = path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    pattern_parts.len() == path_parts.len()
        && pattern_parts
            .iter()
            .zip(path_parts)
            .all(|(pattern, part)| pattern.starts_with('{') || *pattern == part)
}

fn is_resource_allowed(
    permissions: &CachedPermissions,
    resource_type: PermissionResourceType,
    resource_id: &str,
    actions: &[&str],
) -> bool {
    actions.iter().any(|action| {
        let key = PermissionKey {
            resource_type: resource_type.as_str().to_string(),
            resource_id: resource_id.to_string(),
            action: (*action).to_string(),
        };
        !permissions.denied.contains(&key) && permissions.allowed.contains(&key)
    })
}

async fn validate_resource_reference(
    pool: &DatabasePool,
    tenant_id: &str,
    resource_type: PermissionResourceType,
    resource_id: &str,
) -> AppResult<()> {
    if PermissionRepository::resource_exists(pool, tenant_id, resource_type.as_str(), resource_id)
        .await?
    {
        Ok(())
    } else {
        Err(AppError::resource_not_found("授权资源不存在或已删除"))
    }
}

fn api_write(id: String, ctx: &RequestContext, request: CreateApiRequest) -> AppResult<ApiWrite> {
    validate_status(&request.status)?;
    validate_required(&request.api_code, "apiCode")?;
    validate_required(&request.name, "name")?;
    validate_required(&request.http_method, "httpMethod")?;
    validate_required(&request.path_pattern, "pathPattern")?;
    Ok(ApiWrite {
        id,
        tenant_id: ctx.tenant_id.clone(),
        app_id: request.app_id,
        api_code: request.api_code.trim().to_string(),
        name: request.name.trim().to_string(),
        http_method: request.http_method.trim().to_ascii_uppercase(),
        normalized_path: normalize_api_path(&request.path_pattern),
        path_pattern: request.path_pattern.trim().to_string(),
        related_menu_id: trim_optional(request.related_menu_id),
        related_button_id: trim_optional(request.related_button_id),
        public_access: request.public_access,
        auth_required: request.auth_required,
        status: request.status,
        remark: trim_optional(request.remark),
        operator: operator(ctx),
        now: now_utc(),
    })
}

pub fn normalize_api_path(path: &str) -> String {
    let path = path.split('?').next().unwrap_or(path).trim();
    if path.is_empty() {
        return "/".to_string();
    }
    let normalized = path
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| {
            if looks_like_path_param(part) {
                "{id}".to_string()
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("/{normalized}")
}

fn looks_like_path_param(value: &str) -> bool {
    value.starts_with('{')
        || value.parse::<i64>().is_ok()
        || (value.len() >= 24
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
}

fn validate_action(resource_type: PermissionResourceType, action: &str) -> AppResult<()> {
    let allowed = match resource_type {
        PermissionResourceType::Application => ["access", "manage"].as_slice(),
        PermissionResourceType::Menu => ["view", "manage"].as_slice(),
        PermissionResourceType::Button => ["click", "manage"].as_slice(),
        PermissionResourceType::Api => ["call", "manage"].as_slice(),
    };
    if allowed.contains(&action) {
        Ok(())
    } else {
        Err(AppError::param_invalid("资源动作不符合资源类型白名单"))
    }
}

fn validate_status(status: &str) -> AppResult<()> {
    PermissionStatus::try_from(status).map(|_| ())
}

fn validate_required(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::param_invalid(format!("{field} 不能为空")));
    }
    Ok(())
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn require_tenant(ctx: &RequestContext) -> AppResult<String> {
    ctx.tenant_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::param_invalid("当前请求缺少租户上下文"))
}

fn operator(ctx: &RequestContext) -> String {
    ctx.user_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| SYSTEM_OPERATOR.to_string())
}

async fn bump(pool: &DatabasePool, ctx: &RequestContext, reason: &str) -> AppResult<()> {
    PermissionRepository::bump_permission_version(
        pool,
        &require_tenant(ctx)?,
        reason,
        &operator(ctx),
        now_utc(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{normalize_api_path, path_pattern_matches, validate_action};
    use crate::modules::permission::model::PermissionResourceType;

    #[test]
    fn api_path_normalization_replaces_runtime_ids() {
        assert_eq!(
            normalize_api_path("/api/v1/hrm/users/123?x=1"),
            "/api/v1/hrm/users/{id}"
        );
    }

    #[test]
    fn api_path_pattern_matches_short_string_params() {
        assert!(path_pattern_matches(
            "/api/v1/i18n/business_translations/{id}/{id}",
            "/api/v1/i18n/business_translations/form_definition/title"
        ));
    }

    #[test]
    fn action_whitelist_rejects_wrong_resource_action() {
        assert!(validate_action(PermissionResourceType::Api, "call").is_ok());
        assert!(validate_action(PermissionResourceType::Api, "click").is_err());
    }
}
