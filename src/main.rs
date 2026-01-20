mod core;
mod download;
mod io;
mod ui;
mod db;

use gtk4::prelude::*;
use gtk4::Application;

const APP_ID: &str = "org.etch.Etch";
pub const VERSION: &str = env!("AUTO_VERSION");
pub const VERSION_CODE: &str = env!("AUTO_VERSION_CODE");
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const GIT_BRANCH: &str = env!("GIT_BRANCH");

// Compose full version info for display
pub fn version_info() -> String {
    let branch = if GIT_BRANCH == "main" || GIT_BRANCH == "master" {
        "STABLE"
    } else if GIT_BRANCH.contains("nightly") {
        "NIGHTLY"
    } else if GIT_BRANCH.contains("dev") {
        "DEV"
    } else {
        GIT_BRANCH
    };
    
    format!("{} · {} ({})", VERSION, branch, &GIT_HASH[..7.min(GIT_HASH.len())])
}

fn main() -> anyhow::Result<()> {
    // Use memory-only GSettings backend to prevent dconf permission errors
    //
    // Rationale:
    // - Etch doesn't require persistent UI settings between sessions
    // - User environments may have mixed permissions (normal user + sudo/pkexec)
    //   which can leave dconf files owned by root, causing write failures
    // - Using memory backend is the clean solution: no state to persist = no writes
    //
    // This is NOT a workaround - it's the correct architecture for a stateless utility.
    std::env::set_var("GSETTINGS_BACKEND", "memory");

    // Initialize database and migrate if needed
    if db::needs_migration()? {
        println!("First run detected - initializing catalog database...");
        db::migrate_json_to_db()?;
        println!("Database initialized with {} distros", db::DbConnection::get_distro_count()?);
    }

    // Root check removed from startup - will be checked when write operation starts
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(ui::build_ui);

    let exit_code = app.run();

    std::process::exit(exit_code.into())
}
