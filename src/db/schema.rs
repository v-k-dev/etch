use anyhow::{Context, Result};
use rusqlite::Connection;

const SCHEMA_VERSION: i32 = 1;

pub fn init_database(conn: &Connection) -> Result<()> {
    // Create schema_version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        )",
        [],
    )?;

    // Check current version
    let current_version: i32 = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    if current_version < SCHEMA_VERSION {
        create_tables(conn)?;
        
        // Update schema version
        conn.execute("DELETE FROM schema_version", [])?;
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [SCHEMA_VERSION],
        )?;
    }

    Ok(())
}

fn create_tables(conn: &Connection) -> Result<()> {
    // Distros table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS distros (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            category TEXT NOT NULL,
            description TEXT,
            size_bytes INTEGER NOT NULL,
            size_human TEXT NOT NULL,
            verified INTEGER NOT NULL DEFAULT 0,
            date_added TEXT NOT NULL,
            popularity INTEGER NOT NULL DEFAULT 0,
            search_text TEXT NOT NULL
        )",
        [],
    )
    .context("Failed to create distros table")?;

    // Create FTS5 virtual table for full-text search
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS distros_fts USING fts5(
            id UNINDEXED,
            name,
            version,
            category,
            description,
            content=distros,
            content_rowid=rowid
        )",
        [],
    )
    .context("Failed to create FTS5 table")?;

    // Mirrors table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mirrors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            distro_id TEXT NOT NULL,
            url TEXT NOT NULL,
            region TEXT NOT NULL DEFAULT 'Global',
            priority INTEGER NOT NULL DEFAULT 1,
            status TEXT NOT NULL DEFAULT 'ok',
            last_checked TEXT,
            FOREIGN KEY (distro_id) REFERENCES distros(id) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create mirrors table")?;

    // User-added ISOs table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_added (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            local_path TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            added_date TEXT NOT NULL
        )",
        [],
    )
    .context("Failed to create user_added table")?;

    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_distros_category ON distros(category)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_distros_popularity ON distros(popularity DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_mirrors_distro ON mirrors(distro_id, priority)",
        [],
    )?;

    Ok(())
}

pub fn populate_fts(conn: &Connection) -> Result<()> {
    // Populate FTS5 table from distros
    conn.execute(
        "INSERT INTO distros_fts(id, name, version, category, description)
         SELECT id, name, version, category, description FROM distros",
        [],
    )?;
    Ok(())
}
