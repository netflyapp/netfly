//! SQLite visit history.

use crate::paths;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::Mutex;

pub struct History {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visit_count: i64,
    pub last_visit: i64,
}

impl History {
    pub fn open() -> Result<Self, String> {
        let path = paths::history_db()?;
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS history (
              url TEXT PRIMARY KEY,
              title TEXT NOT NULL DEFAULT '',
              visit_count INTEGER NOT NULL DEFAULT 1,
              last_visit INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_last ON history(last_visit DESC);
            "#,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn record(&self, url: &str, title: &str) -> Result<(), String> {
        if url.is_empty() || url == "about:blank" {
            return Ok(());
        }
        let now = now_secs();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"
            INSERT INTO history (url, title, visit_count, last_visit)
            VALUES (?1, ?2, 1, ?3)
            ON CONFLICT(url) DO UPDATE SET
              title = CASE WHEN excluded.title != '' THEN excluded.title ELSE history.title END,
              visit_count = history.visit_count + 1,
              last_visit = excluded.last_visit
            "#,
            params![url, title, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let lim = limit.clamp(1, 100) as i64;
        let mut out = Vec::new();
        if query.trim().is_empty() {
            let mut stmt = conn
                .prepare(
                    "SELECT url, title, visit_count, last_visit FROM history
                     ORDER BY last_visit DESC LIMIT ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![lim], |row| {
                    Ok(HistoryEntry {
                        url: row.get(0)?,
                        title: row.get(1)?,
                        visit_count: row.get(2)?,
                        last_visit: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for r in rows {
                out.push(r.map_err(|e| e.to_string())?);
            }
            return Ok(out);
        }

        let like = format!("%{}%", query.trim());
        let mut stmt = conn
            .prepare(
                "SELECT url, title, visit_count, last_visit FROM history
                 WHERE url LIKE ?1 OR title LIKE ?1
                 ORDER BY last_visit DESC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![like, lim], |row| {
                Ok(HistoryEntry {
                    url: row.get(0)?,
                    title: row.get(1)?,
                    visit_count: row.get(2)?,
                    last_visit: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }
}

fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
