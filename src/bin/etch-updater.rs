use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    // This updater is designed to be run via pkexec with elevated privileges
    // Usage: pkexec etch-updater <download-url> <temp-file> <target-binary>
    
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 4 {
        eprintln!("Usage: etch-updater <download-url> <temp-file> <target-binary>");
        std::process::exit(1);
    }
    
    let download_url = &args[1];
    let temp_file = &args[2];
    let target_binary = &args[3];
    
    println!("UPDATE_START");
    println!("Downloading from: {}", download_url);
    
    // Download the new binary
    if let Err(e) = download_file(download_url, temp_file) {
        eprintln!("ERROR Failed to download: {}", e);
        std::process::exit(1);
    }
    
    println!("DOWNLOAD_COMPLETE");
    
    // Verify the downloaded file is executable
    if let Err(e) = verify_binary(temp_file) {
        eprintln!("ERROR Invalid binary: {}", e);
        std::process::exit(1);
    }
    
    println!("VERIFY_COMPLETE");
    
    // Backup the old binary
    let backup_path = format!("{}.backup", target_binary);
    if Path::new(target_binary).exists() {
        if let Err(e) = fs::copy(target_binary, &backup_path) {
            eprintln!("WARNING Failed to create backup: {}", e);
        }
    }
    
    // Install the new binary
    if let Err(e) = fs::copy(temp_file, target_binary) {
        eprintln!("ERROR Failed to install: {}", e);
        // Restore backup if available
        if Path::new(&backup_path).exists() {
            let _ = fs::copy(&backup_path, target_binary);
        }
        std::process::exit(1);
    }
    
    // Set executable permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        if let Err(e) = fs::set_permissions(target_binary, perms) {
            eprintln!("ERROR Failed to set permissions: {}", e);
            std::process::exit(1);
        }
    }
    
    println!("INSTALL_COMPLETE");
    
    // Clean up
    let _ = fs::remove_file(temp_file);
    let _ = fs::remove_file(&backup_path);
    
    println!("UPDATE_SUCCESS");
    Ok(())
}

fn download_file(url: &str, dest: &str) -> anyhow::Result<()> {
    let output = Command::new("curl")
        .args(["-L", "-o", dest, url, "--progress-bar", "--fail"])
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Download failed: {}", error);
    }
    
    Ok(())
}

fn verify_binary(path: &str) -> anyhow::Result<()> {
    // Check if file exists and is not empty
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        anyhow::bail!("Downloaded file is empty");
    }
    
    // Check if it's an ELF binary
    let mut file = fs::File::open(path)?;
    let mut magic = [0u8; 4];
    io::Read::read_exact(&mut file, &mut magic)?;
    
    if &magic != b"\x7fELF" {
        anyhow::bail!("Not a valid ELF binary");
    }
    
    Ok(())
}
