//! varethic-storage: safe-Rust storage for browser data, the successor path
//! to Chromium's `//sql` usage in `//components/history`, `//components/bookmarks`
//! and `//components/download`.
//!
//! Scope for v0.1: history (visits, search, clear), bookmarks (add/list/remove),
//! downloads (record completions). Backed by SQLite via rusqlite (bundled).
//! No sync, no full-text-search index yet.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visited_at: i64,
    pub visit_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadRecord {
    pub id: i64,
    pub url: String,
    pub path: String,
    pub success: bool,
    pub completed_at: i64,
}

impl Store {
    /// Open (or create) the store at `path`, running migrations.
    pub fn open(path: &Path) -> Result<Store, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    /// In-memory store, primarily for tests.
    pub fn open_in_memory() -> Result<Store, StorageError> {
        let conn = Connection::open_in_memory()?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL DEFAULT '',
                visited_at INTEGER NOT NULL,
                visit_count INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_history_visited ON history(visited_at DESC);
            CREATE TABLE IF NOT EXISTS bookmarks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS downloads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                path TEXT NOT NULL,
                success INTEGER NOT NULL,
                completed_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS profiles (
                name TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    // ---- history ----

    pub fn record_visit(&self, url: &str, title: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO history(url, title, visited_at, visit_count)
             VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(url) DO UPDATE SET
                title = excluded.title,
                visited_at = excluded.visited_at,
                visit_count = visit_count + 1",
            params![url, title, now_unix()],
        )?;
        Ok(())
    }

    pub fn search_history(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, StorageError> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, visited_at, visit_count FROM history
             WHERE url LIKE ?1 OR title LIKE ?1
             ORDER BY visited_at DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![pattern, limit], |r| {
                Ok(HistoryEntry {
                    id: r.get(0)?,
                    url: r.get(1)?,
                    title: r.get(2)?,
                    visited_at: r.get(3)?,
                    visit_count: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn recent_history(&self, limit: u32) -> Result<Vec<HistoryEntry>, StorageError> {
        self.search_history("", limit)
    }

    pub fn clear_history(&self) -> Result<usize, StorageError> {
        Ok(self.conn.execute("DELETE FROM history", [])?)
    }

    // ---- bookmarks ----

    pub fn add_bookmark(&self, url: &str, title: &str) -> Result<i64, StorageError> {
        self.conn.execute(
            "INSERT INTO bookmarks(url, title, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(url) DO UPDATE SET title = excluded.title",
            params![url, title, now_unix()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_bookmarks(&self) -> Result<Vec<Bookmark>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, url, title, created_at FROM bookmarks ORDER BY created_at DESC")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Bookmark {
                    id: r.get(0)?,
                    url: r.get(1)?,
                    title: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn remove_bookmark(&self, id: i64) -> Result<usize, StorageError> {
        Ok(self
            .conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?)
    }

    // ---- profiles ----

    /// Create a profile registry entry (used by the shell to track named profiles).
    pub fn create_profile(&self, name: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO profiles(name, created_at) VALUES (?1, ?2)",
            params![name, now_unix()],
        )?;
        Ok(())
    }

    pub fn list_profiles(&self) -> Result<Vec<String>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM profiles ORDER BY created_at ASC")?;
        let rows = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn delete_profile(&self, name: &str) -> Result<usize, StorageError> {
        Ok(self
            .conn
            .execute("DELETE FROM profiles WHERE name = ?1", params![name])?)
    }

    // ---- settings ----

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---- downloads ----

    pub fn record_download(
        &self,
        url: &str,
        path: &str,
        success: bool,
    ) -> Result<i64, StorageError> {
        self.conn.execute(
            "INSERT INTO downloads(url, path, success, completed_at) VALUES (?1, ?2, ?3, ?4)",
            params![url, path, success as i64, now_unix()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_downloads(&self, limit: u32) -> Result<Vec<DownloadRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, path, success, completed_at FROM downloads
             ORDER BY completed_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| {
                Ok(DownloadRecord {
                    id: r.get(0)?,
                    url: r.get(1)?,
                    path: r.get(2)?,
                    success: r.get::<_, i64>(3)? != 0,
                    completed_at: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_record_search_clear() {
        let s = Store::open_in_memory().unwrap();
        s.record_visit("https://varethic.in.th", "Varethic")
            .unwrap();
        s.record_visit("https://varethic.in.th", "Varethic")
            .unwrap();
        let hits = s.search_history("varethic", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].visit_count, 2);
        assert_eq!(s.recent_history(10).unwrap().len(), 1);
        assert_eq!(s.clear_history().unwrap(), 1);
        assert!(s.recent_history(10).unwrap().is_empty());
    }

    #[test]
    fn bookmarks_add_list_remove() {
        let s = Store::open_in_memory().unwrap();
        let id = s.add_bookmark("https://example.com", "Example").unwrap();
        let list = s.list_bookmarks().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Example");
        s.remove_bookmark(id).unwrap();
        assert!(s.list_bookmarks().unwrap().is_empty());
    }

    #[test]
    fn downloads_record_and_list() {
        let s = Store::open_in_memory().unwrap();
        s.record_download(
            "https://example.com/file.zip",
            "C:/Users/admin/Downloads/file.zip",
            true,
        )
        .unwrap();
        let list = s.list_downloads(10).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].success);
    }

    #[test]
    fn profiles_create_list_delete() {
        let s = Store::open_in_memory().unwrap();
        s.create_profile("Default").unwrap();
        s.create_profile("Work").unwrap();
        s.create_profile("Default").unwrap(); // idempotent
        assert_eq!(s.list_profiles().unwrap(), vec!["Default", "Work"]);
        s.delete_profile("Work").unwrap();
        assert_eq!(s.list_profiles().unwrap(), vec!["Default"]);
    }

    #[test]
    fn settings_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.get_setting("search_url").unwrap(), None);
        s.set_setting(
            "search_url",
            "https://www.google.com/search?q={searchTerms}",
        )
        .unwrap();
        assert_eq!(
            s.get_setting("search_url").unwrap().as_deref(),
            Some("https://www.google.com/search?q={searchTerms}")
        );
        s.set_setting("search_url", "https://duckduckgo.com/?q={searchTerms}")
            .unwrap();
        assert_eq!(
            s.get_setting("search_url").unwrap().as_deref(),
            Some("https://duckduckgo.com/?q={searchTerms}")
        );
    }

    #[test]
    fn persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("browser.db");
        {
            let s = Store::open(&path).unwrap();
            s.record_visit("https://rust-lang.org", "Rust").unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.recent_history(10).unwrap().len(), 1);
    }
}
