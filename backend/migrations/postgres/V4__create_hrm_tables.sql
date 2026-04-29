-- 创建 HRM 人力资源主数据表。
-- PostgreSQL 使用 partial unique index 只约束未删除主数据，允许保留逻辑删除历史记录。
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
    created_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMPTZ NULL,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX uk_hrm_users_active_employee
    ON hrm_users (tenant_id, employee_no)
    WHERE deleted = FALSE;
CREATE INDEX idx_hrm_users_status ON hrm_users (tenant_id, deleted, status);
CREATE INDEX idx_hrm_users_sort ON hrm_users (tenant_id, deleted, sort_no);
CREATE INDEX idx_hrm_users_updated_time ON hrm_users (tenant_id, deleted, updated_time);
CREATE INDEX idx_hrm_users_name ON hrm_users (tenant_id, deleted, name);
CREATE INDEX idx_hrm_users_name_full_pinyin ON hrm_users (tenant_id, deleted, name_full_pinyin);
CREATE INDEX idx_hrm_users_name_simple_pinyin ON hrm_users (tenant_id, deleted, name_simple_pinyin);

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
    created_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMPTZ NULL,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX uk_hrm_orgs_active_code
    ON hrm_orgs (tenant_id, org_code)
    WHERE deleted = FALSE;
CREATE INDEX idx_hrm_orgs_tree ON hrm_orgs (tenant_id, parent_id, deleted, sort_no);
CREATE INDEX idx_hrm_orgs_status ON hrm_orgs (tenant_id, deleted, status);
CREATE INDEX idx_hrm_orgs_sort ON hrm_orgs (tenant_id, deleted, sort_no);
CREATE INDEX idx_hrm_orgs_name ON hrm_orgs (tenant_id, deleted, name);
CREATE INDEX idx_hrm_orgs_name_full_pinyin ON hrm_orgs (tenant_id, deleted, name_full_pinyin);
CREATE INDEX idx_hrm_orgs_name_simple_pinyin ON hrm_orgs (tenant_id, deleted, name_simple_pinyin);

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
    created_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMPTZ NULL,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX uk_hrm_posts_active_code
    ON hrm_posts (tenant_id, post_code)
    WHERE deleted = FALSE;
CREATE INDEX idx_hrm_posts_status ON hrm_posts (tenant_id, deleted, status);
CREATE INDEX idx_hrm_posts_sort ON hrm_posts (tenant_id, deleted, sort_no);
CREATE INDEX idx_hrm_posts_updated_time ON hrm_posts (tenant_id, deleted, updated_time);
CREATE INDEX idx_hrm_posts_name ON hrm_posts (tenant_id, deleted, name);
CREATE INDEX idx_hrm_posts_name_full_pinyin ON hrm_posts (tenant_id, deleted, name_full_pinyin);
CREATE INDEX idx_hrm_posts_name_simple_pinyin ON hrm_posts (tenant_id, deleted, name_simple_pinyin);

-- 用户组织岗位关系支持多组织多岗位，并用 partial unique index 兜底主组织和主岗位唯一性。
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
    created_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMPTZ NULL,
    PRIMARY KEY (id)
);

CREATE UNIQUE INDEX uk_hrm_user_org_posts_active_relation
    ON hrm_user_org_posts (tenant_id, user_id, org_id, post_id)
    WHERE deleted = FALSE;
CREATE UNIQUE INDEX uk_hrm_user_org_posts_primary_org
    ON hrm_user_org_posts (tenant_id, user_id)
    WHERE deleted = FALSE AND primary_org = TRUE;
CREATE UNIQUE INDEX uk_hrm_user_org_posts_primary_post
    ON hrm_user_org_posts (tenant_id, user_id)
    WHERE deleted = FALSE AND primary_post = TRUE;
CREATE INDEX idx_hrm_user_org_posts_user ON hrm_user_org_posts (tenant_id, user_id, deleted);
CREATE INDEX idx_hrm_user_org_posts_org ON hrm_user_org_posts (tenant_id, org_id, deleted);
CREATE INDEX idx_hrm_user_org_posts_post ON hrm_user_org_posts (tenant_id, post_id, deleted);
CREATE INDEX idx_hrm_user_org_posts_sort ON hrm_user_org_posts (tenant_id, deleted, sort_no);
