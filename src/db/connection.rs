use crate::db::{models::*, schema};
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

static DB_CONNECTION: OnceCell<Mutex<Connection>> = OnceCell::new();

pub struct DbConnection;

impl DbConnection {
    /// Get or initialize the database connection
    pub fn get() -> Result<&'static Mutex<Connection>> {
        DB_CONNECTION.get_or_try_init(|| {
            let db_path = Self::get_db_path()?;
            
            // Ensure parent directory exists
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create database directory")?;
            }

            let conn = Connection::open(&db_path)
                .context("Failed to open database connection")?;

            // Enable foreign keys
            conn.execute("PRAGMA foreign_keys = ON", [])?;
            
            // Initialize schema
            schema::init_database(&conn)?;

            Ok(Mutex::new(conn))
        })
    }

    /// Get the database file path
    pub fn get_db_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .context("Could not determine local data directory")?;
        Ok(data_dir.join("etch").join("cache.db"))
    }

    /// Insert a distro into the database
    pub fn insert_distro(distro: &Distro) -> Result<()> {
        let conn = Self::get()?.lock().unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO distros 
             (id, name, version, category, description, size_bytes, size_human, 
              verified, date_added, popularity, search_text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                distro.id,
                distro.name,
                distro.version,
                distro.category,
                distro.description,
                distro.size_bytes,
                distro.size_human,
                distro.verified as i32,
                distro.date_added,
                distro.popularity,
                distro.search_text(),
            ],
        )?;

        Ok(())
    }

    /// Get all distros, optionally filtered by category
    pub fn get_distros(category: Option<&str>) -> Result<Vec<Distro>> {
        let conn = Self::get()?.lock().unwrap();
        
        let query = if let Some(cat) = category {
            format!(
                "SELECT id, name, version, category, description, size_bytes, 
                 size_human, verified, date_added, popularity 
                 FROM distros WHERE category = '{}' 
                 ORDER BY popularity DESC, name",
                cat
            )
        } else {
            "SELECT id, name, version, category, description, size_bytes, 
             size_human, verified, date_added, popularity 
             FROM distros 
             ORDER BY popularity DESC, name".to_string()
        };

        let mut stmt = conn.prepare(&query)?;
        let distros = stmt
            .query_map([], |row| {
                Ok(Distro {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    category: row.get(3)?,
                    description: row.get(4)?,
                    size_bytes: row.get(5)?,
                    size_human: row.get(6)?,
                    verified: row.get::<_, i32>(7)? != 0,
                    date_added: row.get(8)?,
                    popularity: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(distros)
    }

    /// Search distros by text query
    pub fn search_distros(query: &str) -> Result<Vec<Distro>> {
        let conn = Self::get()?.lock().unwrap();
        
        let search_query = format!("%{}%", query.to_lowercase());
        
        let mut stmt = conn.prepare(
            "SELECT id, name, version, category, description, size_bytes, 
             size_human, verified, date_added, popularity 
             FROM distros 
             WHERE search_text LIKE ?1 
             ORDER BY popularity DESC, name"
        )?;

        let distros = stmt
            .query_map([&search_query], |row| {
                Ok(Distro {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    category: row.get(3)?,
                    description: row.get(4)?,
                    size_bytes: row.get(5)?,
                    size_human: row.get(6)?,
                    verified: row.get::<_, i32>(7)? != 0,
                    date_added: row.get(8)?,
                    popularity: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(distros)
    }

    /// Insert or update a mirror
    pub fn insert_mirror(mirror: &Mirror) -> Result<()> {
        let conn = Self::get()?.lock().unwrap();
        
        conn.execute(
            "INSERT INTO mirrors (distro_id, url, region, priority, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                mirror.distro_id,
                mirror.url,
                mirror.region,
                mirror.priority,
                mirror.status,
            ],
        )?;

        Ok(())
    }

    /// Get mirrors for a distro, sorted by priority
    pub fn get_mirrors(distro_id: &str) -> Result<Vec<Mirror>> {
        let conn = Self::get()?.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, distro_id, url, region, priority, status
             FROM mirrors
             WHERE distro_id = ?1
             ORDER BY priority ASC, id ASC"
        )?;

        let mirrors = stmt
            .query_map([distro_id], |row| {
                Ok(Mirror {
                    id: row.get(0)?,
                    distro_id: row.get(1)?,
                    url: row.get(2)?,
                    region: row.get(3)?,
                    priority: row.get(4)?,
                    status: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(mirrors)
    }

    /// Update mirror status
    pub fn update_mirror_status(mirror_id: i64, status: &str) -> Result<()> {
        let conn = Self::get()?.lock().unwrap();
        
        conn.execute(
            "UPDATE mirrors SET status = ?1, last_checked = datetime('now') WHERE id = ?2",
            rusqlite::params![status, mirror_id],
        )?;

        Ok(())
    }

    /// Get distro count
    pub fn get_distro_count() -> Result<usize> {
        let conn = Self::get()?.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM distros", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}
