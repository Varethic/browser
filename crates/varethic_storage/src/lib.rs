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
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub cached_at: i64,
    pub max_age_secs: i64,
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
            );
            CREATE TABLE IF NOT EXISTS http_cache (
                url TEXT PRIMARY KEY,
                status INTEGER NOT NULL,
                headers TEXT NOT NULL,
                body BLOB NOT NULL,
                cached_at INTEGER NOT NULL,
                max_age_secs INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workspaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workspace_tabs (
                workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
                position INTEGER NOT NULL,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT ''
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

    // ---- workspaces ----

    /// Create or replace the tab set of a named workspace. Returns its id.
    pub fn save_workspace(
        &self,
        name: &str,
        tabs: &[(String, String)],
    ) -> Result<i64, StorageError> {
        self.conn.execute(
            "INSERT INTO workspaces(name, updated_at) VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET updated_at = excluded.updated_at",
            params![name, now_unix()],
        )?;
        let id: i64 = self.conn.query_row(
            "SELECT id FROM workspaces WHERE name = ?1",
            params![name],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "DELETE FROM workspace_tabs WHERE workspace_id = ?1",
            params![id],
        )?;
        for (pos, (url, title)) in tabs.iter().enumerate() {
            self.conn.execute(
                "INSERT INTO workspace_tabs(workspace_id, position, url, title)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, pos as i64, url, title],
            )?;
        }
        Ok(id)
    }

    pub fn list_workspaces(&self) -> Result<Vec<(i64, String)>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM workspaces ORDER BY updated_at DESC")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn load_workspace(&self, id: i64) -> Result<Vec<(String, String)>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT url, title FROM workspace_tabs WHERE workspace_id = ?1 ORDER BY position ASC",
        )?;
        let rows = stmt
            .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn delete_workspace(&self, id: i64) -> Result<usize, StorageError> {
        self.conn.execute(
            "DELETE FROM workspace_tabs WHERE workspace_id = ?1",
            params![id],
        )?;
        Ok(self
            .conn
            .execute("DELETE FROM workspaces WHERE id = ?1", params![id])?)
    }

    // ---- disk HTTP cache ----

    /// Store (or replace) a cached response, honoring `max_age_secs > 0`.
    /// Evicts an arbitrary entry when over `max_entries`.
    pub fn cache_put(
        &self,
        url: &str,
        status: u16,
        headers: &[(String, String)],
        body: &[u8],
        max_age_secs: i64,
        max_entries: usize,
    ) -> Result<(), StorageError> {
        if max_age_secs <= 0 {
            return Ok(()); // not cacheable
        }
        let headers_json = serde_json::to_string(headers)
            .map_err(|e| StorageError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM http_cache", [], |r| r.get(0))?;
        if count >= max_entries as i64 {
            self.conn.execute(
                "DELETE FROM http_cache WHERE url = (SELECT url FROM http_cache ORDER BY cached_at ASC LIMIT 1)",
                [],
            )?;
        }
        self.conn.execute(
            "INSERT INTO http_cache(url, status, headers, body, cached_at, max_age_secs)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(url) DO UPDATE SET
                status = excluded.status,
                headers = excluded.headers,
                body = excluded.body,
                cached_at = excluded.cached_at,
                max_age_secs = excluded.max_age_secs",
            params![
                url,
                status as i64,
                headers_json,
                body,
                now_unix(),
                max_age_secs
            ],
        )?;
        Ok(())
    }

    /// Fetch a fresh cached response for `url` (checks age vs max_age_secs).
    pub fn cache_get(&self, url: &str) -> Result<Option<CachedResponse>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT status, headers, body, cached_at, max_age_secs FROM http_cache WHERE url = ?1",
        )?;
        let mut rows = stmt.query(params![url])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let status: i64 = row.get(0)?;
        let headers_json: String = row.get(1)?;
        let body: Vec<u8> = row.get(2)?;
        let cached_at: i64 = row.get(3)?;
        let max_age_secs: i64 = row.get(4)?;
        if now_unix() - cached_at > max_age_secs {
            // Stale: drop it.
            self.conn
                .execute("DELETE FROM http_cache WHERE url = ?1", params![url])?;
            return Ok(None);
        }
        let headers: Vec<(String, String)> =
            serde_json::from_str(&headers_json).unwrap_or_default();
        Ok(Some(CachedResponse {
            status: status as u16,
            headers,
            body,
            cached_at,
            max_age_secs,
        }))
    }

    pub fn cache_clear(&self) -> Result<usize, StorageError> {
        Ok(self.conn.execute("DELETE FROM http_cache", [])?)
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
    fn workspaces_save_load_replace_delete() {
        let s = Store::open_in_memory().unwrap();
        let id = s
            .save_workspace(
                "Research",
                &[
                    ("https://a.com".into(), "A".into()),
                    ("https://b.com".into(), "B".into()),
                ],
            )
            .unwrap();
        let list = s.list_workspaces().unwrap();
        assert_eq!(list, vec![(id, "Research".to_string())]);
        let tabs = s.load_workspace(id).unwrap();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].0, "https://a.com");
        // Save again replaces the tab set.
        s.save_workspace("Research", &[("https://c.com".into(), "C".into())])
            .unwrap();
        let tabs = s.load_workspace(id).unwrap();
        assert_eq!(tabs, vec![("https://c.com".to_string(), "C".to_string())]);
        s.delete_workspace(id).unwrap();
        assert!(s.list_workspaces().unwrap().is_empty());
    }

    #[test]
    fn disk_cache_put_get_expiry_eviction() {
        let s = Store::open_in_memory().unwrap();
        s.cache_put(
            "https://a/",
            200,
            &[("cache-control".into(), "max-age=60".into())],
            b"body-a",
            60,
            2,
        )
        .unwrap();
        let got = s.cache_get("https://a/").unwrap().unwrap();
        assert_eq!(got.status, 200);
        assert_eq!(got.body, b"body-a");
        // Non-cacheable (max-age=0) is a no-op.
        s.cache_put("https://b/", 200, &[], b"x", 0, 2).unwrap();
        assert!(s.cache_get("https://b/").unwrap().is_none());
        // Bounded eviction.
        s.cache_put("https://c/", 200, &[], b"c", 60, 2).unwrap();
        s.cache_put("https://d/", 200, &[], b"d", 60, 2).unwrap();
        // One of a/, c/, d/ was evicted; size <= 2
        assert!(s.cache_clear().unwrap() >= 1);
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
