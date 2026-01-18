use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Instant;

use super::Distro;
use super::verification::verify_sha256;

#[derive(Debug, Clone)]
pub enum DownloadProgress {
    Started { name: String, size: u64 },
    Progress { bytes: u64, total: u64, bps: u64 },
    Verifying,
    Complete { path: PathBuf },
    Error { error: String },
}

pub struct ISOFetcher;

impl ISOFetcher {
    /// Download an ISO with progress reporting
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

        // Download with progress tracking
        let mut response = reqwest::blocking::get(&distro.download_url)
            .context("Failed to start download")?;

        if !response.status().is_success() {
            if let Some(ref tx) = progress_tx {
                let _ = tx.send(DownloadProgress::Error {
                    error: format!("HTTP error: {}", response.status()),
                });
            }
            anyhow::bail!("Download failed with status: {}", response.status());
        }

        let file = File::create(&output_path)?;
        let mut writer = BufWriter::new(file);
        let mut downloaded: u64 = 0;
        let mut buffer = [0u8; 32768]; // 32KB chunks
        let start_time = Instant::now();

        loop {
            match response.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    writer.write_all(&buffer[..n])?;
                    downloaded += n as u64;

                    // Calculate speed
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let bps = if elapsed > 0.0 {
                        (downloaded as f64 / elapsed) as u64
                    } else {
                        0
                    };

                    // Send progress update
                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(DownloadProgress::Progress {
                            bytes: downloaded,
                            total: distro.size_bytes,
                            bps,
                        });
                    }
                }
                Err(e) => {
                    if let Some(ref tx) = progress_tx {
                        let _ = tx.send(DownloadProgress::Error {
                            error: format!("Download error: {}", e),
                        });
                    }
                    return Err(e.into());
                }
            }
        }

        writer.flush()?;
        drop(writer);

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
