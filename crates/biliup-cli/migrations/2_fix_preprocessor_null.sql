-- 修复所有 JSON 字段的空值问题，确保升级兼容性
-- 将 NULL 值、空字符串、以及错误的"null"字符串更新为空数组 JSON

-- 修复 livestreamers 表的 JSON 字段
UPDATE livestreamers 
SET preprocessor = '[]' 
WHERE preprocessor IS NULL OR preprocessor = '' OR preprocessor = 'null';

UPDATE livestreamers 
SET segment_processor = '[]' 
WHERE segment_processor IS NULL OR segment_processor = '' OR segment_processor = 'null';

UPDATE livestreamers 
SET downloaded_processor = '[]' 
WHERE downloaded_processor IS NULL OR downloaded_processor = '' OR downloaded_processor = 'null';

UPDATE livestreamers 
SET postprocessor = '[]' 
WHERE postprocessor IS NULL OR postprocessor = '' OR postprocessor = 'null';

UPDATE livestreamers
SET override = ''
WHERE override IS NULL OR override = 'null';

UPDATE livestreamers
SET excluded_keywords = ''
WHERE excluded_keywords IS NULL OR excluded_keywords = 'null';

UPDATE livestreamers
SET opt_args = ''
WHERE opt_args IS NULL OR opt_args = 'null';

-- 修复 uploadstreamers 表的 JSON 字段
UPDATE uploadstreamers 
SET tags = '[]' 
WHERE tags IS NULL OR tags = '' OR tags = 'null';