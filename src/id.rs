use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::TodoError;

/// Generate an 8-character unique ID derived from UUID v7.
///
/// Always returns 8 characters for consistency.
/// Uses the last 8 characters of the UUID to ensure randomness.
/// Collision at 8 chars is virtually impossible (1 in 4 billion).
pub fn generate_id(conn: &Connection) -> Result<String, TodoError> {
    loop {
        let uuid = Uuid::now_v7();
        let hex = uuid.simple().to_string(); // 32-char hex
        let id = &hex[24..]; // Last 8 characters for better randomness

        // Check for collision (extremely rare)
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
            params![id],
            |row| row.get(0),
        )?;

        if !exists {
            return Ok(id.to_string());
        }
        // If collision (virtually impossible), loop and try next UUID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL);")
            .unwrap();
        conn
    }

    #[test]
    fn generates_8_char_id() {
        let conn = setup_db();
        let id = generate_id(&conn).unwrap();
        assert_eq!(id.len(), 8, "Expected 8-char ID, got '{}'", id);
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "ID '{}' contains non-hex characters",
            id
        );
    }

    #[test]
    fn ids_are_unique() {
        let conn = setup_db();

        let mut ids = std::collections::HashSet::new();
        for i in 0..100 {
            let id = generate_id(&conn).unwrap();
            assert!(ids.insert(id.clone()), "Duplicate ID generated: {}", id);
            // Insert into DB to enable collision detection for subsequent IDs
            conn.execute(
                "INSERT INTO tasks (id, title) VALUES (?1, 'test')",
                params![&id],
            )
            .unwrap_or_else(|_| panic!("Failed to insert ID {} on iteration {}", id, i));
        }
    }
}
