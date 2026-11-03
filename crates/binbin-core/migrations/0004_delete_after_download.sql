-- Existing shares retain their original download and history policy.
UPDATE objects SET data = json_set(data, '$.delete_after_download', json('false'))
WHERE kind = 'share' AND json_type(data, '$.delete_after_download') IS NULL;
