//! HRM 业务服务层。
//!
//! service 负责业务规则、租户边界、拼音生成、组织层级校验和引用检查。handler 不直接访问
//! repository，repository 也不判断业务规则，保证规则入口集中且便于后续接入权限模块。

use std::collections::{HashMap, HashSet};

use crate::{
    context::request_context::RequestContext,
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::hrm::{
        dto::{
            CreateOrgRequest, CreatePostRequest, CreateUserOrgPostRequest, CreateUserRequest,
            OrgPageQuery, OrgTreeNode, PostPageQuery, UpdateOrgRequest, UpdatePostRequest,
            UpdateUserOrgPostRequest, UpdateUserRequest, UserAssignmentResponse,
            UserOrgPostPageQuery, UserPageQuery,
        },
        model::{HrmOrg, HrmPost, HrmStatus, HrmUser, HrmUserOrgPost, OrgType},
        repository::{HrmRepository, OrgWrite, PostWrite, UserOrgPostWrite, UserWrite},
    },
    response::page::PageResult,
    utils::{id::generate_business_id, pinyin::to_pinyin_text, time::now_utc},
};

const SYSTEM_OPERATOR: &str = "system";
const MAX_BATCH_IDS: usize = 500;

pub struct HrmService;

impl HrmService {
    pub async fn create_user(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateUserRequest,
    ) -> AppResult<HrmUser> {
        let tenant_id = tenant_id(ctx)?;
        let data = user_write(generate_business_id(), &tenant_id, ctx, request)?;
        HrmRepository::insert_user(pool, &data)
            .await
            .map_err(map_write_error)?;
        Self::get_user(pool, ctx, &data.id).await
    }

    pub async fn update_user(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: UpdateUserRequest,
    ) -> AppResult<HrmUser> {
        let tenant_id = tenant_id(ctx)?;
        let data = user_write(
            id.to_string(),
            &tenant_id,
            ctx,
            CreateUserRequest {
                employee_no: request.employee_no,
                name: request.name,
                mobile: request.mobile,
                email: request.email,
                sort_no: request.sort_no,
                status: request.status,
            },
        )?;
        let affected = HrmRepository::update_user(pool, id, request.version, &data)
            .await
            .map_err(map_write_error)?;
        ensure_updated(affected)?;
        Self::get_user(pool, ctx, id).await
    }

    pub async fn get_user(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
    ) -> AppResult<HrmUser> {
        HrmRepository::get_user(pool, &tenant_id(ctx)?, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("用户不存在或已删除"))
    }

    pub async fn page_users(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: UserPageQuery,
    ) -> AppResult<PageResult<HrmUser>> {
        let page = query.page.validate_and_normalize()?;
        validate_status_filter(query.status.as_deref())?;
        let (records, total) =
            HrmRepository::page_users(pool, &tenant_id(ctx)?, &query, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn logical_delete_user(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<()> {
        let affected = HrmRepository::logical_delete(
            pool,
            "hrm_users",
            &tenant_id(ctx)?,
            id,
            version,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        ensure_updated(affected)
    }

    pub async fn physical_delete_user(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
    ) -> AppResult<()> {
        let tenant_id = tenant_id(ctx)?;
        if HrmRepository::count_relations_by_user(pool, &tenant_id, id).await? > 0 {
            return Err(AppError::business_error(
                "用户仍存在有效任职关系，不能物理删除",
            ));
        }
        ensure_deleted(HrmRepository::physical_delete(pool, "hrm_users", &tenant_id, id).await?)
    }

    pub async fn create_org(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateOrgRequest,
    ) -> AppResult<HrmOrg> {
        let tenant_id = tenant_id(ctx)?;
        validate_org_tree_for_write(
            pool,
            &tenant_id,
            None,
            request.parent_id.as_deref(),
            &request.org_type,
        )
        .await?;
        let data = org_write(generate_business_id(), &tenant_id, ctx, request)?;
        HrmRepository::insert_org(pool, &data)
            .await
            .map_err(map_write_error)?;
        Self::get_org(pool, ctx, &data.id).await
    }

    pub async fn update_org(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: UpdateOrgRequest,
    ) -> AppResult<HrmOrg> {
        let tenant_id = tenant_id(ctx)?;
        validate_org_tree_for_write(
            pool,
            &tenant_id,
            Some(id),
            request.parent_id.as_deref(),
            &request.org_type,
        )
        .await?;
        let data = org_write(
            id.to_string(),
            &tenant_id,
            ctx,
            CreateOrgRequest {
                parent_id: request.parent_id,
                org_code: request.org_code,
                name: request.name,
                org_type: request.org_type,
                sort_no: request.sort_no,
                status: request.status,
            },
        )?;
        let affected = HrmRepository::update_org(pool, id, request.version, &data)
            .await
            .map_err(map_write_error)?;
        ensure_updated(affected)?;
        Self::get_org(pool, ctx, id).await
    }

    pub async fn get_org(pool: &DatabasePool, ctx: &RequestContext, id: &str) -> AppResult<HrmOrg> {
        HrmRepository::get_org(pool, &tenant_id(ctx)?, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("组织不存在或已删除"))
    }

    pub async fn page_orgs(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: OrgPageQuery,
    ) -> AppResult<PageResult<HrmOrg>> {
        let page = query.page.validate_and_normalize()?;
        validate_status_filter(query.status.as_deref())?;
        if let Some(org_type) = query.org_type.as_deref() {
            OrgType::try_from(org_type)?;
        }
        let (records, total) =
            HrmRepository::page_orgs(pool, &tenant_id(ctx)?, &query, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn org_tree(
        pool: &DatabasePool,
        ctx: &RequestContext,
    ) -> AppResult<Vec<OrgTreeNode>> {
        let orgs = HrmRepository::list_orgs(pool, &tenant_id(ctx)?).await?;
        Ok(build_org_tree(orgs, None))
    }

    pub async fn logical_delete_org(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<()> {
        let tenant_id = tenant_id(ctx)?;
        if HrmRepository::count_active_children(pool, &tenant_id, id).await? > 0 {
            return Err(AppError::business_error("组织仍存在未删除子组织，不能删除"));
        }
        if HrmRepository::count_relations_by_org(pool, &tenant_id, id).await? > 0 {
            return Err(AppError::business_error("组织仍存在有效任职关系，不能删除"));
        }
        let affected = HrmRepository::logical_delete(
            pool,
            "hrm_orgs",
            &tenant_id,
            id,
            version,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        ensure_updated(affected)
    }

    pub async fn physical_delete_org(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
    ) -> AppResult<()> {
        let tenant_id = tenant_id(ctx)?;
        if HrmRepository::count_active_children(pool, &tenant_id, id).await? > 0 {
            return Err(AppError::business_error(
                "组织仍存在未删除子组织，不能物理删除",
            ));
        }
        if HrmRepository::count_relations_by_org(pool, &tenant_id, id).await? > 0 {
            return Err(AppError::business_error(
                "组织仍存在有效任职关系，不能物理删除",
            ));
        }
        ensure_deleted(HrmRepository::physical_delete(pool, "hrm_orgs", &tenant_id, id).await?)
    }

    pub async fn create_post(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreatePostRequest,
    ) -> AppResult<HrmPost> {
        let tenant_id = tenant_id(ctx)?;
        let data = post_write(generate_business_id(), &tenant_id, ctx, request)?;
        HrmRepository::insert_post(pool, &data)
            .await
            .map_err(map_write_error)?;
        Self::get_post(pool, ctx, &data.id).await
    }

    pub async fn update_post(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: UpdatePostRequest,
    ) -> AppResult<HrmPost> {
        let tenant_id = tenant_id(ctx)?;
        let data = post_write(
            id.to_string(),
            &tenant_id,
            ctx,
            CreatePostRequest {
                post_code: request.post_code,
                name: request.name,
                sort_no: request.sort_no,
                status: request.status,
            },
        )?;
        let affected = HrmRepository::update_post(pool, id, request.version, &data)
            .await
            .map_err(map_write_error)?;
        ensure_updated(affected)?;
        Self::get_post(pool, ctx, id).await
    }

    pub async fn get_post(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
    ) -> AppResult<HrmPost> {
        HrmRepository::get_post(pool, &tenant_id(ctx)?, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("岗位不存在或已删除"))
    }

    pub async fn page_posts(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: PostPageQuery,
    ) -> AppResult<PageResult<HrmPost>> {
        let page = query.page.validate_and_normalize()?;
        validate_status_filter(query.status.as_deref())?;
        let (records, total) =
            HrmRepository::page_posts(pool, &tenant_id(ctx)?, &query, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn logical_delete_post(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<()> {
        let tenant_id = tenant_id(ctx)?;
        if HrmRepository::count_relations_by_post(pool, &tenant_id, id).await? > 0 {
            return Err(AppError::business_error("岗位仍存在有效任职关系，不能删除"));
        }
        let affected = HrmRepository::logical_delete(
            pool,
            "hrm_posts",
            &tenant_id,
            id,
            version,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        ensure_updated(affected)
    }

    pub async fn physical_delete_post(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
    ) -> AppResult<()> {
        let tenant_id = tenant_id(ctx)?;
        if HrmRepository::count_relations_by_post(pool, &tenant_id, id).await? > 0 {
            return Err(AppError::business_error(
                "岗位仍存在有效任职关系，不能物理删除",
            ));
        }
        ensure_deleted(HrmRepository::physical_delete(pool, "hrm_posts", &tenant_id, id).await?)
    }

    pub async fn create_relation(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateUserOrgPostRequest,
    ) -> AppResult<HrmUserOrgPost> {
        let tenant_id = tenant_id(ctx)?;
        ensure_user_active(pool, &tenant_id, &request.user_id).await?;
        ensure_org_active(pool, &tenant_id, &request.org_id).await?;
        ensure_post_active(pool, &tenant_id, &request.post_id).await?;
        let data = relation_write(generate_business_id(), &tenant_id, ctx, request);
        HrmRepository::insert_relation_with_primary_clear(pool, &data)
            .await
            .map_err(map_write_error)?;
        Self::get_relation(pool, ctx, &data.id).await
    }

    pub async fn update_relation(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: UpdateUserOrgPostRequest,
    ) -> AppResult<HrmUserOrgPost> {
        let tenant_id = tenant_id(ctx)?;
        ensure_user_active(pool, &tenant_id, &request.user_id).await?;
        ensure_org_active(pool, &tenant_id, &request.org_id).await?;
        ensure_post_active(pool, &tenant_id, &request.post_id).await?;
        let data = relation_write(
            id.to_string(),
            &tenant_id,
            ctx,
            CreateUserOrgPostRequest {
                user_id: request.user_id,
                org_id: request.org_id,
                post_id: request.post_id,
                primary_org: request.primary_org,
                primary_post: request.primary_post,
                sort_no: request.sort_no,
            },
        );
        let affected =
            HrmRepository::update_relation_with_primary_clear(pool, id, request.version, &data)
                .await
                .map_err(map_write_error)?;
        ensure_updated(affected)?;
        Self::get_relation(pool, ctx, id).await
    }

    pub async fn get_relation(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
    ) -> AppResult<HrmUserOrgPost> {
        HrmRepository::get_relation(pool, &tenant_id(ctx)?, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("任职关系不存在或已删除"))
    }

    pub async fn page_relations(
        pool: &DatabasePool,
        ctx: &RequestContext,
        query: UserOrgPostPageQuery,
    ) -> AppResult<PageResult<HrmUserOrgPost>> {
        let page = query.page.validate_and_normalize()?;
        let (records, total) =
            HrmRepository::page_relations(pool, &tenant_id(ctx)?, &query, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn logical_delete_relation(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<()> {
        let affected = HrmRepository::logical_delete(
            pool,
            "hrm_user_org_posts",
            &tenant_id(ctx)?,
            id,
            version,
            &operator(ctx),
            now_utc(),
        )
        .await?;
        ensure_updated(affected)
    }

    pub async fn physical_delete_relation(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
    ) -> AppResult<()> {
        ensure_deleted(
            HrmRepository::physical_delete(pool, "hrm_user_org_posts", &tenant_id(ctx)?, id)
                .await?,
        )
    }

    pub async fn get_users_by_ids(
        pool: &DatabasePool,
        ctx: &RequestContext,
        ids: &[String],
    ) -> AppResult<Vec<HrmUser>> {
        validate_batch_ids(ids)?;
        let users = HrmRepository::list_users_by_ids(pool, &tenant_id(ctx)?, ids).await?;
        let by_id: HashMap<String, HrmUser> = users
            .into_iter()
            .map(|user| (user.id.clone(), user))
            .collect();
        // 批量 SQL 不保证返回顺序，这里按调用方传入 ID 顺序重排，保证内部服务契约稳定。
        Ok(ids.iter().filter_map(|id| by_id.get(id).cloned()).collect())
    }

    pub async fn user_assignments(
        pool: &DatabasePool,
        ctx: &RequestContext,
        user_id: &str,
    ) -> AppResult<Vec<UserAssignmentResponse>> {
        let tenant_id = tenant_id(ctx)?;
        let query = UserOrgPostPageQuery {
            user_id: Some(user_id.to_string()),
            ..Default::default()
        };
        let page = crate::response::page::NormalizedPageQuery {
            page_no: 1,
            page_size: 500,
            offset: 0,
        };
        let (relations, _) = HrmRepository::page_relations(pool, &tenant_id, &query, page).await?;
        let mut result = Vec::with_capacity(relations.len());
        for relation in relations {
            let org = HrmRepository::get_org(pool, &tenant_id, &relation.org_id)
                .await?
                .ok_or_else(|| AppError::resource_not_found("任职关系关联组织不存在"))?;
            let post = HrmRepository::get_post(pool, &tenant_id, &relation.post_id)
                .await?
                .ok_or_else(|| AppError::resource_not_found("任职关系关联岗位不存在"))?;
            result.push(UserAssignmentResponse {
                relation,
                org,
                post,
            });
        }
        Ok(result)
    }
}

fn user_write(
    id: String,
    tenant_id: &str,
    ctx: &RequestContext,
    request: CreateUserRequest,
) -> AppResult<UserWrite> {
    validate_required(&request.employee_no, "employeeNo")?;
    validate_required(&request.name, "name")?;
    let status = HrmStatus::try_from(request.status.as_str())?;
    let pinyin = to_pinyin_text(&request.name);
    Ok(UserWrite {
        id,
        tenant_id: tenant_id.to_string(),
        employee_no: request.employee_no.trim().to_string(),
        name: request.name.trim().to_string(),
        name_full_pinyin: pinyin.full,
        name_simple_pinyin: pinyin.simple,
        mobile: trim_optional(request.mobile),
        email: trim_optional(request.email),
        sort_no: request.sort_no,
        status: status.as_str().to_string(),
        operator: operator(ctx),
        now: now_utc(),
    })
}

fn org_write(
    id: String,
    tenant_id: &str,
    ctx: &RequestContext,
    request: CreateOrgRequest,
) -> AppResult<OrgWrite> {
    validate_required(&request.org_code, "orgCode")?;
    validate_required(&request.name, "name")?;
    let org_type = OrgType::try_from(request.org_type.as_str())?;
    let status = HrmStatus::try_from(request.status.as_str())?;
    let pinyin = to_pinyin_text(&request.name);
    Ok(OrgWrite {
        id,
        tenant_id: tenant_id.to_string(),
        parent_id: trim_optional(request.parent_id),
        org_code: request.org_code.trim().to_string(),
        name: request.name.trim().to_string(),
        name_full_pinyin: pinyin.full,
        name_simple_pinyin: pinyin.simple,
        org_type: org_type.as_str().to_string(),
        sort_no: request.sort_no,
        status: status.as_str().to_string(),
        operator: operator(ctx),
        now: now_utc(),
    })
}

fn post_write(
    id: String,
    tenant_id: &str,
    ctx: &RequestContext,
    request: CreatePostRequest,
) -> AppResult<PostWrite> {
    validate_required(&request.post_code, "postCode")?;
    validate_required(&request.name, "name")?;
    let status = HrmStatus::try_from(request.status.as_str())?;
    let pinyin = to_pinyin_text(&request.name);
    Ok(PostWrite {
        id,
        tenant_id: tenant_id.to_string(),
        post_code: request.post_code.trim().to_string(),
        name: request.name.trim().to_string(),
        name_full_pinyin: pinyin.full,
        name_simple_pinyin: pinyin.simple,
        sort_no: request.sort_no,
        status: status.as_str().to_string(),
        operator: operator(ctx),
        now: now_utc(),
    })
}

fn relation_write(
    id: String,
    tenant_id: &str,
    ctx: &RequestContext,
    request: CreateUserOrgPostRequest,
) -> UserOrgPostWrite {
    UserOrgPostWrite {
        id,
        tenant_id: tenant_id.to_string(),
        user_id: request.user_id,
        org_id: request.org_id,
        post_id: request.post_id,
        primary_org: request.primary_org,
        primary_post: request.primary_post,
        sort_no: request.sort_no,
        operator: operator(ctx),
        now: now_utc(),
    }
}

async fn validate_org_tree_for_write(
    pool: &DatabasePool,
    tenant_id: &str,
    current_id: Option<&str>,
    parent_id: Option<&str>,
    org_type: &str,
) -> AppResult<()> {
    let org_type = OrgType::try_from(org_type)?;
    if parent_id.is_none() && org_type != OrgType::Branch {
        return Err(AppError::business_error("根节点只能是分部"));
    }
    if let (Some(current_id), Some(parent_id)) = (current_id, parent_id) {
        if current_id == parent_id {
            return Err(AppError::business_error("组织不能把自己设置为父级"));
        }
        ensure_parent_is_not_descendant(pool, tenant_id, current_id, parent_id).await?;
    }
    if let Some(parent_id) = parent_id {
        let parent = HrmRepository::get_org(pool, tenant_id, parent_id)
            .await?
            .ok_or_else(|| AppError::business_error("父组织不存在或已删除"))?;
        ensure_enabled(&parent.status, "父组织已禁用，不能作为新增或调整父级")?;
        let parent_type = OrgType::try_from(parent.org_type.as_str())?;
        if parent_type == OrgType::Department && org_type == OrgType::Branch {
            return Err(AppError::business_error("部门下只能创建部门，不能创建分部"));
        }
    }
    if let Some(current_id) = current_id {
        if org_type == OrgType::Department
            && HrmRepository::count_branch_children(pool, tenant_id, current_id).await? > 0
        {
            return Err(AppError::business_error("组织存在分部子节点，不能改为部门"));
        }
    }
    Ok(())
}

async fn ensure_parent_is_not_descendant(
    pool: &DatabasePool,
    tenant_id: &str,
    current_id: &str,
    parent_id: &str,
) -> AppResult<()> {
    let mut cursor = Some(parent_id.to_string());
    let mut visited = HashSet::new();
    while let Some(id) = cursor {
        if id == current_id {
            return Err(AppError::business_error("组织父级调整会形成循环上下级"));
        }
        if !visited.insert(id.clone()) {
            return Err(AppError::business_error("组织祖先链存在循环数据"));
        }
        cursor = HrmRepository::get_org(pool, tenant_id, &id)
            .await?
            .and_then(|org| org.parent_id);
    }
    Ok(())
}

async fn ensure_user_active(pool: &DatabasePool, tenant_id: &str, id: &str) -> AppResult<()> {
    let user = HrmRepository::get_user(pool, tenant_id, id)
        .await?
        .ok_or_else(|| AppError::business_error("用户不存在或已删除"))?;
    ensure_enabled(&user.status, "用户已禁用，不能新增任职关系")
}

async fn ensure_org_active(pool: &DatabasePool, tenant_id: &str, id: &str) -> AppResult<()> {
    let org = HrmRepository::get_org(pool, tenant_id, id)
        .await?
        .ok_or_else(|| AppError::business_error("组织不存在或已删除"))?;
    ensure_enabled(&org.status, "组织已禁用，不能新增任职关系")
}

async fn ensure_post_active(pool: &DatabasePool, tenant_id: &str, id: &str) -> AppResult<()> {
    let post = HrmRepository::get_post(pool, tenant_id, id)
        .await?
        .ok_or_else(|| AppError::business_error("岗位不存在或已删除"))?;
    ensure_enabled(&post.status, "岗位已禁用，不能新增任职关系")
}

fn ensure_enabled(status: &str, message: &str) -> AppResult<()> {
    if HrmStatus::try_from(status)? != HrmStatus::Enabled {
        return Err(AppError::business_error(message));
    }
    Ok(())
}

fn build_org_tree(orgs: Vec<HrmOrg>, parent_id: Option<&str>) -> Vec<OrgTreeNode> {
    let mut by_parent: HashMap<Option<String>, Vec<HrmOrg>> = HashMap::new();
    for org in orgs {
        by_parent
            .entry(org.parent_id.clone())
            .or_default()
            .push(org);
    }
    build_org_tree_from_map(&mut by_parent, parent_id.map(ToOwned::to_owned))
}

fn build_org_tree_from_map(
    by_parent: &mut HashMap<Option<String>, Vec<HrmOrg>>,
    parent_id: Option<String>,
) -> Vec<OrgTreeNode> {
    let mut nodes = by_parent.remove(&parent_id).unwrap_or_default();
    nodes.sort_by(|left, right| {
        left.sort_no
            .cmp(&right.sort_no)
            .then_with(|| left.created_time.cmp(&right.created_time))
    });
    nodes
        .into_iter()
        .map(|org| {
            let id = org.id.clone();
            OrgTreeNode {
                org,
                children: build_org_tree_from_map(by_parent, Some(id)),
            }
        })
        .collect()
}

fn validate_batch_ids(ids: &[String]) -> AppResult<()> {
    if ids.len() > MAX_BATCH_IDS {
        return Err(AppError::param_invalid("批量 ID 单次最多 500 个"));
    }
    Ok(())
}

fn validate_required(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::param_invalid(format!("{field} 不能为空")));
    }
    Ok(())
}

fn validate_status_filter(status: Option<&str>) -> AppResult<()> {
    if let Some(status) = status {
        HrmStatus::try_from(status)?;
    }
    Ok(())
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn tenant_id(ctx: &RequestContext) -> AppResult<String> {
    ctx.tenant_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::param_invalid("租户上下文缺失"))
}

fn operator(ctx: &RequestContext) -> String {
    ctx.user_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| SYSTEM_OPERATOR.to_string())
}

fn ensure_updated(affected: u64) -> AppResult<()> {
    if affected == 0 {
        return Err(AppError::conflict("数据已被修改或不存在，请刷新后重试"));
    }
    Ok(())
}

fn ensure_deleted(affected: u64) -> AppResult<()> {
    if affected == 0 {
        return Err(AppError::resource_not_found(
            "资源不存在，或尚未逻辑删除，不能物理删除",
        ));
    }
    Ok(())
}

fn map_write_error(error: AppError) -> AppError {
    if error
        .internal_message
        .as_deref()
        .is_some_and(|message| message.contains("Duplicate") || message.contains("unique"))
    {
        return AppError::conflict("业务唯一键冲突或主关系重复");
    }
    error
}
