-- 创建系统租户主数据表。
-- PostgreSQL 使用 partial unique index 只约束未删除租户编码唯一性。
CREATE TABLE sys_tenants (
    id VARCHAR(64) NOT NULL,
    tenant_code VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    name_full_pinyin VARCHAR(256) NOT NULL,
    name_simple_pinyin VARCHAR(128) NOT NULL,
    status VARCHAR(32) NOT NULL,
    isolation_mode VARCHAR(32) NOT NULL,
    primary_domain VARCHAR(255) NULL,
    remark VARCHAR(512) NULL,
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

CREATE UNIQUE INDEX uk_sys_tenants_active_code
    ON sys_tenants (tenant_code)
    WHERE deleted = FALSE;
CREATE INDEX idx_sys_tenants_status ON sys_tenants (deleted, status);
CREATE INDEX idx_sys_tenants_updated_time ON sys_tenants (deleted, updated_time);
CREATE INDEX idx_sys_tenants_name ON sys_tenants (deleted, name);
CREATE INDEX idx_sys_tenants_name_full_pinyin ON sys_tenants (deleted, name_full_pinyin);
CREATE INDEX idx_sys_tenants_name_simple_pinyin ON sys_tenants (deleted, name_simple_pinyin);

-- 默认租户用于开发、测试和单租户部署；生产多租户请求仍应由租户解析中间件显式确认上下文。
INSERT INTO sys_tenants (
    id, tenant_code, name, name_full_pinyin, name_simple_pinyin, status, isolation_mode,
    primary_domain, remark, version, deleted, created_by, created_time, updated_by, updated_time
) VALUES (
    'default', 'default', '默认租户', 'morenzuhu', 'mrzh', 'enabled', 'shared_schema',
    NULL, '系统默认租户', 1, FALSE, 'system', CURRENT_TIMESTAMP, 'system', CURRENT_TIMESTAMP
);
