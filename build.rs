use std::process::Command;

fn main() {
    // Get git commit hash
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output();
    
    let git_hash = if let Ok(output) = output {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };
    
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    
    // Get git branch
    let output = Command::new("git")
        .args(&["branch", "--show-current"])
        .output();
    
    let git_branch = if let Ok(output) = output {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };
    
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
    
    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
