use rusqlite::{Connection, OpenFlags, Result};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct SqliteConnectionManager {
    source: PathBuf,
    flags: OpenFlags,
}

impl SqliteConnectionManager {
    pub fn file(path: impl AsRef<Path>) -> Self {
        Self { source: path.as_ref().to_path_buf(), flags: OpenFlags::default() }
    }

    pub fn memory() -> Self {
        Self {
            source: format!("file:{}?mode=memory&cache=shared", Uuid::new_v4().to_string()).into(),
            flags: OpenFlags::default(),
        }
    }

    pub fn with_flags(self, flags: OpenFlags) -> Self {
        Self { flags, ..self }
    }
}

impl r2d2::ManageConnection for SqliteConnectionManager {
    type Connection = Connection;
    type Error = rusqlite::Error;

    fn connect(&self) -> Result<Connection> {
        Connection::open_with_flags(&self.source, self.flags)
    }

    fn is_valid(&self, _conn: &mut Connection) -> Result<()> {
        Ok(())
    }

    fn has_broken(&self, _: &mut Connection) -> bool {
        false
    }
}
