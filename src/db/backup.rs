use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use chrono::Local;

/// Create a backup of the database
pub fn backup_database(db_path: &Path) -> Result<PathBuf> {
    if !db_path.exists() {
        return Err(anyhow::anyhow!("Database file does not exist"));
    }
    
    // Create backup directory
    let backup_dir = db_path.parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?
        .join("backups");
    
    fs::create_dir_all(&backup_dir)
        .context("Failed to create backup directory")?;
    
    // Generate backup filename with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("cache_backup_{}.db", timestamp);
    let backup_path = backup_dir.join(backup_filename);
    
    // Copy database file
    fs::copy(db_path, &backup_path)
        .context("Failed to copy database file")?;
    
    println!("✓ Database backed up to: {}", backup_path.display());
    
    // Clean up old backups (keep only last 10)
    cleanup_old_backups(&backup_dir, 10)?;
    
    Ok(backup_path)
}

/// Restore database from a backup
pub fn restore_database(backup_path: &Path, db_path: &Path) -> Result<()> {
    if !backup_path.exists() {
        return Err(anyhow::anyhow!("Backup file does not exist"));
    }
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create database directory")?;
    }
    
    // Copy backup to database location
    fs::copy(backup_path, db_path)
        .context("Failed to restore database from backup")?;
    
    println!("✓ Database restored from: {}", backup_path.display());
    
    Ok(())
}

/// List available backups
pub fn list_backups(db_path: &Path) -> Result<Vec<PathBuf>> {
    let backup_dir = db_path.parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?
        .join("backups");
    
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut backups = Vec::new();
    
    for entry in fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |e| e == "db") {
            backups.push(path);
        }
    }
    
    // Sort by modification time (newest first)
    backups.sort_by(|a, b| {
        let a_time = fs::metadata(a).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let b_time = fs::metadata(b).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });
    
    Ok(backups)
}

/// Clean up old backups, keeping only the specified number
fn cleanup_old_backups(backup_dir: &Path, keep_count: usize) -> Result<()> {
    let backups = fs::read_dir(backup_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().is_file() && 
            entry.path().extension().map_or(false, |e| e == "db")
        })
        .collect::<Vec<_>>();
    
    if backups.len() <= keep_count {
        return Ok(());
    }
    
    // Sort by modification time (oldest first)
    let mut sorted_backups = backups;
    sorted_backups.sort_by(|a, b| {
        let a_time = fs::metadata(a.path()).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let b_time = fs::metadata(b.path()).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        a_time.cmp(&b_time)
    });
    
    // Remove old backups
    let to_remove = sorted_backups.len() - keep_count;
    for entry in sorted_backups.iter().take(to_remove) {
        if let Err(e) = fs::remove_file(entry.path()) {
            eprintln!("Warning: Failed to remove old backup {}: {}", entry.path().display(), e);
        }
    }
    
    Ok(())
}

/// Export database to JSON (for human-readable backup)
pub fn export_to_json(db_path: &Path) -> Result<PathBuf> {
    use rusqlite::Connection;
    use serde_json::json;
    
    let conn = Connection::open(db_path)
        .context("Failed to open database")?;
    
    // Read distros
    let mut stmt = conn.prepare("SELECT id, name, version, category, download_url, sha256, size_bytes, size_human, description, verified, popularity FROM distros")?;
    let distros = stmt.query_map([], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "version": row.get::<_, String>(2)?,
            "category": row.get::<_, String>(3)?,
            "download_url": row.get::<_, String>(4)?,
            "sha256": row.get::<_, String>(5)?,
            "size_bytes": row.get::<_, i64>(6)?,
            "size_human": row.get::<_, String>(7)?,
            "description": row.get::<_, String>(8)?,
            "verified": row.get::<_, bool>(9)?,
            "popularity": row.get::<_, i64>(10)?
        }))
    })?
    .collect::<Result<Vec<_>, _>>()?;
    
    // Create export file
    let export_dir = db_path.parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid database path"))?
        .join("backups");
    
    fs::create_dir_all(&export_dir)?;
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let export_filename = format!("catalog_export_{}.json", timestamp);
    let export_path = export_dir.join(export_filename);
    
    let export_data = json!({
        "version": "1.0",
        "exported_at": Local::now().to_rfc3339(),
        "distros": distros
    });
    
    fs::write(&export_path, serde_json::to_string_pretty(&export_data)?)
        .context("Failed to write export file")?;
    
    println!("✓ Database exported to: {}", export_path.display());
    
    Ok(export_path)
}
