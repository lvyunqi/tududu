-- 钱包。余额单位就是计费单位，整数，不存小数
CREATE TABLE wallets (
    user_id       UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    balance       BIGINT NOT NULL DEFAULT 0 CHECK (balance >= 0),
    total_charged BIGINT NOT NULL DEFAULT 0,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE wallets IS '用户钱包，余额单位由 billing_config 定义';

INSERT INTO wallets (user_id) SELECT id FROM users ON CONFLICT DO NOTHING;

-- 账本只追加，不退不改
CREATE TABLE ledger_entries (
    id            BIGSERIAL PRIMARY KEY,
    user_id       UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL,
    amount        BIGINT NOT NULL,
    balance_after BIGINT NOT NULL,
    ref_type      TEXT,
    ref_id        UUID,
    remark        TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE ledger_entries IS '钱包流水，只追加不修改';
COMMENT ON COLUMN ledger_entries.amount IS '正数入账，负数出账';
COMMENT ON COLUMN ledger_entries.kind IS 'charge 扣费 / refund 退回 / grant 发放 / adjust 调整';

CREATE INDEX idx_ledger_user_created ON ledger_entries (user_id, created_at DESC);

-- 计费单位，全库只有一行
CREATE TABLE billing_config (
    id            SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    unit_name     TEXT NOT NULL DEFAULT '点',
    unit_per_yuan BIGINT NOT NULL DEFAULT 100 CHECK (unit_per_yuan > 0),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE billing_config IS '计费单位的名称与换算，单位可以按需要改成点、米粒、次数';

INSERT INTO billing_config (id) VALUES (1) ON CONFLICT DO NOTHING;

-- 价格表。size 或 quality 为空表示通配
CREATE TABLE billing_prices (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    model      TEXT NOT NULL,
    size       TEXT,
    quality    TEXT,
    price      BIGINT NOT NULL CHECK (price >= 0),
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE NULLS NOT DISTINCT (model, size, quality)
);

COMMENT ON TABLE billing_prices IS '按模型、尺寸、质量配置的单价，空值表示通配';

ALTER TABLE jobs ADD COLUMN price BIGINT NOT NULL DEFAULT 0;

COMMENT ON COLUMN jobs.price IS '本次任务预扣的计费单位数量';
