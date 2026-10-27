CREATE INDEX IF NOT EXISTS removed_files_by_share
    ON objects(json_extract(data, '$.file.share')) WHERE kind = 'removed_file';
