-- 创建业务数据多语言翻译表。
-- PostgreSQL 使用 partial unique index 只约束未删除翻译，允许保留历史删除记录。
CREATE TABLE business_translations (
    id VARCHAR(64) NOT NULL,
    resource_type VARCHAR(64) NOT NULL,
    resource_id VARCHAR(64) NOT NULL,
    field_name VARCHAR(64) NOT NULL,
    locale VARCHAR(32) NOT NULL,
    text_value TEXT NOT NULL,
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

CREATE UNIQUE INDEX uk_business_translations_active_key
    ON business_translations (resource_type, resource_id, field_name, locale)
    WHERE deleted = FALSE;

CREATE INDEX idx_business_translations_lookup
    ON business_translations (resource_type, resource_id, locale, deleted);

CREATE INDEX idx_business_translations_field_locale
    ON business_translations (resource_type, field_name, locale, deleted);
