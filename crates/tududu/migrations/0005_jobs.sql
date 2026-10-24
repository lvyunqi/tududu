-- 图像生成任务
CREATE TABLE jobs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider_id     UUID REFERENCES providers (id) ON DELETE SET NULL,
    channel_id      UUID REFERENCES channels (id) ON DELETE SET NULL,
    model           TEXT NOT NULL,
    prompt          TEXT NOT NULL,
    input_asset_id  UUID REFERENCES assets (id) ON DELETE SET NULL,
    params          JSONB NOT NULL DEFAULT '{}'::jsonb,
    status          SMALLINT NOT NULL DEFAULT 0,
    attempt_count   INT NOT NULL DEFAULT 0,
    max_attempts    INT NOT NULL DEFAULT 3,
    output_asset_id UUID REFERENCES assets (id) ON DELETE SET NULL,
    error_code      INT,
    error_message   TEXT,
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE jobs IS '图像任务';
COMMENT ON COLUMN jobs.status IS '0 排队 / 1 执行中 / 2 成功 / 3 失败 / 4 已取消';

CREATE INDEX idx_jobs_user_created ON jobs (user_id, created_at DESC);
CREATE INDEX idx_jobs_status ON jobs (status);

-- 每次上游调用的记录，用于排障与渠道质量统计
CREATE TABLE job_attempts (
    id          BIGSERIAL PRIMARY KEY,
    job_id      UUID NOT NULL REFERENCES jobs (id) ON DELETE CASCADE,
    attempt_no  INT NOT NULL,
    channel_id  UUID REFERENCES channels (id) ON DELETE SET NULL,
    status      SMALLINT NOT NULL,
    http_status INT,
    latency_ms  INT,
    error_code  TEXT,
    error_body  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (job_id, attempt_no)
);

COMMENT ON TABLE job_attempts IS '单个任务的上游调用记录';
