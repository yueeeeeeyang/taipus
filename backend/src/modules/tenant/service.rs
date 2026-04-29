//! 租户业务服务层。
//!
//! service 负责租户状态、隔离模式、拼音生成、乐观锁结果判断和租户解析校验。

use crate::{
    context::request_context::RequestContext,
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::tenant::{
        dto::{CreateTenantRequest, TenantPageQuery, UpdateTenantRequest},
        model::{Tenant, TenantIsolationMode, TenantStatus},
        repository::{TenantRepository, TenantWrite},
    },
    response::page::PageResult,
    utils::{id::generate_business_id, pinyin::to_pinyin_text, time::now_utc},
};

const SYSTEM_OPERATOR: &str = "system";

pub struct TenantService;

impl TenantService {
    pub async fn create(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateTenantRequest,
    ) -> AppResult<Tenant> {
        let data = tenant_write(generate_business_id(), ctx, request)?;
        TenantRepository::insert(pool, &data)
            .await
            .map_err(map_write_error)?;
        Self::get(pool, &data.id).await
    }

    pub async fn update(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: UpdateTenantRequest,
    ) -> AppResult<Tenant> {
        let data = tenant_write(
            id.to_string(),
            ctx,
            CreateTenantRequest {
                tenant_code: request.tenant_code,
                name: request.name,
                isolation_mode: request.isolation_mode,
                primary_domain: request.primary_domain,
                remark: request.remark,
            },
        )?;
        let affected = TenantRepository::update(pool, id, request.version, &data)
            .await
            .map_err(map_write_error)?;
        ensure_updated(affected)?;
        Self::get(pool, id).await
    }

    pub async fn get(pool: &DatabasePool, id: &str) -> AppResult<Tenant> {
        TenantRepository::get(pool, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("租户不存在或已删除"))
    }

    pub async fn page(
        pool: &DatabasePool,
        query: TenantPageQuery,
    ) -> AppResult<PageResult<Tenant>> {
        let page = query.page.validate_and_normalize()?;
        validate_status_filter(query.status.as_deref())?;
        if let Some(mode) = query.isolation_mode.as_deref() {
            TenantIsolationMode::try_from(mode)?;
        }
        let (records, total) = TenantRepository::page(pool, &query, page).await?;
        Ok(PageResult::new(records, page, total))
    }

    pub async fn logical_delete(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<()> {
        let affected =
            TenantRepository::logical_delete(pool, id, version, &operator(ctx), now_utc()).await?;
        ensure_updated(affected)
    }

    pub async fn physical_delete(pool: &DatabasePool, id: &str) -> AppResult<()> {
        let affected = TenantRepository::physical_delete(pool, id).await?;
        if affected == 0 {
            return Err(AppError::resource_not_found(
                "租户不存在，或尚未逻辑删除，不能物理删除",
            ));
        }
        Ok(())
    }

    pub async fn enable(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<Tenant> {
        Self::change_status(pool, ctx, id, version, TenantStatus::Enabled).await
    }

    pub async fn disable(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<Tenant> {
        Self::change_status(pool, ctx, id, version, TenantStatus::Disabled).await
    }

    pub async fn suspend(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<Tenant> {
        Self::change_status(pool, ctx, id, version, TenantStatus::Suspended).await
    }

    pub async fn ensure_request_tenant_active(
        pool: &DatabasePool,
        tenant_id: &str,
    ) -> AppResult<()> {
        let tenant = TenantRepository::get(pool, tenant_id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("租户不存在或已删除"))?;
        match TenantStatus::try_from(tenant.status.as_str())? {
            TenantStatus::Enabled => Ok(()),
            TenantStatus::Disabled => Err(AppError::business_error("租户已禁用，不能访问业务接口")),
            TenantStatus::Suspended => {
                Err(AppError::business_error("租户已暂停，不能访问业务接口"))
            }
        }
    }

    async fn change_status(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
        status: TenantStatus,
    ) -> AppResult<Tenant> {
        let affected = TenantRepository::update_status(
            pool,
            id,
            version,
            status.as_str(),
            &operator(ctx),
            now_utc(),
        )
        .await?;
        ensure_updated(affected)?;
        Self::get(pool, id).await
    }
}

fn tenant_write(
    id: String,
    ctx: &RequestContext,
    request: CreateTenantRequest,
) -> AppResult<TenantWrite> {
    validate_required(&request.tenant_code, "tenantCode")?;
    validate_required(&request.name, "name")?;
    let isolation_mode = TenantIsolationMode::try_from(request.isolation_mode.as_str())?;
    if isolation_mode != TenantIsolationMode::SharedSchema {
        return Err(AppError::business_error(
            "首版租户隔离模式只允许 shared_schema",
        ));
    }
    let pinyin = to_pinyin_text(&request.name);
    Ok(TenantWrite {
        id,
        tenant_code: request.tenant_code.trim().to_string(),
        name: request.name.trim().to_string(),
        name_full_pinyin: pinyin.full,
        name_simple_pinyin: pinyin.simple,
        isolation_mode: isolation_mode.as_str().to_string(),
        primary_domain: trim_optional(request.primary_domain),
        remark: trim_optional(request.remark),
        operator: operator(ctx),
        now: now_utc(),
    })
}

fn validate_required(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::param_invalid(format!("{field} 不能为空")));
    }
    Ok(())
}

fn validate_status_filter(status: Option<&str>) -> AppResult<()> {
    if let Some(status) = status {
        TenantStatus::try_from(status)?;
    }
    Ok(())
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

fn map_write_error(error: AppError) -> AppError {
    if error
        .internal_message
        .as_deref()
        .is_some_and(|message| message.contains("Duplicate") || message.contains("unique"))
    {
        return AppError::conflict("租户编码已存在");
    }
    error
}
