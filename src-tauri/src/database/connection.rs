use rusqlite::Connection;
use std::path::Path;
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    connection.execute_batch("PRAGMA secure_delete=ON;")?;
    connection.execute_batch(super::SCHEMA)?;
    crate::database::pricing::initialize(&connection)?;
    Ok(connection)
}
