-- Legacy cancelled jobs have no trustworthy start time, so do not report a fabricated elapsed time.
UPDATE render_jobs
SET finished_at=NULL
WHERE state='cancelled' AND started_at IS NULL;
