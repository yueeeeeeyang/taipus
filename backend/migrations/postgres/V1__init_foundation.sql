-- 初始化后端基础底座元数据表。
-- 该表用于记录平台基础能力版本等低频元数据，同时验证统一基础字段、逻辑删除和索引约定。
-- PostgreSQL 脚本与 MySQL 脚本必须保持相同版本号，方便审计跨数据库兼容状态。
CREATE TABLE app_metadata (
    id VARCHAR(64) NOT NULL,
    metadata_key VARCHAR(128) NOT NULL,
    metadata_value TEXT NULL,
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

CREATE UNIQUE INDEX uk_app_metadata_key_deleted
    ON app_metadata (metadata_key, deleted);

CREATE INDEX idx_app_metadata_deleted_updated_time
    ON app_metadata (deleted, updated_time);

-- 默认元数据用于标记当前底座 migration 已初始化，后续可用于启动期诊断和兼容性检查。
INSERT INTO app_metadata (
    id,
    metadata_key,
    metadata_value,
    version,
    deleted,
    created_by,
    updated_by
) VALUES (
    'backend_foundation_version',
    'backend_foundation_version',
    '202604280001',
    1,
    FALSE,
    'system',
    'system'
);
