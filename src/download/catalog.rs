use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Remote catalog URL - points to GitHub-hosted catalog
pub const CATALOG_URL: &str = 
    "https://raw.githubusercontent.com/v-k-dev/etch/main/catalog.json";

/// A single distribution entry in the catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distro {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: DistroCategory,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub description: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DistroCategory {
    Ubuntu,
    Fedora,
    Mint,
    Debian,
    Arch,
    Other,
}

impl DistroCategory {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Ubuntu => "Ubuntu",
            Self::Fedora => "Fedora",
            Self::Mint => "Linux Mint",
            Self::Debian => "Debian",
            Self::Arch => "Arch Linux",
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
    /// Fetch the catalog from remote URL
    pub fn fetch() -> Result<Self> {
        let response = reqwest::blocking::get(CATALOG_URL)?;
        let catalog: DistrosCatalog = response.json()?;
        Ok(catalog)
    }

    /// Load catalog from local cache file
    pub fn load_from_cache(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let catalog: DistrosCatalog = serde_json::from_str(&content)?;
        Ok(catalog)
    }

    /// Save catalog to local cache
    pub fn save_to_cache(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get distros by category
    pub fn by_category(&self, category: DistroCategory) -> Vec<&Distro> {
        self.distros
            .iter()
            .filter(|d| d.category == category)
            .collect()
    }

    /// Get all categories present in catalog
    pub fn categories(&self) -> Vec<DistroCategory> {
        let mut cats: Vec<_> = self.distros.iter().map(|d| d.category.clone()).collect();
        cats.sort_by_key(|c| format!("{:?}", c));
        cats.dedup();
        cats
    }
}
