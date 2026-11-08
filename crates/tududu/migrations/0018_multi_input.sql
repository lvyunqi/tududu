-- 一个任务可以带多张参考图
CREATE TABLE job_inputs (
    job_id   UUID NOT NULL REFERENCES jobs (id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES assets (id) ON DELETE CASCADE,
    position SMALLINT NOT NULL,
    PRIMARY KEY (job_id, position)
);

CREATE INDEX idx_job_inputs_job ON job_inputs (job_id);

-- 老任务按单图回填
INSERT INTO job_inputs (job_id, asset_id, position)
SELECT id, input_asset_id, 0 FROM jobs WHERE input_asset_id IS NOT NULL;

-- 会话消息也记一下这一轮用到的所有参考图，方便前端一起显示
ALTER TABLE session_messages ADD COLUMN input_asset_ids UUID[] NOT NULL DEFAULT '{}';

UPDATE session_messages
   SET input_asset_ids = ARRAY[input_asset_id]
 WHERE input_asset_id IS NOT NULL;
