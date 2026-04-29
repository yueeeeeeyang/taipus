//! 认证模块业务服务。
//!
//! service 负责账号状态、租户关系、密码校验、令牌签发和刷新令牌状态流转，handler 不承载业务规则。

use chrono::Utc;

use crate::{
    config::settings::AuthConfig,
    context::request_context::RequestContext,
    db::executor::DatabasePool,
    error::app_error::{AppError, AppResult},
    modules::{
        auth::{
            dto::*,
            model::{
                Account, AccountStatus, AccountTenant, AccountTenantStatus, ClientType,
                RefreshTokenSession, RefreshTokenStatus,
            },
            password::{PASSWORD_ALGO_ARGON2ID, hash_password, verify_password},
            repository::{AccountTenantWrite, AccountWrite, AuthRepository, RefreshTokenWrite},
            token::{AuthTokenService, hash_refresh_token},
        },
        tenant::service::TenantService,
    },
    response::page::PageResult,
    utils::{id::generate_business_id, pinyin::to_pinyin_text, time::now_utc},
};

pub struct AuthService;

impl AuthService {
    pub async fn login(
        pool: &DatabasePool,
        config: &AuthConfig,
        ctx: &RequestContext,
        request: LoginRequest,
    ) -> AppResult<TokenResponse> {
        validate_required(&request.username, "username")?;
        validate_required(&request.password, "password")?;
        let client_type = ClientType::try_from(request.client_type.as_str())?;
        let account =
            match AuthRepository::find_account_by_username(pool, request.username.trim()).await? {
                Some(account) => account,
                None => {
                    audit(
                        pool,
                        ctx,
                        None,
                        None,
                        "login",
                        "failure",
                        Some(client_type.as_str()),
                        "账号或密码错误",
                    )
                    .await;
                    return Err(AppError::unauthorized("账号或密码错误"));
                }
            };
        if !verify_password(&request.password, &account.password_hash)? {
            audit(
                pool,
                ctx,
                None,
                Some(&account.id),
                "login",
                "failure",
                Some(client_type.as_str()),
                "账号或密码错误",
            )
            .await;
            return Err(AppError::unauthorized("账号或密码错误"));
        }
        ensure_account_loginable(&account)?;
        let tenants = AuthRepository::list_account_tenants(pool, &account.id).await?;
        let tenant = choose_login_tenant(&tenants, request.tenant_id.as_deref())?;
        ensure_account_tenant_enabled(&tenant)?;
        TenantService::ensure_request_tenant_active(pool, &tenant.tenant_id).await?;
        let response = Self::issue_tokens_for_session(
            pool,
            config,
            ctx,
            &account,
            &tenant,
            client_type.as_str(),
            request.device_id,
            request.device_name,
            None,
        )
        .await?;
        AuthRepository::update_last_login(pool, &account.id, ctx.client_ip.as_deref(), now_utc())
            .await?;
        audit(
            pool,
            ctx,
            Some(&tenant.tenant_id),
            Some(&account.id),
            "login",
            "success",
            Some(client_type.as_str()),
            "登录成功",
        )
        .await;
        Ok(response)
    }

    pub async fn refresh(
        pool: &DatabasePool,
        config: &AuthConfig,
        ctx: &RequestContext,
        request: RefreshRequest,
    ) -> AppResult<TokenResponse> {
        let client_type = ClientType::try_from(request.client_type.as_str())?;
        let token_hash = hash_refresh_token(config, &request.refresh_token)?;
        let session = AuthRepository::find_refresh_by_hash(pool, &token_hash)
            .await?
            .ok_or_else(|| AppError::unauthorized("刷新令牌无效或已过期"))?;
        if session.status != RefreshTokenStatus::Active.as_str() {
            if session.status == RefreshTokenStatus::Rotated.as_str() {
                let _ = AuthRepository::change_refresh_status(
                    pool,
                    &session.id,
                    session.version,
                    RefreshTokenStatus::Compromised.as_str(),
                    &session.account_id,
                    Some("刷新令牌重放"),
                    now_utc(),
                )
                .await;
                let _ = AuthRepository::revoke_token_family_sessions(
                    pool,
                    &session.token_family,
                    &session.account_id,
                    "刷新令牌重放",
                    now_utc(),
                )
                .await;
            }
            return Err(AppError::unauthorized("刷新令牌无效或已过期"));
        }
        if session.expires_time <= Utc::now() {
            let _ = AuthRepository::change_refresh_status(
                pool,
                &session.id,
                session.version,
                RefreshTokenStatus::Expired.as_str(),
                &session.account_id,
                Some("刷新令牌过期"),
                now_utc(),
            )
            .await;
            return Err(AppError::unauthorized("刷新令牌无效或已过期"));
        }
        if session.client_type != client_type.as_str() {
            return Err(AppError::unauthorized("刷新令牌无效或已过期"));
        }
        let account = Self::ensure_account_active(pool, &session.account_id).await?;
        let tenant =
            AuthRepository::find_account_tenant(pool, &session.account_id, &session.tenant_id)
                .await?
                .ok_or_else(|| AppError::unauthorized("账号租户关系无效"))?;
        ensure_account_tenant_enabled(&tenant)?;
        TenantService::ensure_request_tenant_active(pool, &session.tenant_id).await?;
        AuthRepository::change_refresh_status(
            pool,
            &session.id,
            session.version,
            RefreshTokenStatus::Rotated.as_str(),
            &account.id,
            Some("刷新令牌轮换"),
            now_utc(),
        )
        .await?;
        let response = Self::issue_tokens_for_session(
            pool,
            config,
            ctx,
            &account,
            &tenant,
            client_type.as_str(),
            request.device_id,
            session.device_name,
            Some(session.token_family),
        )
        .await?;
        audit(
            pool,
            ctx,
            Some(&tenant.tenant_id),
            Some(&account.id),
            "refresh",
            "success",
            Some(client_type.as_str()),
            "刷新令牌成功",
        )
        .await;
        Ok(response)
    }

    pub async fn logout(
        pool: &DatabasePool,
        config: &AuthConfig,
        ctx: &RequestContext,
        request: LogoutRequest,
    ) -> AppResult<()> {
        let refresh_token = request
            .refresh_token
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::param_invalid("refreshToken 不能为空"))?;
        let hash = hash_refresh_token(config, &refresh_token)?;
        let session = AuthRepository::find_refresh_by_hash(pool, &hash)
            .await?
            .ok_or_else(|| AppError::unauthorized("刷新令牌无效或已过期"))?;
        if ctx
            .user_id
            .as_deref()
            .is_some_and(|account_id| account_id != session.account_id)
        {
            return Err(AppError::forbidden("不能退出其他账号的会话"));
        }
        AuthRepository::change_refresh_status(
            pool,
            &session.id,
            session.version,
            RefreshTokenStatus::Revoked.as_str(),
            ctx.user_id.as_deref().unwrap_or("system"),
            Some("退出登录"),
            now_utc(),
        )
        .await?;
        Ok(())
    }

    pub async fn logout_all(pool: &DatabasePool, ctx: &RequestContext) -> AppResult<()> {
        let account_id = ctx
            .user_id
            .as_deref()
            .ok_or_else(|| AppError::unauthorized("未认证或登录已过期"))?;
        AuthRepository::revoke_account_sessions(
            pool,
            account_id,
            account_id,
            "退出全部终端",
            now_utc(),
        )
        .await
    }

    pub async fn switch_tenant(
        pool: &DatabasePool,
        config: &AuthConfig,
        ctx: &RequestContext,
        request: SwitchTenantRequest,
    ) -> AppResult<TokenResponse> {
        let hash = hash_refresh_token(config, &request.refresh_token)?;
        let session = AuthRepository::find_refresh_by_hash(pool, &hash)
            .await?
            .ok_or_else(|| AppError::unauthorized("刷新令牌无效或已过期"))?;
        if session.status != RefreshTokenStatus::Active.as_str()
            || session.expires_time <= Utc::now()
        {
            return Err(AppError::unauthorized("刷新令牌无效或已过期"));
        }
        let account = Self::ensure_account_active(pool, &session.account_id).await?;
        let tenant = AuthRepository::find_account_tenant(pool, &account.id, &request.tenant_id)
            .await?
            .ok_or_else(|| AppError::business_error("账号不能访问目标租户"))?;
        ensure_account_tenant_enabled(&tenant)?;
        TenantService::ensure_request_tenant_active(pool, &tenant.tenant_id).await?;
        AuthRepository::change_refresh_status(
            pool,
            &session.id,
            session.version,
            RefreshTokenStatus::Rotated.as_str(),
            &account.id,
            Some("切换租户"),
            now_utc(),
        )
        .await?;
        Self::issue_tokens_for_session(
            pool,
            config,
            ctx,
            &account,
            &tenant,
            &session.client_type,
            session.device_id,
            session.device_name,
            Some(session.token_family),
        )
        .await
    }

    pub async fn me(pool: &DatabasePool, ctx: &RequestContext) -> AppResult<AccountSummary> {
        let account_id = ctx
            .user_id
            .as_deref()
            .ok_or_else(|| AppError::unauthorized("未认证或登录已过期"))?;
        Self::ensure_account_active(pool, account_id)
            .await
            .map(account_summary)
    }

    pub async fn my_tenants(
        pool: &DatabasePool,
        ctx: &RequestContext,
    ) -> AppResult<Vec<AccountTenantSummary>> {
        let account_id = ctx
            .user_id
            .as_deref()
            .ok_or_else(|| AppError::unauthorized("未认证或登录已过期"))?;
        let tenants = AuthRepository::list_account_tenants(pool, account_id).await?;
        Ok(tenants.into_iter().map(account_tenant_summary).collect())
    }

    pub async fn create_account(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateAccountRequest,
    ) -> AppResult<Account> {
        validate_required(&request.username, "username")?;
        validate_required(&request.display_name, "displayName")?;
        let status = AccountStatus::try_from(request.status.as_str())?;
        let pinyin = to_pinyin_text(&request.display_name);
        let now = now_utc();
        let data = AccountWrite {
            id: generate_business_id(),
            username: request.username.trim().to_string(),
            display_name: request.display_name.trim().to_string(),
            display_name_full_pinyin: pinyin.full,
            display_name_simple_pinyin: pinyin.simple,
            password_hash: hash_password(&request.password)?,
            password_algo: PASSWORD_ALGO_ARGON2ID.to_string(),
            status: status.as_str().to_string(),
            hrm_user_id: request.hrm_user_id,
            operator: operator(ctx),
            now,
        };
        AuthRepository::insert_account(pool, &data).await?;
        Self::get_account(pool, &data.id).await
    }

    pub async fn update_account(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: UpdateAccountRequest,
    ) -> AppResult<Account> {
        validate_required(&request.username, "username")?;
        validate_required(&request.display_name, "displayName")?;
        let status = AccountStatus::try_from(request.status.as_str())?;
        let current = Self::get_account(pool, id).await?;
        let pinyin = to_pinyin_text(&request.display_name);
        let data = AccountWrite {
            id: id.to_string(),
            username: request.username.trim().to_string(),
            display_name: request.display_name.trim().to_string(),
            display_name_full_pinyin: pinyin.full,
            display_name_simple_pinyin: pinyin.simple,
            password_hash: current.password_hash,
            password_algo: current.password_algo,
            status: status.as_str().to_string(),
            hrm_user_id: request.hrm_user_id,
            operator: operator(ctx),
            now: now_utc(),
        };
        ensure_updated(AuthRepository::update_account(pool, id, request.version, &data).await?)?;
        Self::get_account(pool, id).await
    }

    pub async fn get_account(pool: &DatabasePool, id: &str) -> AppResult<Account> {
        AuthRepository::find_account_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("账号不存在或已删除"))
    }

    pub async fn page_accounts(
        pool: &DatabasePool,
        query: AccountPageQuery,
    ) -> AppResult<PageResult<Account>> {
        let page = query.page.validate_and_normalize()?;
        AuthRepository::page_accounts(pool, &query, page).await
    }

    pub async fn set_account_status(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
        status: AccountStatus,
    ) -> AppResult<Account> {
        ensure_updated(
            AuthRepository::update_account_status(
                pool,
                id,
                version,
                status.as_str(),
                &operator(ctx),
                now_utc(),
            )
            .await?,
        )?;
        if status != AccountStatus::Enabled {
            AuthRepository::revoke_account_sessions(
                pool,
                id,
                &operator(ctx),
                "账号状态变更",
                now_utc(),
            )
            .await?;
        }
        Self::get_account(pool, id).await
    }

    pub async fn reset_password(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: ResetPasswordRequest,
    ) -> AppResult<()> {
        let password_hash = hash_password(&request.password)?;
        ensure_updated(
            AuthRepository::reset_password(
                pool,
                id,
                request.version,
                &password_hash,
                &operator(ctx),
                now_utc(),
            )
            .await?,
        )?;
        AuthRepository::revoke_account_sessions(pool, id, &operator(ctx), "重置密码", now_utc())
            .await
    }

    pub async fn delete_account(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<()> {
        ensure_updated(
            AuthRepository::logical_delete_account(pool, id, version, &operator(ctx), now_utc())
                .await?,
        )?;
        AuthRepository::revoke_account_sessions(pool, id, &operator(ctx), "账号删除", now_utc())
            .await
    }

    pub async fn physical_delete_account(pool: &DatabasePool, id: &str) -> AppResult<()> {
        ensure_updated(AuthRepository::physical_delete_account(pool, id).await?)
    }

    pub async fn create_account_tenant(
        pool: &DatabasePool,
        ctx: &RequestContext,
        request: CreateAccountTenantRequest,
    ) -> AppResult<AccountTenant> {
        Self::get_account(pool, &request.account_id).await?;
        TenantService::ensure_request_tenant_active(pool, &request.tenant_id).await?;
        let status = AccountTenantStatus::try_from(request.status.as_str())?;
        let data = AccountTenantWrite {
            id: generate_business_id(),
            account_id: request.account_id,
            tenant_id: request.tenant_id,
            status: status.as_str().to_string(),
            is_default: request.is_default,
            operator: operator(ctx),
            now: now_utc(),
        };
        AuthRepository::insert_account_tenant(pool, &data).await?;
        AuthRepository::find_account_tenant_by_id(pool, &data.id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("账号租户关系不存在或已删除"))
    }

    pub async fn update_account_tenant(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: UpdateAccountTenantRequest,
    ) -> AppResult<AccountTenant> {
        let status = AccountTenantStatus::try_from(request.status.as_str())?;
        ensure_updated(
            AuthRepository::update_account_tenant(
                pool,
                id,
                request.version,
                status.as_str(),
                request.is_default,
                &operator(ctx),
                now_utc(),
            )
            .await?,
        )?;
        AuthRepository::find_account_tenant_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::resource_not_found("账号租户关系不存在或已删除"))
    }

    pub async fn page_account_tenants(
        pool: &DatabasePool,
        query: AccountTenantPageQuery,
    ) -> AppResult<PageResult<AccountTenant>> {
        let page = query.page.validate_and_normalize()?;
        AuthRepository::page_account_tenants(pool, &query, page).await
    }

    pub async fn delete_account_tenant(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        version: i64,
    ) -> AppResult<()> {
        ensure_updated(
            AuthRepository::logical_delete_account_tenant(
                pool,
                id,
                version,
                &operator(ctx),
                now_utc(),
            )
            .await?,
        )
    }

    pub async fn page_sessions(
        pool: &DatabasePool,
        query: SessionPageQuery,
    ) -> AppResult<PageResult<RefreshTokenSession>> {
        let page = query.page.validate_and_normalize()?;
        AuthRepository::page_sessions(pool, &query, page).await
    }

    pub async fn revoke_session(
        pool: &DatabasePool,
        ctx: &RequestContext,
        id: &str,
        request: RevokeSessionRequest,
    ) -> AppResult<()> {
        ensure_updated(
            AuthRepository::change_refresh_status(
                pool,
                id,
                request.version,
                RefreshTokenStatus::Revoked.as_str(),
                &operator(ctx),
                request.reason.as_deref(),
                now_utc(),
            )
            .await?,
        )
    }

    pub async fn ensure_authenticated_request(
        pool: &DatabasePool,
        account_id: &str,
        tenant_id: &str,
        session_id: &str,
    ) -> AppResult<Account> {
        let account = Self::ensure_account_active(pool, account_id).await?;
        let relation = AuthRepository::find_account_tenant(pool, account_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::forbidden("账号不能访问当前租户"))?;
        ensure_account_tenant_enabled(&relation)?;
        let session = AuthRepository::find_refresh_by_id(pool, session_id)
            .await?
            .ok_or_else(|| AppError::unauthorized("会话无效或已过期"))?;
        if session.account_id != account_id
            || session.tenant_id != tenant_id
            || session.status != RefreshTokenStatus::Active.as_str()
            || session.expires_time <= Utc::now()
        {
            return Err(AppError::unauthorized("会话无效或已过期"));
        }
        Ok(account)
    }

    async fn ensure_account_active(pool: &DatabasePool, account_id: &str) -> AppResult<Account> {
        let account = Self::get_account(pool, account_id).await?;
        ensure_account_loginable(&account)?;
        Ok(account)
    }

    async fn issue_tokens_for_session(
        pool: &DatabasePool,
        config: &AuthConfig,
        ctx: &RequestContext,
        account: &Account,
        tenant: &AccountTenant,
        client_type: &str,
        device_id: Option<String>,
        device_name: Option<String>,
        token_family: Option<String>,
    ) -> AppResult<TokenResponse> {
        let session_id = generate_business_id();
        let pair = AuthTokenService::issue_pair(
            config,
            &account.id,
            &tenant.tenant_id,
            &session_id,
            client_type,
        )?;
        let refresh_hash = hash_refresh_token(config, &pair.refresh_token)?;
        AuthRepository::insert_refresh_token(
            pool,
            &RefreshTokenWrite {
                id: session_id,
                account_id: account.id.clone(),
                tenant_id: tenant.tenant_id.clone(),
                token_hash: refresh_hash,
                token_family: token_family.unwrap_or_else(generate_business_id),
                status: RefreshTokenStatus::Active.as_str().to_string(),
                client_type: client_type.to_string(),
                device_id,
                device_name,
                ip: ctx.client_ip.clone(),
                user_agent: ctx.user_agent.clone(),
                expires_time: pair.refresh_expires_at,
                operator: account.id.clone(),
                now: now_utc(),
            },
        )
        .await?;
        let tenants = AuthRepository::list_account_tenants(pool, &account.id).await?;
        Ok(TokenResponse {
            access_token: pair.access_token,
            refresh_token: pair.refresh_token,
            expires_in: (pair.access_expires_at - Utc::now()).num_seconds().max(0),
            refresh_expires_in: (pair.refresh_expires_at - Utc::now()).num_seconds().max(0),
            token_type: "Bearer".to_string(),
            tenant_id: tenant.tenant_id.clone(),
            account: account_summary(account.clone()),
            tenants: tenants.into_iter().map(account_tenant_summary).collect(),
        })
    }
}

pub async fn bootstrap_admin(pool: &DatabasePool, config: &AuthConfig) -> AppResult<()> {
    let (Some(username), Some(password), Some(display_name), Some(tenant_id)) = (
        config.bootstrap_admin_username.as_deref(),
        config.bootstrap_admin_password.as_deref(),
        config.bootstrap_admin_display_name.as_deref(),
        config.bootstrap_admin_tenant_id.as_deref(),
    ) else {
        return Ok(());
    };
    if AuthRepository::find_account_by_username(pool, username)
        .await?
        .is_some()
    {
        return Ok(());
    }
    let ctx = RequestContext::anonymous("bootstrap-auth");
    let account = AuthService::create_account(
        pool,
        &ctx,
        CreateAccountRequest {
            username: username.to_string(),
            display_name: display_name.to_string(),
            password: password.to_string(),
            status: AccountStatus::Enabled.as_str().to_string(),
            hrm_user_id: None,
        },
    )
    .await?;
    AuthService::create_account_tenant(
        pool,
        &ctx,
        CreateAccountTenantRequest {
            account_id: account.id,
            tenant_id: tenant_id.to_string(),
            status: AccountTenantStatus::Enabled.as_str().to_string(),
            is_default: true,
        },
    )
    .await?;
    Ok(())
}

fn choose_login_tenant(
    tenants: &[AccountTenant],
    requested: Option<&str>,
) -> AppResult<AccountTenant> {
    let enabled: Vec<_> = tenants
        .iter()
        .filter(|tenant| tenant.status == AccountTenantStatus::Enabled.as_str())
        .collect();
    if let Some(requested) = requested.filter(|value| !value.trim().is_empty()) {
        return enabled
            .into_iter()
            .find(|tenant| tenant.tenant_id == requested.trim())
            .cloned()
            .ok_or_else(|| AppError::business_error("账号不能访问指定租户"));
    }
    let defaults: Vec<_> = enabled.iter().filter(|tenant| tenant.is_default).collect();
    if defaults.len() == 1 {
        return Ok((*defaults[0]).clone());
    }
    if enabled.len() == 1 {
        return Ok((*enabled[0]).clone());
    }
    Err(AppError::business_error(
        "账号存在多个可访问租户，请指定 tenantId",
    ))
}

fn ensure_account_loginable(account: &Account) -> AppResult<()> {
    match AccountStatus::try_from(account.status.as_str())? {
        AccountStatus::Enabled => Ok(()),
        AccountStatus::Disabled => Err(AppError::business_error("账号已禁用")),
        AccountStatus::Locked => Err(AppError::business_error("账号已锁定")),
        AccountStatus::PasswordExpired => Err(AppError::business_error("密码已过期")),
    }
}

fn ensure_account_tenant_enabled(relation: &AccountTenant) -> AppResult<()> {
    match AccountTenantStatus::try_from(relation.status.as_str())? {
        AccountTenantStatus::Enabled => Ok(()),
        AccountTenantStatus::Disabled => Err(AppError::business_error("账号租户关系已禁用")),
    }
}

fn account_summary(account: Account) -> AccountSummary {
    AccountSummary {
        id: account.id,
        username: account.username,
        display_name: account.display_name,
        status: account.status,
        hrm_user_id: account.hrm_user_id,
    }
}

fn account_tenant_summary(tenant: AccountTenant) -> AccountTenantSummary {
    AccountTenantSummary {
        id: tenant.id,
        account_id: tenant.account_id,
        tenant_id: tenant.tenant_id,
        tenant_name: None,
        status: tenant.status,
        is_default: tenant.is_default,
    }
}

fn validate_required(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        Err(AppError::param_invalid(format!("{field} 不能为空")))
    } else {
        Ok(())
    }
}

fn operator(ctx: &RequestContext) -> String {
    ctx.user_id.clone().unwrap_or_else(|| "system".to_string())
}

fn ensure_updated(affected: u64) -> AppResult<()> {
    if affected == 0 {
        Err(AppError::conflict("数据已被修改，请刷新后重试"))
    } else {
        Ok(())
    }
}

async fn audit(
    pool: &DatabasePool,
    ctx: &RequestContext,
    tenant_id: Option<&str>,
    account_id: Option<&str>,
    event_type: &str,
    result: &str,
    client_type: Option<&str>,
    message: &str,
) {
    let _ = AuthRepository::insert_audit(
        pool,
        tenant_id,
        account_id,
        event_type,
        result,
        client_type,
        ctx.client_ip.as_deref(),
        ctx.user_agent.as_deref(),
        &ctx.trace_id,
        Some(message),
    )
    .await;
}
