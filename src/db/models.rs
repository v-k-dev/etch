use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distro {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub description: String,
    pub size_bytes: i64,
    pub size_human: String,
    pub verified: bool,
    pub date_added: String,
    pub popularity: i32,
}

#[derive(Debug, Clone)]
pub struct Mirror {
    pub id: i64,
    pub distro_id: String,
    pub url: String,
    pub region: String,
    pub priority: i32,
    pub status: String,
}

impl Distro {
    pub fn search_text(&self) -> String {
        format!(
            "{} {} {} {}",
            self.name.to_lowercase(),
            self.version.to_lowercase(),
            self.category.to_lowercase(),
            self.description.to_lowercase()
        )
    }
}
