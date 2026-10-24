-- 充值套餐
CREATE TABLE recharge_plans (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL,
    amount_fen BIGINT NOT NULL CHECK (amount_fen > 0),
    credit     BIGINT NOT NULL CHECK (credit > 0),
    bonus      BIGINT NOT NULL DEFAULT 0 CHECK (bonus >= 0),
    sort       INT NOT NULL DEFAULT 0,
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE recharge_plans IS '充值套餐，金额以分为单位，避免浮点误差';
COMMENT ON COLUMN recharge_plans.credit IS '到账的计费单位数量';

-- 易支付通道
CREATE TABLE pay_channels (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    gateway_url TEXT NOT NULL,
    pid         TEXT NOT NULL,
    key_cipher  TEXT NOT NULL,
    pay_type    TEXT NOT NULL DEFAULT 'alipay',
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE pay_channels IS '易支付通道，一条记录对应一组商户号与密钥';

CREATE INDEX idx_pay_channels_pid ON pay_channels (pid);

-- 充值订单
CREATE TABLE pay_orders (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_no   TEXT NOT NULL UNIQUE,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    plan_id    UUID REFERENCES recharge_plans (id) ON DELETE SET NULL,
    channel_id UUID REFERENCES pay_channels (id) ON DELETE SET NULL,
    amount_fen BIGINT NOT NULL CHECK (amount_fen > 0),
    credit     BIGINT NOT NULL CHECK (credit > 0),
    bonus      BIGINT NOT NULL DEFAULT 0,
    status     SMALLINT NOT NULL DEFAULT 0,
    subject    TEXT NOT NULL,
    trade_no   TEXT,
    pay_type   TEXT,
    paid_at    TIMESTAMPTZ,
    expire_at  TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON COLUMN pay_orders.status IS '0 待支付 / 1 已支付 / 2 已关闭';

CREATE INDEX idx_pay_orders_user ON pay_orders (user_id, created_at DESC);
CREATE INDEX idx_pay_orders_status ON pay_orders (status);

-- 回调原文留档
CREATE TABLE pay_notifies (
    id            BIGSERIAL PRIMARY KEY,
    order_no      TEXT,
    trade_no      TEXT,
    raw_body      TEXT NOT NULL,
    sign_valid    BOOLEAN NOT NULL,
    verify_result TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE pay_notifies IS '支付回调原文与验签结果，排障用';

-- 同一业务对象同一类型只允许记一次，保证重放安全
CREATE UNIQUE INDEX idx_ledger_ref ON ledger_entries (ref_type, ref_id, kind) WHERE ref_id IS NOT NULL;
