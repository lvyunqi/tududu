-- 金额改成四位小数。
--
-- 计费单位不一定都是整数：单价可以配成 0.5 点，余额和流水也要跟着能带小数。
-- 用定点 NUMERIC 而不是浮点，避免累计误差。
ALTER TABLE wallets
    ALTER COLUMN balance TYPE NUMERIC(18, 4) USING balance::numeric(18, 4),
    ALTER COLUMN total_charged TYPE NUMERIC(18, 4) USING total_charged::numeric(18, 4);

ALTER TABLE ledger_entries
    ALTER COLUMN amount TYPE NUMERIC(18, 4) USING amount::numeric(18, 4),
    ALTER COLUMN balance_after TYPE NUMERIC(18, 4) USING balance_after::numeric(18, 4);

ALTER TABLE billing_prices
    ALTER COLUMN price TYPE NUMERIC(18, 4) USING price::numeric(18, 4);

ALTER TABLE jobs
    ALTER COLUMN price TYPE NUMERIC(18, 4) USING price::numeric(18, 4);

ALTER TABLE recharge_plans
    ALTER COLUMN credit TYPE NUMERIC(18, 4) USING credit::numeric(18, 4),
    ALTER COLUMN bonus TYPE NUMERIC(18, 4) USING bonus::numeric(18, 4);

ALTER TABLE pay_orders
    ALTER COLUMN credit TYPE NUMERIC(18, 4) USING credit::numeric(18, 4),
    ALTER COLUMN bonus TYPE NUMERIC(18, 4) USING bonus::numeric(18, 4);
