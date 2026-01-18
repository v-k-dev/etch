use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Instant;
use std::process::{Command, Stdio};

use super::Distro;
use super::verification::verify_sha256;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DownloadProgress {
    Started { name: String, size: u64 },
    Progress { bytes: u64, total: u64, bps: u64 },
    Verifying,
    Complete { path: PathBuf },
    Error { error: String },
}

pub struct ISOFetcher;

impl ISOFetcher {
    /// Download an ISO with progress reporting using curl
    pub fn download(
        distro: &Distro,
        destination_dir: &Path,
        progress_tx: Option<Sender<DownloadProgress>>,
    ) -> Result<PathBuf> {
        // Create destination directory
        std::fs::create_dir_all(destination_dir)?;

        // Determine output filename
        let filename = format!("{}-{}.iso", 
            distro.id.replace(' ', "-"), 
            distro.version.replace(' ', "-")
        );
        let output_path = destination_dir.join(&filename);

        // Send start notification
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(DownloadProgress::Started {
                name: distro.name.clone(),
                size: distro.size_bytes,
            });
        }

        println!("Starting download with curl: {}", distro.download_url);
        println!("Output: {}", output_path.display());

        // Use curl for downloading with progress
        let mut child = Command::new("curl")
            .arg("-L") // Follow redirects
            .arg("-#") // Show progress bar
            .arg("-o")
            .arg(&output_path)
            .arg(&distro.download_url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start curl - is it installed?")?;

        // Monitor stderr for progress (curl outputs progress to stderr)
        let stderr = child.stderr.take();
        if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let start_time = Instant::now();
            
            for line_result in reader.lines() {
                if let Ok(_line) = line_result {
                    // curl progress format: ######## (percentage)
                    // We'll estimate based on time and expected size
                    let elapsed = start_time.elapsed().as_secs_f64();
                    
                    // Simple progress estimation
                    if let Some(ref tx) = progress_tx {
                        // Check if file exists and get its current size
                        if let Ok(metadata) = std::fs::metadata(&output_path) {
                            let downloaded = metadata.len();
                            let bps = if elapsed > 0.0 {
                                (downloaded as f64 / elapsed) as u64
                            } else {
                                0
                            };
                            
                            let _ = tx.send(DownloadProgress::Progress {
                                bytes: downloaded,
                                total: distro.size_bytes,
                                bps,
                            });
                        }
                    }
                }
            }
        }

        // Wait for curl to finish
        let status = child.wait()?;

        if !status.success() {
            let error_msg = format!("curl failed with exit code: {}", status.code().unwrap_or(-1));
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress::Error {
                    error: error_msg.clone(),
                });
            }
            anyhow::bail!(error_msg);
        }

        // Verify file exists and has reasonable size
        if !output_path.exists() {
            anyhow::bail!("Download completed but file not found");
        }

        let file_size = std::fs::metadata(&output_path)?.len();
        if file_size < 1_000_000 { // Less than 1MB is suspicious
            anyhow::bail!("Downloaded file is too small ({}), download may have failed", file_size);
        }

        println!("Download complete: {} bytes", file_size);

        // Skip SHA256 verification if hash is placeholder or empty
        if distro.sha256.is_empty() || distro.sha256.starts_with("PLACEHOLDER") {
            println!("Skipping verification - no valid hash provided");
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress::Complete {
                    path: output_path.clone(),
                });
            }
            return Ok(output_path);
        }

        // Verify SHA256
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(DownloadProgress::Verifying);
        }

        match verify_sha256(&output_path, &distro.sha256) {
            Ok(true) => {
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(DownloadProgress::Complete {
                        path: output_path.clone(),
                    });
                }
                Ok(output_path)
            }
            Ok(false) | Err(_) => {
                // Clean up corrupted file
                let _ = std::fs::remove_file(&output_path);
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(DownloadProgress::Error {
                        error: "SHA256 verification failed".to_string(),
                    });
                }
                anyhow::bail!("SHA256 checksum verification failed")
            }
        }
    }

    /// Get default download directory
    pub fn default_download_dir() -> PathBuf {
        dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("Etch")
    }
}
