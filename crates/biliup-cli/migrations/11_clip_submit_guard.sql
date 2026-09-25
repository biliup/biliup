-- 切片投稿防重复。发布队列只在内存里：投稿请求发出后、记下稿件号之前进程退出（或记账失败），
-- 切片仍是未发布，可以再发一次，B 站上就多出一个稿件。
--
-- submit_state：调投稿接口前记为 'submitting'，submit_job / submit_started_at 是哪个发布任务
-- （进程内编号）、什么时候开始投的；记账成功（state = 'published'）或投稿明确失败时清空。
-- 启动时还停在 'submitting' 的改成 'unknown'（投稿结果未知），再次发布要用户先到 B 站确认。
ALTER TABLE clips ADD COLUMN submit_state TEXT CHECK (submit_state IN ('submitting', 'unknown'));
ALTER TABLE clips ADD COLUMN submit_job INTEGER;
ALTER TABLE clips ADD COLUMN submit_started_at INTEGER;
