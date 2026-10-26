use crate::error::{Error, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

pub fn get<T: DeserializeOwned>(db: &Connection, kind: &str, id: &str) -> Result<Option<T>> {
    let s: Option<String> = db
        .query_row(
            "SELECT data FROM objects WHERE kind=? AND id=?",
            params![kind, id],
            |r| r.get(0),
        )
        .optional()?;
    s.map(|s| serde_json::from_str(&s).map_err(Error::from))
        .transpose()
}
pub fn all<T: DeserializeOwned>(db: &Connection, kind: &str) -> Result<Vec<T>> {
    let mut st = db.prepare("SELECT data FROM objects WHERE kind=? ORDER BY rowid")?;
    let rows = st.query_map([kind], |r| r.get::<_, String>(0))?;
    rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
}
pub fn put<T: Serialize>(db: &Connection, kind: &str, id: &str, v: &T) -> Result<()> {
    db.execute(
        "INSERT INTO objects(kind,id,data) VALUES(?,?,?)
         ON CONFLICT(kind,id) DO UPDATE SET data=excluded.data",
        params![kind, id, serde_json::to_string(v)?],
    )?;
    Ok(())
}
pub fn remove(db: &Connection, kind: &str, id: &str) -> Result<()> {
    db.execute(
        "DELETE FROM objects WHERE kind=? AND id=?",
        params![kind, id],
    )?;
    Ok(())
}

pub(crate) fn files_for_share(
    db: &Connection,
    share_id: &str,
) -> Result<Vec<crate::models::StoredFile>> {
    let mut statement = db.prepare(
        "SELECT data FROM objects WHERE kind='file'
         AND json_extract(data,'$.share')=? ORDER BY rowid",
    )?;
    statement
        .query_map([share_id], |row| row.get::<_, String>(0))?
        .map(|value| Ok(serde_json::from_str(&value?)?))
        .collect()
}
