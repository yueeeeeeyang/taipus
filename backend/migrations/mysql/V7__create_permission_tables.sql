-- 创建权限模块首版表。
-- 权限模块采用应用、菜单、按钮、接口资源拆表设计，授权规则通过 resource_type + resource_id 统一引用资源。

CREATE TABLE sys_permission_applications (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NULL,
    app_code VARCHAR(128) NOT NULL,
    name VARCHAR(128) NOT NULL,
    name_full_pinyin VARCHAR(256) NOT NULL,
    name_simple_pinyin VARCHAR(128) NOT NULL,
    platform VARCHAR(32) NOT NULL,
    home_path VARCHAR(512) NULL,
    icon VARCHAR(128) NULL,
    sort_no BIGINT NOT NULL,
    status VARCHAR(32) NOT NULL,
    remark VARCHAR(512) NULL,
    active_app_key VARCHAR(256) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN CONCAT(COALESCE(tenant_id, 'platform'), '#', app_code) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_permission_applications_active_code UNIQUE (active_app_key)
);

CREATE INDEX idx_perm_apps_tenant_status ON sys_permission_applications (tenant_id, deleted, status);
CREATE INDEX idx_perm_apps_tenant_sort ON sys_permission_applications (tenant_id, deleted, sort_no);

CREATE TABLE sys_permission_menus (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NULL,
    app_id VARCHAR(64) NOT NULL,
    parent_id VARCHAR(64) NULL,
    menu_code VARCHAR(128) NOT NULL,
    name VARCHAR(128) NOT NULL,
    name_full_pinyin VARCHAR(256) NOT NULL,
    name_simple_pinyin VARCHAR(128) NOT NULL,
    platform VARCHAR(32) NOT NULL,
    route_path VARCHAR(512) NOT NULL,
    component VARCHAR(255) NULL,
    icon VARCHAR(128) NULL,
    visible BOOLEAN NOT NULL DEFAULT TRUE,
    keep_alive BOOLEAN NOT NULL DEFAULT FALSE,
    sort_no BIGINT NOT NULL,
    status VARCHAR(32) NOT NULL,
    remark VARCHAR(512) NULL,
    active_menu_key VARCHAR(256) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN CONCAT(COALESCE(tenant_id, 'platform'), '#', menu_code) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_permission_menus_active_code UNIQUE (active_menu_key)
);

CREATE INDEX idx_perm_menus_app_status ON sys_permission_menus (tenant_id, app_id, deleted, status);
CREATE INDEX idx_perm_menus_parent_sort ON sys_permission_menus (tenant_id, parent_id, deleted, sort_no);

CREATE TABLE sys_permission_buttons (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NULL,
    app_id VARCHAR(64) NOT NULL,
    menu_id VARCHAR(64) NOT NULL,
    button_code VARCHAR(128) NOT NULL,
    name VARCHAR(128) NOT NULL,
    action_key VARCHAR(64) NOT NULL,
    button_type VARCHAR(32) NOT NULL,
    icon VARCHAR(128) NULL,
    sort_no BIGINT NOT NULL,
    status VARCHAR(32) NOT NULL,
    remark VARCHAR(512) NULL,
    active_button_key VARCHAR(256) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN CONCAT(COALESCE(tenant_id, 'platform'), '#', button_code) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_permission_buttons_active_code UNIQUE (active_button_key)
);

CREATE INDEX idx_perm_buttons_menu_sort ON sys_permission_buttons (tenant_id, menu_id, deleted, status, sort_no);

CREATE TABLE sys_permission_apis (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NULL,
    app_id VARCHAR(64) NULL,
    api_code VARCHAR(128) NOT NULL,
    name VARCHAR(128) NOT NULL,
    http_method VARCHAR(16) NOT NULL,
    path_pattern VARCHAR(512) NOT NULL,
    normalized_path VARCHAR(512) NOT NULL,
    related_menu_id VARCHAR(64) NULL,
    related_button_id VARCHAR(64) NULL,
    public_access BOOLEAN NOT NULL DEFAULT FALSE,
    auth_required BOOLEAN NOT NULL DEFAULT TRUE,
    status VARCHAR(32) NOT NULL,
    remark VARCHAR(512) NULL,
    active_api_key VARCHAR(256) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN CONCAT(COALESCE(tenant_id, 'platform'), '#', api_code) ELSE NULL END
    ) STORED,
    active_route_key VARCHAR(600) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE AND status = 'enabled' THEN CONCAT(http_method, '#', normalized_path) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_permission_apis_active_code UNIQUE (active_api_key),
    CONSTRAINT uk_sys_permission_apis_active_route UNIQUE (active_route_key)
);

CREATE INDEX idx_perm_apis_route ON sys_permission_apis (http_method, normalized_path, deleted, status);
CREATE INDEX idx_perm_apis_app_status ON sys_permission_apis (tenant_id, app_id, deleted, status);

CREATE TABLE sys_roles (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    role_code VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    name_full_pinyin VARCHAR(256) NOT NULL,
    name_simple_pinyin VARCHAR(128) NOT NULL,
    role_type VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL,
    sort_no BIGINT NOT NULL,
    remark VARCHAR(512) NULL,
    active_role_key VARCHAR(160) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN CONCAT(tenant_id, '#', role_code) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_roles_active_code UNIQUE (active_role_key)
);

CREATE INDEX idx_sys_roles_tenant_status ON sys_roles (tenant_id, deleted, status);
CREATE INDEX idx_sys_roles_tenant_sort ON sys_roles (tenant_id, deleted, sort_no);

CREATE TABLE sys_role_relations (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    parent_role_id VARCHAR(64) NOT NULL,
    child_role_id VARCHAR(64) NOT NULL,
    active_relation_key VARCHAR(64) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN MD5(CONCAT(tenant_id, '#', parent_role_id, '#', child_role_id)) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_role_relations_active_pair UNIQUE (active_relation_key)
);

CREATE INDEX idx_role_relations_parent ON sys_role_relations (tenant_id, parent_role_id, deleted);
CREATE INDEX idx_role_relations_child ON sys_role_relations (tenant_id, child_role_id, deleted);

CREATE TABLE sys_role_closures (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    ancestor_role_id VARCHAR(64) NOT NULL,
    descendant_role_id VARCHAR(64) NOT NULL,
    depth BIGINT NOT NULL,
    active_closure_key VARCHAR(64) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN MD5(CONCAT(tenant_id, '#', ancestor_role_id, '#', descendant_role_id)) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_role_closures_active_pair UNIQUE (active_closure_key)
);

CREATE INDEX idx_role_closures_descendant ON sys_role_closures (tenant_id, descendant_role_id, deleted);
CREATE INDEX idx_role_closures_ancestor ON sys_role_closures (tenant_id, ancestor_role_id, deleted);

CREATE TABLE sys_account_roles (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    account_id VARCHAR(64) NOT NULL,
    role_id VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL,
    active_account_role_key VARCHAR(64) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN MD5(CONCAT(tenant_id, '#', account_id, '#', role_id)) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_account_roles_active_pair UNIQUE (active_account_role_key)
);

CREATE INDEX idx_account_roles_account_status ON sys_account_roles (tenant_id, account_id, deleted, status);
CREATE INDEX idx_account_roles_role_status ON sys_account_roles (tenant_id, role_id, deleted, status);

CREATE TABLE sys_permission_grants (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    subject_type VARCHAR(32) NOT NULL,
    subject_id VARCHAR(64) NOT NULL,
    resource_type VARCHAR(32) NOT NULL,
    resource_id VARCHAR(64) NOT NULL,
    action VARCHAR(32) NOT NULL,
    effect VARCHAR(16) NOT NULL,
    grant_source VARCHAR(32) NOT NULL,
    condition_type VARCHAR(32) NULL,
    condition_value TEXT NULL,
    active_grant_key VARCHAR(320) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN CONCAT(tenant_id, '#', subject_type, '#', subject_id, '#', resource_type, '#', resource_id, '#', action) ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_permission_grants_active_key UNIQUE (active_grant_key)
);

CREATE INDEX idx_perm_grants_subject ON sys_permission_grants (tenant_id, subject_type, subject_id, deleted);
CREATE INDEX idx_perm_grants_resource ON sys_permission_grants (tenant_id, resource_type, resource_id, action, deleted);

CREATE TABLE sys_permission_versions (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    version_no BIGINT NOT NULL,
    changed_reason VARCHAR(128) NULL,
    active_tenant_key VARCHAR(128) GENERATED ALWAYS AS (
        CASE WHEN deleted = FALSE THEN tenant_id ELSE NULL END
    ) STORED,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP NOT NULL,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP NOT NULL,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP NULL,
    CONSTRAINT uk_sys_permission_versions_active_tenant UNIQUE (active_tenant_key)
);

CREATE TABLE sys_permission_audit_logs (
    id VARCHAR(64) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(64) NULL,
    account_id VARCHAR(64) NULL,
    event_type VARCHAR(64) NOT NULL,
    resource_type VARCHAR(32) NULL,
    resource_id VARCHAR(64) NULL,
    action VARCHAR(32) NULL,
    result VARCHAR(32) NOT NULL,
    trace_id VARCHAR(64) NOT NULL,
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

CREATE INDEX idx_perm_audit_tenant_time ON sys_permission_audit_logs (tenant_id, created_time);
CREATE INDEX idx_perm_audit_account_time ON sys_permission_audit_logs (account_id, created_time);

INSERT INTO sys_permission_versions (id, tenant_id, version_no, changed_reason, version, deleted, created_by, created_time, updated_by, updated_time)
VALUES ('permission_version_default', 'default', 1, '权限模块初始化', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP);

INSERT INTO sys_permission_apis (id, tenant_id, api_code, name, http_method, path_pattern, normalized_path, public_access, auth_required, status, remark, version, deleted, created_by, created_time, updated_by, updated_time)
VALUES
('api_health_live', NULL, 'health.live', '存活检查', 'GET', '/health/live', '/health/live', TRUE, FALSE, 'enabled', '系统公开探针', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_health_ready', NULL, 'health.ready', '就绪检查', 'GET', '/health/ready', '/health/ready', TRUE, FALSE, 'enabled', '系统公开探针', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_i18n_system_resources', NULL, 'i18n.system_resources', '系统多语言资源', 'GET', '/api/v1/i18n/system_resources', '/api/v1/i18n/system_resources', TRUE, FALSE, 'enabled', '前端公开资源接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_auth_login', NULL, 'auth.login', '登录', 'POST', '/api/v1/auth/login', '/api/v1/auth/login', TRUE, FALSE, 'enabled', '认证公开接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_auth_refresh', NULL, 'auth.refresh', '刷新令牌', 'POST', '/api/v1/auth/refresh', '/api/v1/auth/refresh', TRUE, FALSE, 'enabled', '认证公开接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_auth_me', NULL, 'auth.me', '当前账号', 'GET', '/api/v1/auth/me', '/api/v1/auth/me', FALSE, TRUE, 'enabled', '认证账号接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_auth_tenants', NULL, 'auth.tenants', '当前账号租户', 'GET', '/api/v1/auth/tenants', '/api/v1/auth/tenants', FALSE, TRUE, 'enabled', '认证账号接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_auth_logout', NULL, 'auth.logout', '退出登录', 'POST', '/api/v1/auth/logout', '/api/v1/auth/logout', FALSE, TRUE, 'enabled', '认证账号接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_auth_logout_all', NULL, 'auth.logout_all', '退出全部终端', 'POST', '/api/v1/auth/logout_all', '/api/v1/auth/logout_all', FALSE, TRUE, 'enabled', '认证账号接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_auth_switch_tenant', NULL, 'auth.switch_tenant', '切换租户', 'POST', '/api/v1/auth/switch_tenant', '/api/v1/auth/switch_tenant', FALSE, TRUE, 'enabled', '认证账号接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP);

INSERT INTO sys_permission_apis (id, tenant_id, api_code, name, http_method, path_pattern, normalized_path, public_access, auth_required, status, remark, version, deleted, created_by, created_time, updated_by, updated_time)
VALUES
('api_i18n_business_translations_get', NULL, 'i18n.business_translations.get', '查询业务翻译', 'GET', '/api/v1/i18n/business_translations/{resource_type}/{resource_id}', '/api/v1/i18n/business_translations/{id}/{id}', FALSE, TRUE, 'enabled', '多语言管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_i18n_business_translations_put', NULL, 'i18n.business_translations.put', '保存业务翻译', 'PUT', '/api/v1/i18n/business_translations/{resource_type}/{resource_id}', '/api/v1/i18n/business_translations/{id}/{id}', FALSE, TRUE, 'enabled', '多语言管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_post', NULL, 'system.tenants.create', '新增租户', 'POST', '/api/v1/system/tenants', '/api/v1/system/tenants', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_get', NULL, 'system.tenants.page', '分页查询租户', 'GET', '/api/v1/system/tenants', '/api/v1/system/tenants', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_id_get', NULL, 'system.tenants.get', '查询租户', 'GET', '/api/v1/system/tenants/{id}', '/api/v1/system/tenants/{id}', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_id_put', NULL, 'system.tenants.update', '修改租户', 'PUT', '/api/v1/system/tenants/{id}', '/api/v1/system/tenants/{id}', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_id_delete', NULL, 'system.tenants.delete', '删除租户', 'DELETE', '/api/v1/system/tenants/{id}', '/api/v1/system/tenants/{id}', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_users_post', NULL, 'hrm.users.create', '新增 HRM 用户', 'POST', '/api/v1/hrm/users', '/api/v1/hrm/users', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_users_get', NULL, 'hrm.users.page', '分页查询 HRM 用户', 'GET', '/api/v1/hrm/users', '/api/v1/hrm/users', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_users_id_get', NULL, 'hrm.users.get', '查询 HRM 用户', 'GET', '/api/v1/hrm/users/{id}', '/api/v1/hrm/users/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_users_id_put', NULL, 'hrm.users.update', '修改 HRM 用户', 'PUT', '/api/v1/hrm/users/{id}', '/api/v1/hrm/users/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_users_id_delete', NULL, 'hrm.users.delete', '删除 HRM 用户', 'DELETE', '/api/v1/hrm/users/{id}', '/api/v1/hrm/users/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_org_tree', NULL, 'hrm.org_tree', '查询组织树', 'GET', '/api/v1/hrm/org_tree', '/api/v1/hrm/org_tree', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_applications_post', NULL, 'permission.applications.create', '新增应用资源', 'POST', '/api/v1/permission/applications', '/api/v1/permission/applications', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_applications_get', NULL, 'permission.applications.page', '分页查询应用资源', 'GET', '/api/v1/permission/applications', '/api/v1/permission/applications', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_menus_post', NULL, 'permission.menus.create', '新增菜单资源', 'POST', '/api/v1/permission/menus', '/api/v1/permission/menus', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_menu_tree', NULL, 'permission.menu_tree', '查询菜单树', 'GET', '/api/v1/permission/menu_tree', '/api/v1/permission/menu_tree', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_buttons_post', NULL, 'permission.buttons.create', '新增按钮资源', 'POST', '/api/v1/permission/buttons', '/api/v1/permission/buttons', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_buttons_get', NULL, 'permission.buttons.page', '查询按钮资源', 'GET', '/api/v1/permission/buttons', '/api/v1/permission/buttons', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_apis_post', NULL, 'permission.apis.create', '新增接口资源', 'POST', '/api/v1/permission/apis', '/api/v1/permission/apis', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_roles_post', NULL, 'permission.roles.create', '新增角色', 'POST', '/api/v1/permission/roles', '/api/v1/permission/roles', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_roles_get', NULL, 'permission.roles.page', '分页查询角色', 'GET', '/api/v1/permission/roles', '/api/v1/permission/roles', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_role_permissions_get', NULL, 'permission.roles.permissions.get', '查询角色授权', 'GET', '/api/v1/permission/roles/{id}/permissions', '/api/v1/permission/roles/{id}/permissions', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_role_permissions_put', NULL, 'permission.roles.permissions.put', '保存角色授权', 'PUT', '/api/v1/permission/roles/{id}/permissions', '/api/v1/permission/roles/{id}/permissions', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_me_permissions', NULL, 'permission.me.permissions', '当前账号权限', 'GET', '/api/v1/permission/me/permissions', '/api/v1/permission/me/permissions', FALSE, TRUE, 'enabled', '权限查询接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_version', NULL, 'permission.version', '权限版本', 'GET', '/api/v1/permission/version', '/api/v1/permission/version', FALSE, TRUE, 'enabled', '权限查询接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP);

INSERT INTO sys_permission_apis (id, tenant_id, api_code, name, http_method, path_pattern, normalized_path, public_access, auth_required, status, remark, version, deleted, created_by, created_time, updated_by, updated_time)
VALUES
('api_permission_applications_id_get', NULL, 'permission.applications.get', '查询应用资源', 'GET', '/api/v1/permission/applications/{id}', '/api/v1/permission/applications/{id}', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_menus_get', NULL, 'permission.menus.page', '查询菜单资源', 'GET', '/api/v1/permission/menus', '/api/v1/permission/menus', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_roles_id_get', NULL, 'permission.roles.get', '查询角色', 'GET', '/api/v1/permission/roles/{id}', '/api/v1/permission/roles/{id}', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_role_inherited_get', NULL, 'permission.roles.inherited_permissions.get', '查询角色继承授权', 'GET', '/api/v1/permission/roles/{id}/inherited_permissions', '/api/v1/permission/roles/{id}/inherited_permissions', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_me_resources', NULL, 'permission.me.resources', '当前账号资源', 'GET', '/api/v1/permission/me/resources', '/api/v1/permission/me/resources', FALSE, TRUE, 'enabled', '权限查询接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_me_menus', NULL, 'permission.me.menus', '当前账号菜单', 'GET', '/api/v1/permission/me/menus', '/api/v1/permission/me/menus', FALSE, TRUE, 'enabled', '权限查询接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_me_buttons', NULL, 'permission.me.buttons', '当前账号按钮', 'GET', '/api/v1/permission/me/buttons', '/api/v1/permission/me/buttons', FALSE, TRUE, 'enabled', '权限查询接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_apis_import', NULL, 'permission.apis.import', '导入接口资源', 'POST', '/api/v1/permission/apis/import', '/api/v1/permission/apis/import', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP);

INSERT INTO sys_permission_apis (id, tenant_id, api_code, name, http_method, path_pattern, normalized_path, public_access, auth_required, status, remark, version, deleted, created_by, created_time, updated_by, updated_time)
VALUES
('api_permission_role_tree', NULL, 'permission.role_tree', '查询角色树', 'GET', '/api/v1/permission/role_tree', '/api/v1/permission/role_tree', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_role_parents_post', NULL, 'permission.roles.parents.set', '设置角色父角色', 'POST', '/api/v1/permission/roles/{id}/parents', '/api/v1/permission/roles/{id}/parents', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_resource_grants_get', NULL, 'permission.resource_grants.get', '查询资源授权', 'GET', '/api/v1/permission/resource_grants', '/api/v1/permission/resource_grants', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_resource_grants_put', NULL, 'permission.resource_grants.put', '保存资源授权', 'PUT', '/api/v1/permission/resource_grants', '/api/v1/permission/resource_grants', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_account_roles_get', NULL, 'permission.accounts.roles.get', '查询账号角色', 'GET', '/api/v1/permission/accounts/{id}/roles', '/api/v1/permission/accounts/{id}/roles', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_permission_account_roles_put', NULL, 'permission.accounts.roles.put', '保存账号角色', 'PUT', '/api/v1/permission/accounts/{id}/roles', '/api/v1/permission/accounts/{id}/roles', FALSE, TRUE, 'enabled', '权限管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP);

INSERT INTO sys_permission_apis (id, tenant_id, api_code, name, http_method, path_pattern, normalized_path, public_access, auth_required, status, remark, version, deleted, created_by, created_time, updated_by, updated_time)
VALUES
('api_system_tenants_id_physical_delete', NULL, 'system.tenants.physical_delete', '物理删除租户', 'DELETE', '/api/v1/system/tenants/{id}/physical', '/api/v1/system/tenants/{id}/physical', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_id_enable_post', NULL, 'system.tenants.enable', '启用租户', 'POST', '/api/v1/system/tenants/{id}/enable', '/api/v1/system/tenants/{id}/enable', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_id_disable_post', NULL, 'system.tenants.disable', '禁用租户', 'POST', '/api/v1/system/tenants/{id}/disable', '/api/v1/system/tenants/{id}/disable', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_tenants_id_suspend_post', NULL, 'system.tenants.suspend', '暂停租户', 'POST', '/api/v1/system/tenants/{id}/suspend', '/api/v1/system/tenants/{id}/suspend', FALSE, TRUE, 'enabled', '租户管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_post', NULL, 'system.auth.accounts.create', '新增账号', 'POST', '/api/v1/system/auth/accounts', '/api/v1/system/auth/accounts', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_get', NULL, 'system.auth.accounts.page', '分页查询账号', 'GET', '/api/v1/system/auth/accounts', '/api/v1/system/auth/accounts', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_get', NULL, 'system.auth.accounts.get', '查询账号', 'GET', '/api/v1/system/auth/accounts/{id}', '/api/v1/system/auth/accounts/{id}', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_put', NULL, 'system.auth.accounts.update', '修改账号', 'PUT', '/api/v1/system/auth/accounts/{id}', '/api/v1/system/auth/accounts/{id}', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_delete', NULL, 'system.auth.accounts.delete', '删除账号', 'DELETE', '/api/v1/system/auth/accounts/{id}', '/api/v1/system/auth/accounts/{id}', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_physical_delete', NULL, 'system.auth.accounts.physical_delete', '物理删除账号', 'DELETE', '/api/v1/system/auth/accounts/{id}/physical', '/api/v1/system/auth/accounts/{id}/physical', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_enable_post', NULL, 'system.auth.accounts.enable', '启用账号', 'POST', '/api/v1/system/auth/accounts/{id}/enable', '/api/v1/system/auth/accounts/{id}/enable', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_disable_post', NULL, 'system.auth.accounts.disable', '禁用账号', 'POST', '/api/v1/system/auth/accounts/{id}/disable', '/api/v1/system/auth/accounts/{id}/disable', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_lock_post', NULL, 'system.auth.accounts.lock', '锁定账号', 'POST', '/api/v1/system/auth/accounts/{id}/lock', '/api/v1/system/auth/accounts/{id}/lock', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_accounts_id_reset_password_post', NULL, 'system.auth.accounts.reset_password', '重置密码', 'POST', '/api/v1/system/auth/accounts/{id}/reset_password', '/api/v1/system/auth/accounts/{id}/reset_password', FALSE, TRUE, 'enabled', '账号管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_account_tenants_post', NULL, 'system.auth.account_tenants.create', '新增账号租户关系', 'POST', '/api/v1/system/auth/account_tenants', '/api/v1/system/auth/account_tenants', FALSE, TRUE, 'enabled', '账号租户接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_account_tenants_get', NULL, 'system.auth.account_tenants.page', '分页查询账号租户关系', 'GET', '/api/v1/system/auth/account_tenants', '/api/v1/system/auth/account_tenants', FALSE, TRUE, 'enabled', '账号租户接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_account_tenants_id_put', NULL, 'system.auth.account_tenants.update', '修改账号租户关系', 'PUT', '/api/v1/system/auth/account_tenants/{id}', '/api/v1/system/auth/account_tenants/{id}', FALSE, TRUE, 'enabled', '账号租户接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_account_tenants_id_delete', NULL, 'system.auth.account_tenants.delete', '删除账号租户关系', 'DELETE', '/api/v1/system/auth/account_tenants/{id}', '/api/v1/system/auth/account_tenants/{id}', FALSE, TRUE, 'enabled', '账号租户接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_sessions_get', NULL, 'system.auth.sessions.page', '分页查询会话', 'GET', '/api/v1/system/auth/sessions', '/api/v1/system/auth/sessions', FALSE, TRUE, 'enabled', '会话管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_system_auth_sessions_id_revoke_post', NULL, 'system.auth.sessions.revoke', '撤销会话', 'POST', '/api/v1/system/auth/sessions/{id}/revoke', '/api/v1/system/auth/sessions/{id}/revoke', FALSE, TRUE, 'enabled', '会话管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_users_id_physical_delete', NULL, 'hrm.users.physical_delete', '物理删除 HRM 用户', 'DELETE', '/api/v1/hrm/users/{id}/physical', '/api/v1/hrm/users/{id}/physical', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_orgs_post', NULL, 'hrm.orgs.create', '新增组织', 'POST', '/api/v1/hrm/orgs', '/api/v1/hrm/orgs', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_orgs_get', NULL, 'hrm.orgs.page', '分页查询组织', 'GET', '/api/v1/hrm/orgs', '/api/v1/hrm/orgs', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_orgs_id_get', NULL, 'hrm.orgs.get', '查询组织', 'GET', '/api/v1/hrm/orgs/{id}', '/api/v1/hrm/orgs/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_orgs_id_put', NULL, 'hrm.orgs.update', '修改组织', 'PUT', '/api/v1/hrm/orgs/{id}', '/api/v1/hrm/orgs/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_orgs_id_delete', NULL, 'hrm.orgs.delete', '删除组织', 'DELETE', '/api/v1/hrm/orgs/{id}', '/api/v1/hrm/orgs/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_orgs_id_physical_delete', NULL, 'hrm.orgs.physical_delete', '物理删除组织', 'DELETE', '/api/v1/hrm/orgs/{id}/physical', '/api/v1/hrm/orgs/{id}/physical', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_posts_post', NULL, 'hrm.posts.create', '新增岗位', 'POST', '/api/v1/hrm/posts', '/api/v1/hrm/posts', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_posts_get', NULL, 'hrm.posts.page', '分页查询岗位', 'GET', '/api/v1/hrm/posts', '/api/v1/hrm/posts', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_posts_id_get', NULL, 'hrm.posts.get', '查询岗位', 'GET', '/api/v1/hrm/posts/{id}', '/api/v1/hrm/posts/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_posts_id_put', NULL, 'hrm.posts.update', '修改岗位', 'PUT', '/api/v1/hrm/posts/{id}', '/api/v1/hrm/posts/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_posts_id_delete', NULL, 'hrm.posts.delete', '删除岗位', 'DELETE', '/api/v1/hrm/posts/{id}', '/api/v1/hrm/posts/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_posts_id_physical_delete', NULL, 'hrm.posts.physical_delete', '物理删除岗位', 'DELETE', '/api/v1/hrm/posts/{id}/physical', '/api/v1/hrm/posts/{id}/physical', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_user_org_posts_post', NULL, 'hrm.user_org_posts.create', '新增任职关系', 'POST', '/api/v1/hrm/user_org_posts', '/api/v1/hrm/user_org_posts', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_user_org_posts_get', NULL, 'hrm.user_org_posts.page', '分页查询任职关系', 'GET', '/api/v1/hrm/user_org_posts', '/api/v1/hrm/user_org_posts', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_user_org_posts_id_get', NULL, 'hrm.user_org_posts.get', '查询任职关系', 'GET', '/api/v1/hrm/user_org_posts/{id}', '/api/v1/hrm/user_org_posts/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_user_org_posts_id_put', NULL, 'hrm.user_org_posts.update', '修改任职关系', 'PUT', '/api/v1/hrm/user_org_posts/{id}', '/api/v1/hrm/user_org_posts/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_user_org_posts_id_delete', NULL, 'hrm.user_org_posts.delete', '删除任职关系', 'DELETE', '/api/v1/hrm/user_org_posts/{id}', '/api/v1/hrm/user_org_posts/{id}', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP),
('api_hrm_user_org_posts_id_physical_delete', NULL, 'hrm.user_org_posts.physical_delete', '物理删除任职关系', 'DELETE', '/api/v1/hrm/user_org_posts/{id}/physical', '/api/v1/hrm/user_org_posts/{id}/physical', FALSE, TRUE, 'enabled', 'HRM 管理接口', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP);
