mod core;
mod io;
mod ui;

use gtk4::prelude::*;
use gtk4::Application;

const APP_ID: &str = "org.etch.Etch";
const VERSION: &str = "0.1: NIGHTLY (Wings)";
#[allow(dead_code)]
const VERSION_CODE: &str = "3";

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

    // Root check removed from startup - will be checked when write operation starts
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(ui::build_ui);

    let exit_code = app.run();

    std::process::exit(exit_code.into())
}
