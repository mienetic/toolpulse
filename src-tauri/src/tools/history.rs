//! SQLite-backed history of version snapshots.
//!
//! Every full scan persists one row per tool so the UI can show how a tool's
//! version changed over time. The DB lives in the OS app-data directory.

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;

use super::types::*;

/// Async-friendly handle around a single SQLite connection.
///
/// We hold one connection behind a mutex; Toolpulse is a single-user desktop
/// app, so write contention is negligible.
pub struct History {
    conn: Arc<Mutex<Connection>>,
}

impl History {
    /// Open (or create) the database at `path`, initializing the schema.
    pub fn open(path: PathBuf) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                tool_name   TEXT NOT NULL,
                version     TEXT,
                latest      TEXT,
                is_outdated INTEGER NOT NULL,
                checked_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_tool_time
                ON snapshots(tool_name, checked_at);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Persist one row per tool status from a scan.
    pub async fn record(&self, statuses: &[ToolStatus]) -> Result<(), rusqlite::Error> {
        let conn = self.conn.clone();
        let rows: Vec<(String, Option<String>, Option<String>, bool, i64)> = statuses
            .iter()
            .map(|s| {
                (
                    s.name.clone(),
                    s.installed_version.clone(),
                    s.latest_version.clone(),
                    s.is_outdated,
                    s.checked_at,
                )
            })
            .collect();
        // rusqlite::Connection is not Send across all platforms' SQLite builds,
        // but `bundled` is Send; we still spawn a blocking task to be safe.
        tokio::task::spawn_blocking(move || -> Result<(), rusqlite::Error> {
            let conn = conn.blocking_lock();
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO snapshots (tool_name, version, latest, is_outdated, checked_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )?;
                for (name, version, latest, outdated, checked_at) in &rows {
                    stmt.execute(rusqlite::params![
                        name,
                        version,
                        latest,
                        outdated,
                        checked_at,
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
        .await
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))??;
        Ok(())
    }

    /// Return snapshots from the last `days` days, newest first.
    pub async fn recent(&self, days: i64) -> Result<Vec<Snapshot>, rusqlite::Error> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<Snapshot>, rusqlite::Error> {
            let conn = conn.blocking_lock();
            let cutoff = chrono::Utc::now().timestamp() - days * 86_400;
            let mut stmt = conn.prepare(
                "SELECT id, tool_name, version, latest, is_outdated, checked_at
                 FROM snapshots
                 WHERE checked_at >= ?1
                 ORDER BY checked_at DESC",
            )?;
            let rows = stmt.query_map(rusqlite::params![cutoff], |row| {
                Ok(Snapshot {
                    id: row.get(0)?,
                    tool_name: row.get(1)?,
                    version: row.get(2)?,
                    latest: row.get(3)?,
                    is_outdated: row.get::<_, i64>(4)? != 0,
                    checked_at: row.get(5)?,
                })
            })?;
            rows.collect()
        })
        .await
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
    }
}
