-- Add first-class tid_v2 column for Bilibili new partition IDs
ALTER TABLE uploadstreamers ADD COLUMN tid_v2 INTEGER;
