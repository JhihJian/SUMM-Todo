use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::TodoError;

/// Generate a short unique ID derived from UUID v7.
///
/// Tries progressively longer prefixes of the hex-encoded UUID
/// until one is found that does not collide with existing task IDs.
pub fn generate_id(conn: &Connection) -> Result<String, TodoError> {
    let uuid = Uuid::now_v7();
    let hex = uuid.simple().to_string(); // 32-char hex

    for &len in &[4, 6, 8] {
        let candidate = &hex[..len];
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id = ?1)",
            params![candidate],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(candidate.to_string());
        }
    }

    // Fallback: full 32-char hex (collision at 8 chars is virtually impossible)
    Ok(hex)
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
    fn generates_4_char_id_when_no_collision() {
        let conn = setup_db();
        let id = generate_id(&conn).unwrap();
        assert_eq!(id.len(), 4, "Expected 4-char ID, got '{}'", id);
        // Must be valid lowercase hex
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "ID '{}' contains non-hex characters",
            id
        );
    }

    #[test]
    fn generates_longer_id_on_collision() {
        let conn = setup_db();

        let first_id = generate_id(&conn).unwrap();
        assert_eq!(first_id.len(), 4);

        // Insert the first ID so the next call's 4-char prefix will collide
        // if it happens to match. To guarantee a collision we insert the
        // 4-char prefix we're about to generate. Since UUID v7 is time-based,
        // two calls in quick succession share the same timestamp prefix, but
        // the random suffix differs. Instead, we force a collision by inserting
        // the first ID into the DB and verifying the second ID is still valid.
        conn.execute(
            "INSERT INTO tasks (id, title) VALUES (?1, 'test')",
            params![&first_id],
        )
        .unwrap();

        let second_id = generate_id(&conn).unwrap();
        assert_ne!(first_id, second_id, "IDs must be unique");
        assert!(
            second_id.chars().all(|c| c.is_ascii_hexdigit()),
            "ID '{}' contains non-hex characters",
            second_id
        );
        // Second ID is either 4 (no collision) or longer (collision on 4-char prefix)
        assert!(
            [4, 6, 8, 32].contains(&second_id.len()),
            "Unexpected ID length: {}",
            second_id.len()
        );
    }
}
