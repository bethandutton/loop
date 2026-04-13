use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn db_path_public() -> PathBuf {
        Self::db_path()
    }

    fn db_path() -> PathBuf {
        let support_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Herd");
        support_dir.join("herd.db")
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS Repo (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                path            TEXT NOT NULL,
                worktrees_dir   TEXT NOT NULL,
                primary_branch  TEXT NOT NULL DEFAULT 'main',
                preview_port    INTEGER NOT NULL DEFAULT 3000,
                is_active       BOOLEAN NOT NULL DEFAULT 1,
                created_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS Ticket (
                id                      TEXT PRIMARY KEY,
                identifier              TEXT,
                repo_id                 TEXT REFERENCES Repo(id),
                title                   TEXT NOT NULL,
                plan_markdown           TEXT,
                plan_dirty              BOOLEAN NOT NULL DEFAULT 0,
                status                  TEXT NOT NULL DEFAULT 'backlog',
                priority                INTEGER NOT NULL DEFAULT 0,
                cycle_id                TEXT,
                branch_name             TEXT,
                worktree_path           TEXT,
                claude_session_id       TEXT,
                pr_number               INTEGER,
                pr_state                TEXT,
                pr_url                  TEXT,
                last_seen_pr_event_id   TEXT,
                handoff_summary         TEXT,
                tags                    TEXT NOT NULL DEFAULT '[]',
                created_at              TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at              TEXT NOT NULL DEFAULT (datetime('now')),
                done_at                 TEXT
            );

            CREATE TABLE IF NOT EXISTS ClaudeSession (
                id              TEXT PRIMARY KEY,
                ticket_id       TEXT REFERENCES Ticket(id),
                repo_id         TEXT REFERENCES Repo(id),
                pty_pid         INTEGER,
                scrollback_path TEXT NOT NULL,
                state           TEXT NOT NULL DEFAULT 'idle',
                started_at      TEXT NOT NULL DEFAULT (datetime('now')),
                ended_at        TEXT
            );

            CREATE TABLE IF NOT EXISTS ServiceRun (
                id              TEXT PRIMARY KEY,
                repo_id         TEXT REFERENCES Repo(id),
                worktree_path   TEXT NOT NULL,
                script_name     TEXT NOT NULL,
                pty_pid         INTEGER,
                state           TEXT NOT NULL DEFAULT 'stopped',
                started_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- v2 tables: present in v1 schema, unused in v1
            CREATE TABLE IF NOT EXISTS AutoAction (
                id              TEXT PRIMARY KEY,
                ticket_id       TEXT REFERENCES Ticket(id),
                trigger_status  TEXT NOT NULL,
                prompt          TEXT NOT NULL,
                enabled         BOOLEAN NOT NULL DEFAULT 1,
                created_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS AutoActionRun (
                id              TEXT PRIMARY KEY,
                auto_action_id  TEXT REFERENCES AutoAction(id),
                ticket_id       TEXT REFERENCES Ticket(id),
                fired_at        TEXT NOT NULL DEFAULT (datetime('now')),
                outcome         TEXT NOT NULL,
                notes           TEXT
            );

            CREATE TABLE IF NOT EXISTS Settings (
                key     TEXT PRIMARY KEY,
                value   TEXT NOT NULL
            );
            ",
        )?;

        // Add identifier column if missing (migration for existing DBs)
        let has_identifier: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('Ticket') WHERE name='identifier'")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map(|c| c > 0)
            .unwrap_or(false);
        if !has_identifier {
            conn.execute_batch("ALTER TABLE Ticket ADD COLUMN identifier TEXT;")?;
        }

        Ok(())
    }

    /// Upsert a ticket from Linear data. Preserves local-only fields (worktree_path, etc.)
    pub fn upsert_ticket(
        &self,
        id: &str,
        identifier: &str,
        repo_id: &str,
        title: &str,
        status: &str,
        priority: i64,
        tags: &str,
        branch_name: Option<&str>,
        created_at: &str,
        updated_at: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO Ticket (id, identifier, repo_id, title, status, priority, tags, branch_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                identifier = excluded.identifier,
                title = excluded.title,
                status = CASE WHEN Ticket.worktree_path IS NOT NULL THEN Ticket.status ELSE excluded.status END,
                priority = excluded.priority,
                tags = excluded.tags,
                branch_name = COALESCE(excluded.branch_name, Ticket.branch_name),
                updated_at = excluded.updated_at",
            rusqlite::params![id, identifier, repo_id, title, status, priority, tags, branch_name, created_at, updated_at],
        )?;
        Ok(())
    }

    pub fn get_all_tickets(&self, repo_id: &str) -> Result<Vec<TicketRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, identifier, title, status, priority, branch_name, tags, created_at, updated_at
             FROM Ticket WHERE repo_id = ?1"
        )?;
        let rows = stmt.query_map([repo_id], |row| {
            Ok(TicketRow {
                id: row.get(0)?,
                identifier: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                title: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                branch_name: row.get(5)?,
                tags: row.get::<_, String>(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut tickets = Vec::new();
        for row in rows {
            tickets.push(row?);
        }
        Ok(tickets)
    }

    pub fn update_ticket_status(&self, ticket_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE Ticket SET status = ?2, updated_at = datetime('now') WHERE id = ?1",
            [ticket_id, status],
        )?;
        Ok(())
    }

    pub fn update_ticket_branch(
        &self,
        ticket_id: &str,
        branch_name: &str,
        worktree_path: &str,
        session_id: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE Ticket SET branch_name = ?2, worktree_path = ?3, claude_session_id = ?4, updated_at = datetime('now') WHERE id = ?1",
            rusqlite::params![ticket_id, branch_name, worktree_path, session_id],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM Settings WHERE key = ?1")?;
        let result = stmt
            .query_row([key], |row| row.get(0))
            .ok();
        Ok(result)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO Settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub fn has_repos(&self) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM Repo", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn create_repo(
        &self,
        name: &str,
        path: &str,
        worktrees_dir: &str,
        primary_branch: &str,
        preview_port: i64,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO Repo (id, name, path, worktrees_dir, primary_branch, preview_port)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, name, path, worktrees_dir, primary_branch, preview_port],
        )?;
        Ok(id)
    }

    pub fn get_active_repo(&self) -> Result<Option<RepoRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, worktrees_dir, primary_branch, preview_port, is_active, created_at
             FROM Repo WHERE is_active = 1 LIMIT 1",
        )?;
        let result = stmt
            .query_row([], |row| {
                Ok(RepoRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    worktrees_dir: row.get(3)?,
                    primary_branch: row.get(4)?,
                    preview_port: row.get(5)?,
                    is_active: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .ok();
        Ok(result)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TicketRow {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub status: String,
    pub priority: i64,
    pub branch_name: Option<String>,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoRow {
    pub id: String,
    pub name: String,
    pub path: String,
    pub worktrees_dir: String,
    pub primary_branch: String,
    pub preview_port: i64,
    pub is_active: bool,
    pub created_at: String,
}
