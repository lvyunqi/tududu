-- 管理端操作留痕
CREATE TABLE audit_logs (
    id          BIGSERIAL PRIMARY KEY,
    action      TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id   TEXT,
    detail      TEXT NOT NULL DEFAULT '',
    operator    TEXT NOT NULL DEFAULT 'admin_token',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE audit_logs IS '管理端写操作留痕，补单、调账、改状态都记一条';

CREATE INDEX idx_audit_created ON audit_logs (created_at DESC);
