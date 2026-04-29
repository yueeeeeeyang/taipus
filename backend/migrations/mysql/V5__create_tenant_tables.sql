-- 创建系统租户主数据表。
-- 首版采用共享库共享表模式，所有业务表通过 tenant_id 隔离租户数据。
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
    created_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP(6) NULL,
    active_tenant_code VARCHAR(64)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE THEN tenant_code
                ELSE NULL
            END
        ) STORED,
    PRIMARY KEY (id),
    UNIQUE KEY uk_sys_tenants_active_code (active_tenant_code),
    KEY idx_sys_tenants_status (deleted, status),
    KEY idx_sys_tenants_updated_time (deleted, updated_time),
    KEY idx_sys_tenants_name (deleted, name),
    KEY idx_sys_tenants_name_full_pinyin (deleted, name_full_pinyin),
    KEY idx_sys_tenants_name_simple_pinyin (deleted, name_simple_pinyin)
);

-- 默认租户用于开发、测试和单租户部署；生产多租户请求仍应由租户解析中间件显式确认上下文。
INSERT INTO sys_tenants (
    id, tenant_code, name, name_full_pinyin, name_simple_pinyin, status, isolation_mode,
    primary_domain, remark, version, deleted, created_by, created_time, updated_by, updated_time
) VALUES (
    'default', 'default', '默认租户', 'morenzuhu', 'mrzh', 'enabled', 'shared_schema',
    NULL, '系统默认租户', 1, FALSE, 'system', CURRENT_TIMESTAMP(6), 'system', CURRENT_TIMESTAMP(6)
);
