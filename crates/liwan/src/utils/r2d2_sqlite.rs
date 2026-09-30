use crate::utils::hash::db_name;
use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};

pub struct SqliteConnectionManager {
    source: PathBuf,
}

impl SqliteConnectionManager {
    pub fn file(path: impl AsRef<Path>) -> Self {
        Self { source: path.as_ref().to_path_buf() }
    }

    pub fn memory() -> Self {
        Self { source: format!("file:{}?mode=memory&cache=shared", db_name()).into() }
    }
}

impl r2d2::ManageConnection for SqliteConnectionManager {
    type Connection = Connection;
    type Error = rusqlite::Error;

    fn connect(&self) -> Result<Connection> {
        let connection = Connection::open(&self.source)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(connection)
    }

    fn is_valid(&self, _conn: &mut Connection) -> Result<()> {
        Ok(())
    }

    fn has_broken(&self, _: &mut Connection) -> bool {
        false
    }
}
