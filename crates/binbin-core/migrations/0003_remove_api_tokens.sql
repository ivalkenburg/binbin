DELETE FROM objects
    WHERE kind = 'credential' AND json_extract(data, '$.session') = 0;
