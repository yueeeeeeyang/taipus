-- 创建 HRM 人力资源主数据表。
-- HRM 首版只维护用户、组织、岗位和任职关系主数据，不承接登录认证或权限授权语义。
CREATE TABLE hrm_users (
    id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL,
    employee_no VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    name_full_pinyin VARCHAR(256) NOT NULL,
    name_simple_pinyin VARCHAR(128) NOT NULL,
    mobile VARCHAR(32) NULL,
    email VARCHAR(255) NULL,
    sort_no BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP(6) NULL,
    active_employee_key VARCHAR(160)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE THEN CONCAT(tenant_id, '#', employee_no)
                ELSE NULL
            END
        ) STORED,
    PRIMARY KEY (id),
    UNIQUE KEY uk_hrm_users_active_employee (active_employee_key),
    KEY idx_hrm_users_status (tenant_id, deleted, status),
    KEY idx_hrm_users_sort (tenant_id, deleted, sort_no),
    KEY idx_hrm_users_updated_time (tenant_id, deleted, updated_time),
    KEY idx_hrm_users_name (tenant_id, deleted, name),
    KEY idx_hrm_users_name_full_pinyin (tenant_id, deleted, name_full_pinyin),
    KEY idx_hrm_users_name_simple_pinyin (tenant_id, deleted, name_simple_pinyin)
);

-- 组织机构只允许分部和部门两类，层级合法性由 service 显式校验。
CREATE TABLE hrm_orgs (
    id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL,
    parent_id VARCHAR(64) NULL,
    org_code VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    name_full_pinyin VARCHAR(256) NOT NULL,
    name_simple_pinyin VARCHAR(128) NOT NULL,
    org_type VARCHAR(32) NOT NULL,
    sort_no BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP(6) NULL,
    active_org_key VARCHAR(160)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE THEN CONCAT(tenant_id, '#', org_code)
                ELSE NULL
            END
        ) STORED,
    PRIMARY KEY (id),
    UNIQUE KEY uk_hrm_orgs_active_code (active_org_key),
    KEY idx_hrm_orgs_tree (tenant_id, parent_id, deleted, sort_no),
    KEY idx_hrm_orgs_status (tenant_id, deleted, status),
    KEY idx_hrm_orgs_sort (tenant_id, deleted, sort_no),
    KEY idx_hrm_orgs_name (tenant_id, deleted, name),
    KEY idx_hrm_orgs_name_full_pinyin (tenant_id, deleted, name_full_pinyin),
    KEY idx_hrm_orgs_name_simple_pinyin (tenant_id, deleted, name_simple_pinyin)
);

-- 岗位是租户级主数据，具体任职组织通过 hrm_user_org_posts 表表达。
CREATE TABLE hrm_posts (
    id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL,
    post_code VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    name_full_pinyin VARCHAR(256) NOT NULL,
    name_simple_pinyin VARCHAR(128) NOT NULL,
    sort_no BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP(6) NULL,
    active_post_key VARCHAR(160)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE THEN CONCAT(tenant_id, '#', post_code)
                ELSE NULL
            END
        ) STORED,
    PRIMARY KEY (id),
    UNIQUE KEY uk_hrm_posts_active_code (active_post_key),
    KEY idx_hrm_posts_status (tenant_id, deleted, status),
    KEY idx_hrm_posts_sort (tenant_id, deleted, sort_no),
    KEY idx_hrm_posts_updated_time (tenant_id, deleted, updated_time),
    KEY idx_hrm_posts_name (tenant_id, deleted, name),
    KEY idx_hrm_posts_name_full_pinyin (tenant_id, deleted, name_full_pinyin),
    KEY idx_hrm_posts_name_simple_pinyin (tenant_id, deleted, name_simple_pinyin)
);

-- 用户组织岗位关系支持多组织多岗位，并用数据库唯一约束兜底主组织和主岗位唯一性。
CREATE TABLE hrm_user_org_posts (
    id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL,
    user_id VARCHAR(64) NOT NULL,
    org_id VARCHAR(64) NOT NULL,
    post_id VARCHAR(64) NOT NULL,
    primary_org BOOLEAN NOT NULL DEFAULT FALSE,
    primary_post BOOLEAN NOT NULL DEFAULT FALSE,
    sort_no BIGINT NOT NULL DEFAULT 0,
    version BIGINT NOT NULL DEFAULT 1,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_by VARCHAR(64) NOT NULL,
    created_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP(6) NULL,
    active_relation_key VARCHAR(320)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE THEN CONCAT(tenant_id, '#', user_id, '#', org_id, '#', post_id)
                ELSE NULL
            END
        ) STORED,
    active_primary_org_key VARCHAR(160)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE AND primary_org = TRUE THEN CONCAT(tenant_id, '#', user_id)
                ELSE NULL
            END
        ) STORED,
    active_primary_post_key VARCHAR(160)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE AND primary_post = TRUE THEN CONCAT(tenant_id, '#', user_id)
                ELSE NULL
            END
        ) STORED,
    PRIMARY KEY (id),
    UNIQUE KEY uk_hrm_user_org_posts_active_relation (active_relation_key),
    UNIQUE KEY uk_hrm_user_org_posts_primary_org (active_primary_org_key),
    UNIQUE KEY uk_hrm_user_org_posts_primary_post (active_primary_post_key),
    KEY idx_hrm_user_org_posts_user (tenant_id, user_id, deleted),
    KEY idx_hrm_user_org_posts_org (tenant_id, org_id, deleted),
    KEY idx_hrm_user_org_posts_post (tenant_id, post_id, deleted),
    KEY idx_hrm_user_org_posts_sort (tenant_id, deleted, sort_no)
);
