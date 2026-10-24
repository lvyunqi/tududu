-- 回收站要能算出保留期，软删的时候记下时间
ALTER TABLE assets ADD COLUMN trashed_at TIMESTAMPTZ;

COMMENT ON COLUMN assets.trashed_at IS '移入回收站的时间，用于计算清理期限';

CREATE INDEX idx_assets_trash ON assets (trashed_at) WHERE status = 2;
