-- 修复所有 JSON 字段的空值问题，确保升级兼容性
-- 将 NULL 值、空字符串、以及错误的"null"字符串更新为空数组 JSON

-- 修复 livestreamers 表的 JSON 字段
UPDATE livestreamers 
SET preprocessor = null
WHERE preprocessor = '' OR preprocessor = 'null';

UPDATE livestreamers 
SET segment_processor = null
WHERE segment_processor = '' OR segment_processor = 'null';

UPDATE livestreamers 
SET downloaded_processor = null
WHERE downloaded_processor = '' OR downloaded_processor = 'null';

UPDATE livestreamers 
SET postprocessor = null
WHERE postprocessor = '' OR postprocessor = 'null';

UPDATE livestreamers
SET override = null
WHERE override = '' OR override = 'null';

UPDATE livestreamers
SET excluded_keywords = null
WHERE excluded_keywords = '' OR excluded_keywords = 'null';

UPDATE livestreamers
SET opt_args = null
WHERE opt_args = '' OR opt_args = 'null';

-- 修复 uploadstreamers 表的 JSON 字段
UPDATE uploadstreamers 
SET tags = null
WHERE tags = '' OR tags = 'null';