use crate::db::{DbConnection, Distro as DbDistro};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono;

/// Remote catalog URL - points to GitHub-hosted catalog
#[allow(dead_code)]
pub const CATALOG_URL: &str = 
    "https://raw.githubusercontent.com/v-k-dev/etch/main-nightly-wings/catalog.json";

/// Stability level of the distribution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Stability {
    Stable,
    Testing,
    Experimental,
    Nightly,
}

impl Default for Stability {
    fn default() -> Self {
        Stability::Stable
    }
}

/// A single distribution entry in the catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distro {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: DistroCategory,
    #[serde(default)]
    pub stability: Stability,
    pub download_url: String,
    #[serde(default)]
    pub mirrors: Vec<String>,
    pub sha256: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub description: String,
    pub verified: bool,
}

impl From<DbDistro> for Distro {
    fn from(db_distro: DbDistro) -> Self {
        // Get primary mirror URL
        let mirrors = DbConnection::get_mirrors(&db_distro.id).unwrap_or_default();
        let download_url = mirrors.first()
            .map(|m| m.url.clone())
            .unwrap_or_default();
        
        let category = match db_distro.category.as_str() {
            "ubuntu" => DistroCategory::Ubuntu,
            "fedora" => DistroCategory::Fedora,
            "mint" => DistroCategory::Mint,
            "debian" => DistroCategory::Debian,
            "arch" => DistroCategory::Arch,
            "raspberry" => DistroCategory::Raspberry,
            "suse" => DistroCategory::Suse,
            "gaming" => DistroCategory::Gaming,
            _ => DistroCategory::Other,
        };

        Self {
            id: db_distro.id.clone(),
            name: db_distro.name,
            version: db_distro.version,
            category,
            stability: Stability::Stable, // Default to stable
            download_url,
            mirrors: mirrors.iter().skip(1).map(|m| m.url.clone()).collect(),
            sha256: "PLACEHOLDER_HASH_UPDATE_WITH_REAL_HASH".to_string(),
            size_bytes: db_distro.size_bytes as u64,
            size_human: db_distro.size_human,
            description: db_distro.description,
            verified: db_distro.verified,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DistroCategory {
    Ubuntu,
    Fedora,
    Mint,
    Debian,
    Arch,
    Raspberry,
    Suse,
    Gaming,
    Other,
}

impl DistroCategory {
    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            Self::Ubuntu => "Ubuntu",
            Self::Fedora => "Fedora",
            Self::Mint => "Linux Mint",
            Self::Debian => "Debian",
            Self::Arch => "Arch Linux",
            Self::Raspberry => "Raspberry Pi",
            Self::Suse => "openSUSE",
            Self::Gaming => "Gaming",
            Self::Other => "Other",
        }
    }
}

/// The complete distros catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistrosCatalog {
    pub version: u32,
    pub last_updated: String,
    pub distros: Vec<Distro>,
}

impl DistrosCatalog {
    /// Load the catalog from SQLite database
    pub fn fetch() -> Result<Self> {
        let db_distros = DbConnection::get_distros(None)
            .context("Failed to load distros from database")?;
        
        let distros: Vec<Distro> = db_distros.into_iter()
            .map(|db_distro| db_distro.into())
            .collect();
        
        Ok(DistrosCatalog {
            version: 1,
            last_updated: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            distros,
        })
    }
    
    /// Search distros by query
    pub fn search(query: &str) -> Result<Vec<Distro>> {
        let db_distros = DbConnection::search_distros(query)
            .context("Failed to search distros")?;
        
        Ok(db_distros.into_iter()
            .map(|db_distro| db_distro.into())
            .collect())
    }
    
    /// Get distros by category
    pub fn by_category(category: &DistroCategory) -> Result<Vec<Distro>> {
        let category_str = match category {
            DistroCategory::Ubuntu => "ubuntu",
            DistroCategory::Fedora => "fedora",
            DistroCategory::Mint => "mint",
            DistroCategory::Debian => "debian",
            DistroCategory::Arch => "arch",
            DistroCategory::Raspberry => "raspberry",
            DistroCategory::Suse => "suse",
            DistroCategory::Gaming => "gaming",
            DistroCategory::Other => "other",
        };
        
        let db_distros = DbConnection::get_distros(Some(category_str))
            .context("Failed to load distros by category")?;
        
        Ok(db_distros.into_iter()
            .map(|db_distro| db_distro.into())
            .collect())
    }
    
    /// Legacy hardcoded catalog (kept for fallback)
    #[allow(dead_code)]
    #[allow(dead_code)]
    pub fn load_from_cache(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let catalog: DistrosCatalog = serde_json::from_str(&content)?;
        Ok(catalog)
    }

    /// Save catalog to local cache
    #[allow(dead_code)]
    pub fn save_to_cache(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get all categories present in catalog
    #[allow(dead_code)]
    pub fn categories(&self) -> Vec<DistroCategory> {
        let mut cats: Vec<_> = self.distros.iter().map(|d| d.category.clone()).collect();
        cats.sort_by_key(|c| format!("{:?}", c));
        cats.dedup();
        cats
    }
}
