use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub struct Note {
    pub id: i64,
    pub created_at: i64,
    pub body: String,
}

pub struct Store {
    conn: Connection,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS notes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    body       TEXT    NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts
    USING fts5(body, content='notes', content_rowid='id');
CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, body) VALUES (new.id, new.body);
END;
CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
END;
CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
    INSERT INTO notes_fts(rowid, body) VALUES (new.id, new.body);
END;
"#;

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store { conn })
    }

    pub fn add(&self, body: &str, created_at: i64) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO notes (created_at, body) VALUES (?1, ?2)",
            params![created_at, body],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Most recent `n` notes, returned oldest-first so the newest ends up at the bottom.
    pub fn recent(&self, n: usize) -> rusqlite::Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, body FROM
               (SELECT id, created_at, body FROM notes ORDER BY id DESC LIMIT ?1)
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![n as i64], row_to_note)?;
        rows.collect()
    }

    /// Full-text search. Every word must match. Best matches first.
    /// `hl` wraps matched terms (e.g. ANSI bold on/off).
    pub fn search(&self, words: &[String], hl: (&str, &str)) -> rusqlite::Result<Vec<Note>> {
        let Some(query) = build_fts_query(words) else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.created_at, highlight(notes_fts, 0, ?1, ?2)
               FROM notes_fts JOIN notes n ON n.id = notes_fts.rowid
              WHERE notes_fts MATCH ?3
              ORDER BY rank",
        )?;
        let rows = stmt.query_map(params![hl.0, hl.1, query], row_to_note)?;
        rows.collect()
    }

    /// Delete a note; returns the deleted note's body if it existed.
    pub fn delete(&self, id: i64) -> rusqlite::Result<Option<String>> {
        let body: Option<String> = self
            .conn
            .query_row("SELECT body FROM notes WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?;
        if body.is_some() {
            self.conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        }
        Ok(body)
    }
}

fn row_to_note(r: &rusqlite::Row) -> rusqlite::Result<Note> {
    Ok(Note { id: r.get(0)?, created_at: r.get(1)?, body: r.get(2)? })
}

/// Turn user words into a safe FTS5 query: each word becomes a quoted string
/// (so punctuation can't break the syntax), a trailing `*` means prefix match,
/// and words are implicitly ANDed.
pub fn build_fts_query(words: &[String]) -> Option<String> {
    let terms: Vec<String> = words
        .iter()
        .flat_map(|w| w.split_whitespace())
        .filter_map(|w| {
            let (core, prefix) = match w.strip_suffix('*') {
                Some(c) => (c, true),
                None => (w, false),
            };
            if !core.chars().any(char::is_alphanumeric) {
                return None;
            }
            let quoted = format!("\"{}\"", core.replace('"', "\"\""));
            Some(if prefix { quoted + "*" } else { quoted })
        })
        .collect();
    if terms.is_empty() { None } else { Some(terms.join(" ")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn query_builder() {
        assert_eq!(build_fts_query(&s(&["milk"])).unwrap(), r#""milk""#);
        assert_eq!(build_fts_query(&s(&["mil*", "eggs"])).unwrap(), r#""mil"* "eggs""#);
        assert_eq!(build_fts_query(&s(&["say \"hi\""])).unwrap(), r#""say" """hi""""#);
        assert_eq!(build_fts_query(&s(&["-", "*"])), None);
    }
}
