CREATE TABLE IF NOT EXISTS objects (
    kind TEXT NOT NULL,
    id TEXT NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (kind, id)
);
CREATE INDEX IF NOT EXISTS files_by_share
    ON objects(json_extract(data, '$.share')) WHERE kind = 'file';
PRAGMA user_version = 1;
