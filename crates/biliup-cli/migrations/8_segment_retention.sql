-- 分段的推迟删除与引用。
--
-- 删除点（后处理 rm、边录边传投稿后、过滤删除）遇到被引用的分段、或按「投稿后保留录像」
-- 要多留一段时间的分段，不立即删，而是把 state 标成 pending_delete，由每分钟一次的清理任务
-- 在到期且引用释放后删除。
--
-- delete_after：保留期到期的 Unix 毫秒；NULL = 不按时间等（只等引用释放 / 场次 retain_until）。
ALTER TABLE segments ADD COLUMN delete_after INTEGER;

-- 引用（标记范围、切片等）按场次时间区间登记，一个引用方一行；segments.pin_count 是与之重叠的
-- 引用数，登记 / 撤销引用和分段写入时重算。按区间存而不是直接加减计数：引用可以落在仍在录的
-- 尾部，之后新写出来的分段同样算被引用；同一个引用方重复登记也不会重复计数。
CREATE TABLE segment_pins (
    owner      TEXT    PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES stream_sessions (id) ON DELETE CASCADE,
    from_ms    INTEGER NOT NULL,
    to_ms      INTEGER NOT NULL
);

CREATE INDEX idx_segment_pins_session ON segment_pins (session_id);
