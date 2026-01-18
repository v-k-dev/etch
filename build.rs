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
    
    // Generate semantic version
    let (version, version_code) = generate_version();
    println!("cargo:rustc-env=AUTO_VERSION={}", version);
    println!("cargo:rustc-env=AUTO_VERSION_CODE={}", version_code);
    
    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}

fn generate_version() -> (String, String) {
    // Try to get the latest git tag
    let tag_output = Command::new("git")
        .args(&["describe", "--tags", "--abbrev=0"])
        .output();
    
    let (base_version, latest_tag) = if let Ok(ref output) = tag_output {
        let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let version = if tag.starts_with('v') {
            tag[1..].to_string()
        } else {
            tag.clone()
        };
        (version, tag)
    } else {
        ("0.1.0".to_string(), String::new())
    };
    
    // Check if we're exactly on a tag
    let on_exact_tag = Command::new("git")
        .args(&["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .is_ok();
    
    // If we're on a tag, just use that version
    if on_exact_tag {
        return (base_version, "0".to_string());
    }
    
    // Count commits since last tag (or all commits if no tag)
    let commit_count_output = Command::new("git")
        .args(&["rev-list", "--count", "HEAD"])
        .output();
    
    let commit_count = if let Ok(output) = commit_count_output {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        "0".to_string()
    };
    
    // Get commits since last tag to determine version bump
    let commits_since_tag = if !latest_tag.is_empty() {
        Command::new("git")
            .args(&["log", &format!("{}..HEAD", latest_tag), "--oneline", "--format=%s"])
            .output()
    } else {
        Command::new("git")
            .args(&["log", "--oneline", "--format=%s"])
            .output()
    };
    
    let (major_bump, minor_bump, patch_bump) = if let Ok(output) = commits_since_tag {
        let commits = String::from_utf8_lossy(&output.stdout);
        analyze_commits(&commits)
    } else {
        (0, 0, 0)
    };
    
    // Parse base version
    let parts: Vec<&str> = base_version.split('.').collect();
    let major: u32 = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let patch: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    
    // Calculate new version based on bumps
    let new_version = if major_bump > 0 {
        format!("{}.0.0", major + major_bump)
    } else if minor_bump > 0 {
        format!("{}.{}.0", major, minor + minor_bump)
    } else if patch_bump > 0 {
        format!("{}.{}.{}", major, minor, patch + patch_bump)
    } else {
        // No commits since tag - use tag version
        base_version.clone()
    };
    
    (new_version, commit_count)
}

fn analyze_commits(commits: &str) -> (u32, u32, u32) {
    let mut major = 0;
    let mut minor = 0;
    let mut patch = 0;
    
    for line in commits.lines() {
        let lower = line.to_lowercase();
        
        // Check for breaking changes (MAJOR version bump)
        if lower.contains("breaking") || 
           lower.contains("!:") ||
           lower.starts_with("major:") {
            major = 1;
        }
        // Check for new features (MINOR version bump)
        else if lower.starts_with("feat:") || 
                lower.starts_with("feature:") ||
                lower.starts_with("add:") {
            if major == 0 {
                minor = 1;
            }
        }
        // Check for fixes/patches (PATCH version bump)
        else if lower.starts_with("fix:") ||
                lower.starts_with("bugfix:") ||
                lower.starts_with("patch:") {
            if major == 0 && minor == 0 {
                patch = 1;
            }
        }
    }
    
    // Default to no bump if at a tagged commit
    if major == 0 && minor == 0 && patch == 0 {
        // Check if we're exactly on a tag
        let on_tag = Command::new("git")
            .args(&["describe", "--exact-match", "--tags", "HEAD"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        if !on_tag {
            patch = 1;
        }
    }
    
    (major, minor, patch)
}
