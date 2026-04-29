-- 创建认证模块首版表。
-- 认证模块只负责账号、账号租户关系、刷新令牌会话和安全审计，不承载角色或权限体系。

CREATE TABLE sys_accounts (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    username VARCHAR(64) NOT NULL,
    display_name VARCHAR(128) NOT NULL,
    display_name_full_pinyin VARCHAR(256) NOT NULL,
    display_name_simple_pinyin VARCHAR(128) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    password_algo VARCHAR(32) NOT NULL,
    password_updated_time TIMESTAMP NULL,
    status VARCHAR(32) NOT NULL,
    hrm_user_id VARCHAR(64) NULL,
    last_login_time TIMESTAMP NULL,
    last_login_ip VARCHAR(64) NULL,
    active_username VARCHAR(128) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN username ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_accounts_active_username UNIQUE (active_username)
);

CREATE INDEX idx_sys_accounts_deleted_status ON sys_accounts (deleted, status);
CREATE INDEX idx_sys_accounts_deleted_updated ON sys_accounts (deleted, updated_time);
CREATE INDEX idx_sys_accounts_display_name ON sys_accounts (display_name);
CREATE INDEX idx_sys_accounts_display_full_pinyin ON sys_accounts (display_name_full_pinyin);
CREATE INDEX idx_sys_accounts_display_simple_pinyin ON sys_accounts (display_name_simple_pinyin);

CREATE TABLE sys_account_tenants (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    account_id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    active_relation_key VARCHAR(140) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN CONCAT(account_id, '#', tenant_id) ELSE NULL END
    ) STORED,
    active_default_key VARCHAR(64) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE AND is_default = TRUE THEN account_id ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_account_tenants_active_relation UNIQUE (active_relation_key),
    CONSTRAINT uk_sys_account_tenants_active_default UNIQUE (active_default_key)
);

CREATE INDEX idx_sys_account_tenants_account_status ON sys_account_tenants (account_id, deleted, status);
CREATE INDEX idx_sys_account_tenants_tenant_status ON sys_account_tenants (tenant_id, deleted, status);

CREATE TABLE sys_refresh_tokens (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    account_id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL,
    token_hash VARCHAR(255) NOT NULL,
    token_family VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL,
    client_type VARCHAR(32) NOT NULL,
    device_id VARCHAR(128) NULL,
    device_name VARCHAR(128) NULL,
    ip VARCHAR(64) NULL,
    user_agent VARCHAR(512) NULL,
    expires_time TIMESTAMP NOT NULL,
    last_used_time TIMESTAMP NULL,
    revoked_by VARCHAR(64) NULL,
    revoked_time TIMESTAMP NULL,
    revoked_reason VARCHAR(255) NULL,
    active_token_hash VARCHAR(255) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN token_hash ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_refresh_tokens_active_hash UNIQUE (active_token_hash)
);

CREATE INDEX idx_sys_refresh_tokens_account_status ON sys_refresh_tokens (account_id, deleted, status);
CREATE INDEX idx_sys_refresh_tokens_tenant_status ON sys_refresh_tokens (tenant_id, deleted, status);
CREATE INDEX idx_sys_refresh_tokens_expires ON sys_refresh_tokens (expires_time);
CREATE INDEX idx_sys_refresh_tokens_family ON sys_refresh_tokens (token_family);

CREATE TABLE sys_auth_audit_logs (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NULL,
    account_id VARCHAR(64) NULL,
    event_type VARCHAR(64) NOT NULL,
    result VARCHAR(32) NOT NULL,
    client_type VARCHAR(32) NULL,
    ip VARCHAR(64) NULL,
    user_agent VARCHAR(512) NULL,
    trace_id VARCHAR(64) NOT NULL,
    risk_level VARCHAR(32) NULL,
    message VARCHAR(512) NULL,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL
);

CREATE INDEX idx_sys_auth_audit_logs_account_time ON sys_auth_audit_logs (account_id, created_time);
CREATE INDEX idx_sys_auth_audit_logs_tenant_time ON sys_auth_audit_logs (tenant_id, created_time);
CREATE INDEX idx_sys_auth_audit_logs_event_time ON sys_auth_audit_logs (event_type, created_time);
