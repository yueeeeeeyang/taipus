-- 创建业务数据多语言翻译表。
-- 该表只保存用户录入或业务产生的数据翻译，不存放系统固定文案。
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
    created_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_by VARCHAR(64) NOT NULL,
    updated_time TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    deleted_by VARCHAR(64) NULL,
    deleted_time TIMESTAMP(6) NULL,
    active_translation_key VARCHAR(256)
        GENERATED ALWAYS AS (
            CASE
                WHEN deleted = FALSE THEN CONCAT(resource_type, '#', resource_id, '#', field_name, '#', locale)
                ELSE NULL
            END
        ) STORED,
    PRIMARY KEY (id),
    UNIQUE KEY uk_business_translations_active_key (active_translation_key),
    KEY idx_business_translations_lookup (resource_type, resource_id, locale, deleted),
    KEY idx_business_translations_field_locale (resource_type, field_name, locale, deleted)
);
