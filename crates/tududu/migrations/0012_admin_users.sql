-- 管理员账号。之前的单一令牌没有操作人区分，改成真实账号。
CREATE TABLE admin_users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name  TEXT NOT NULL DEFAULT '',
    role          TEXT NOT NULL DEFAULT 'admin',
    status        SMALLINT NOT NULL DEFAULT 1,
    last_login_at TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON COLUMN admin_users.status IS '1 正常 / 2 停用';
COMMENT ON COLUMN admin_users.role IS '目前只有 admin 一种，预留扩展';

ALTER TABLE audit_logs ADD COLUMN operator_id UUID;

COMMENT ON COLUMN audit_logs.operator_id IS '操作人，走旧令牌时为空';
