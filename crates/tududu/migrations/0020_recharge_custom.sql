-- 充值改成自定义金额：不再依赖套餐，后台只配置快捷金额。
ALTER TABLE billing_config
    ADD COLUMN IF NOT EXISTS quick_amounts TEXT NOT NULL DEFAULT '1000,2000,5000,10000,20000';

COMMENT ON COLUMN billing_config.quick_amounts IS '快捷充值金额，逗号分隔，单位分';
