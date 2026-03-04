use rusqlite::{Connection, Result as SqlResult, OpenFlags};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        conn.pragma_update(None, "journal_mode", &"WAL")?;
        conn.pragma_update(None, "synchronous", &"NORMAL")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                root_path   TEXT NOT NULL UNIQUE,
                config      TEXT NOT NULL,
                created_at  INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS documents (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL REFERENCES projects(id),
                rel_path    TEXT NOT NULL,
                layer       TEXT NOT NULL,
                hash        TEXT NOT NULL,
                line_count  INTEGER NOT NULL,
                last_scanned INTEGER NOT NULL,
                health      TEXT NOT NULL,
                UNIQUE(project_id, rel_path)
            );

            CREATE TABLE IF NOT EXISTS doc_chunks (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id TEXT NOT NULL REFERENCES documents(id),
                chunk_text  TEXT NOT NULL,
                start_line  INTEGER NOT NULL,
                end_line    INTEGER NOT NULL,
                metadata    TEXT
            );

            CREATE TABLE IF NOT EXISTS conflicts (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL REFERENCES projects(id),
                document_id TEXT NOT NULL REFERENCES documents(id),
                chunk_id    INTEGER REFERENCES doc_chunks(id),
                commit_hash TEXT,
                description TEXT NOT NULL,
                suggestion  TEXT,
                severity    TEXT NOT NULL DEFAULT 'medium',
                status      TEXT NOT NULL DEFAULT 'open',
                created_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS rule_suggestions (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL REFERENCES projects(id),
                cluster_id  TEXT,
                pattern     TEXT NOT NULL,
                frequency   INTEGER NOT NULL,
                suggestion  TEXT NOT NULL,
                golden_example TEXT,
                target_file TEXT NOT NULL,
                status      TEXT NOT NULL DEFAULT 'proposed',
                created_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS gc_history (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL REFERENCES projects(id),
                source_file TEXT NOT NULL,
                archive_file TEXT NOT NULL,
                items_archived INTEGER NOT NULL,
                lines_before INTEGER NOT NULL,
                lines_after INTEGER NOT NULL,
                executed_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS violations (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL REFERENCES projects(id),
                category    TEXT NOT NULL,
                severity    TEXT NOT NULL DEFAULT 'medium',
                file_path   TEXT NOT NULL,
                line_number INTEGER,
                description TEXT NOT NULL,
                rule_ref    TEXT NOT NULL,
                status      TEXT NOT NULL DEFAULT 'open',
                detected_at INTEGER NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}
