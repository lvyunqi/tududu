-- 创作会话：一次带图的连续对话
CREATE TABLE sessions (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    title          TEXT NOT NULL DEFAULT '新会话',
    model          TEXT NOT NULL,
    status         SMALLINT NOT NULL DEFAULT 1,
    last_active_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON COLUMN sessions.status IS '1 正常 / 2 已归档 / 3 已删除';

CREATE INDEX idx_sessions_user ON sessions (user_id, last_active_at DESC);

-- 会话里的消息，一轮修改对应一对用户与助手消息
CREATE TABLE session_messages (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id       UUID NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    role             TEXT NOT NULL,
    content          TEXT NOT NULL DEFAULT '',
    input_asset_id   UUID REFERENCES assets (id) ON DELETE SET NULL,
    output_asset_id  UUID REFERENCES assets (id) ON DELETE SET NULL,
    job_id           UUID REFERENCES jobs (id) ON DELETE SET NULL,
    status           SMALLINT NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON COLUMN session_messages.role IS 'user 或 assistant';
COMMENT ON COLUMN session_messages.status IS '1 等待 / 2 执行中 / 3 完成 / 4 失败';

CREATE INDEX idx_session_messages ON session_messages (session_id, created_at);

ALTER TABLE jobs ADD COLUMN session_id UUID REFERENCES sessions (id) ON DELETE SET NULL;
ALTER TABLE jobs ADD COLUMN message_id UUID REFERENCES session_messages (id) ON DELETE SET NULL;
