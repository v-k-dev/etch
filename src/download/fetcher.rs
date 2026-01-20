use crate::db::DbConnection;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;
use std::time::Instant;

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
    /// Validate a download URL by sending a HEAD request
    pub async fn validate_url(url: &str) -> Result<bool> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        
        match client.head(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    println!("✓ URL validation passed: {}", url);
                    Ok(true)
                } else {
                    println!("✗ URL validation failed with status {}: {}", response.status(), url);
                    Ok(false)
                }
            }
            Err(e) => {
                println!("✗ URL validation error: {} - {}", url, e);
                Err(anyhow::anyhow!("URL validation failed: {}", e))
            }
        }
    }
    
    /// Download an ISO with progress reporting using reqwest with mirror fallback
    pub fn download(
        distro: &Distro,
        destination_dir: &Path,
        progress_tx: Option<Sender<DownloadProgress>>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<PathBuf> {
        // Get all mirrors for this distro
        let mirrors = DbConnection::get_mirrors(&distro.id).unwrap_or_default();
        
        if mirrors.is_empty() {
            // Fallback to download_url if no mirrors in DB
            return Self::download_from_url(
                &distro.download_url,
                distro,
                destination_dir,
                progress_tx,
                cancel_flag,
            );
        }

        // Try mirrors in priority order
        let mut last_error = None;
        for mirror in mirrors {
            println!("→ Trying mirror: {} ({})", mirror.region, mirror.url);
            
            match Self::download_from_url(
                &mirror.url,
                distro,
                destination_dir,
                progress_tx.clone(),
                cancel_flag.clone(),
            ) {
                Ok(path) => {
                    // Update mirror status to "ok"
                    let _ = DbConnection::update_mirror_status(mirror.id, "ok");
                    return Ok(path);
                }
                Err(e) => {
                    println!("✗ Mirror failed: {}", e);
                    // Mark mirror as down
                    let _ = DbConnection::update_mirror_status(mirror.id, "down");
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // All mirrors failed
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All mirrors failed")))
    }

    /// Download from a specific URL (internal helper)
    fn download_from_url(
        url: &str,
        distro: &Distro,
        destination_dir: &Path,
        progress_tx: Option<Sender<DownloadProgress>>,
        cancel_flag: Option<Arc<AtomicBool>>,
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

        println!("→ Starting download: {}", distro.name);
        println!("  URL: {}", url);
        println!("  Destination: {}", output_path.display());

        // Use reqwest for reliable downloading
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;

        let mut response = client
            .get(url)
            .send()
            .context("Failed to start download")?;

        if !response.status().is_success() {
            let error_msg = format!("HTTP error: {}", response.status());
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress::Error {
                    error: error_msg.clone(),
                });
            }
            anyhow::bail!(error_msg);
        }

        // Get total size from Content-Length header
        let total_size = response.content_length().unwrap_or(distro.size_bytes);

        // Create output file
        let mut file = std::fs::File::create(&output_path)
            .context("Failed to create output file")?;

        // Download with progress reporting
        let mut downloaded: u64 = 0;
        let mut last_report = Instant::now();
        let mut speed_samples: Vec<f64> = Vec::new();
        let start_time = Instant::now();

        let mut buffer = vec![0; 128 * 1024]; // 128KB buffer

        loop {
            // Check cancel flag
            if let Some(ref flag) = cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    println!("✗ Download cancelled by user");
                    let _ = std::fs::remove_file(&output_path);
                    anyhow::bail!("Download cancelled");
                }
            }

            // Read chunk
            use std::io::Read;
            let bytes_read = response.read(&mut buffer)
                .context("Failed to read from response")?;

            if bytes_read == 0 {
                break; // EOF
            }

            // Write to file
            file.write_all(&buffer[..bytes_read])
                .context("Failed to write to file")?;

            downloaded += bytes_read as u64;

            // Report progress every 200ms
            if last_report.elapsed().as_millis() >= 200 {
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                let speed_bps = if elapsed_secs > 0.0 {
                    downloaded as f64 / elapsed_secs
                } else {
                    0.0
                };

                // Keep last 10 speed samples for smoothing
                speed_samples.push(speed_bps);
                if speed_samples.len() > 10 {
                    speed_samples.remove(0);
                }

                let avg_speed = speed_samples.iter().sum::<f64>() / speed_samples.len() as f64;

                if let Some(ref tx) = progress_tx {
                    let _ = tx.send(DownloadProgress::Progress {
                        bytes: downloaded,
                        total: total_size,
                        bps: avg_speed as u64,
                    });
                }

                last_report = Instant::now();
            }
        }

        // Ensure all data is written
        file.flush().context("Failed to flush file")?;
        drop(file);

        println!("✓ Download complete: {} bytes", downloaded);

        // Verify file exists and has reasonable size
        if !output_path.exists() {
            anyhow::bail!("Download completed but file not found");
        }

        let file_size = std::fs::metadata(&output_path)?.len();
        if file_size < 1_000_000 { // Less than 1MB is suspicious
            anyhow::bail!("Downloaded file is too small ({} bytes), download may have failed", file_size);
        }

        // Skip SHA256 verification if hash is placeholder or empty
        if distro.sha256.is_empty() || distro.sha256.starts_with("PLACEHOLDER") {
            println!("⚠ Skipping verification - no valid hash provided");
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress::Complete {
                    path: output_path.clone(),
                });
            }
            return Ok(output_path);
        }

        // Verify SHA256
        println!("⟳ Verifying SHA256 checksum...");
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(DownloadProgress::Verifying);
        }

        match verify_sha256(&output_path, &distro.sha256) {
            Ok(true) => {
                println!("✓ Checksum verified successfully");
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
