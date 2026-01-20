use crate::db::{DbConnection, Distro, Mirror};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct JsonDistro {
    id: String,
    name: String,
    version: String,
    category: String,
    download_url: String,
    #[serde(default)]
    mirrors: Vec<String>,
    #[serde(default)]
    sha256: String,
    size_bytes: i64,
    size_human: String,
    description: String,
    verified: bool,
}

#[derive(Debug, Deserialize)]
struct JsonCatalog {
    distros: Vec<JsonDistro>,
}

pub fn migrate_json_to_db() -> Result<()> {
    // Create backup before migration if database exists
    let db_path = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
        .join("etch")
        .join("cache.db");
    
    if db_path.exists() {
        println!("Creating backup before migration...");
        if let Err(e) = crate::db::backup_database(&db_path) {
            eprintln!("Warning: Failed to create backup: {}", e);
            // Continue anyway - backup failure shouldn't stop migration
        }
    }
    
    // Load JSON catalog from embedded data
    let json_data = include_str!("../../catalog.json");
    let catalog: JsonCatalog = serde_json::from_str(json_data)
        .context("Failed to parse catalog.json")?;

    println!("Migrating {} distros to SQLite...", catalog.distros.len());

    let today = Utc::now().format("%Y-%m-%d").to_string();

    for json_distro in catalog.distros {
        // Convert to DB model
        let distro = Distro {
            id: json_distro.id.clone(),
            name: json_distro.name,
            version: json_distro.version,
            category: json_distro.category,
            description: json_distro.description,
            size_bytes: json_distro.size_bytes,
            size_human: json_distro.size_human,
            verified: json_distro.verified,
            date_added: today.clone(),
            popularity: calculate_popularity(&json_distro.id),
        };

        DbConnection::insert_distro(&distro)?;

        // Insert primary mirror (download_url)
        let primary_mirror = Mirror {
            id: 0, // Auto-generated
            distro_id: json_distro.id.clone(),
            url: json_distro.download_url,
            region: "Global".to_string(),
            priority: 1,
            status: "ok".to_string(),
        };
        DbConnection::insert_mirror(&primary_mirror)?;

        // Insert additional mirrors if present
        for (idx, mirror_url) in json_distro.mirrors.iter().enumerate() {
            let mirror = Mirror {
                id: 0,
                distro_id: json_distro.id.clone(),
                url: mirror_url.clone(),
                region: detect_region(mirror_url),
                priority: (idx + 2) as i32,
                status: "ok".to_string(),
            };
            DbConnection::insert_mirror(&mirror)?;
        }
    }

    println!("✓ Migration complete!");
    Ok(())
}

/// Calculate popularity score based on distro ID/category
fn calculate_popularity(id: &str) -> i32 {
    // Popular distros get higher scores (shown first)
    match id {
        id if id.starts_with("ubuntu") => 99,
        id if id.starts_with("fedora") => 95,
        id if id.starts_with("debian") => 94,
        id if id.starts_with("arch") => 98,
        id if id.starts_with("mint") => 92,
        id if id.starts_with("pop-os") => 90,
        id if id.starts_with("manjaro") => 88,
        id if id.starts_with("kali") => 85,
        id if id.starts_with("nixos") => 82,
        id if id.starts_with("tails") => 80,
        id if id.starts_with("qubes") => 78,
        _ => 50, // Default for others
    }
}

/// Detect region from mirror URL
fn detect_region(url: &str) -> String {
    if url.contains("us.") || url.contains("usa") || url.contains("america") {
        "US".to_string()
    } else if url.contains("eu.") || url.contains("europe") {
        "EU".to_string()
    } else if url.contains("asia") || url.contains("jp.") || url.contains("cn.") {
        "Asia".to_string()
    } else {
        "Global".to_string()
    }
}

/// Check if migration is needed
pub fn needs_migration() -> Result<bool> {
    match DbConnection::get_distro_count() {
        Ok(count) => Ok(count == 0),
        Err(_) => Ok(true), // If error, assume migration needed
    }
}
