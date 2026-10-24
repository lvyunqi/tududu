-- 敏感词表。默认留空，由管理端维护，避免把词库写进代码。
CREATE TABLE sensitive_words (
    id         BIGSERIAL PRIMARY KEY,
    word       TEXT NOT NULL UNIQUE,
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE sensitive_words IS '广场内容过滤用的词表，命中即转人工审核';
