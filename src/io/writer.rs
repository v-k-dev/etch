use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks

/// Write ISO image to block device
/// Must report real progress via callback
#[allow(dead_code)]
pub fn write_iso(
    source_iso: &Path,
    target_device: &Path,
    progress_callback: impl Fn(u64, u64, u64), // (bytes_written, total_bytes, bytes_per_second)
) -> Result<()> {
    // Open source ISO for reading
    let mut source = File::open(source_iso).context(format!(
        "Failed to open source ISO: {}",
        source_iso.display()
    ))?;

    let total_size = source
        .metadata()
        .context("Failed to get source file size")?
        .len();

    // Open target device for writing (requires root/sudo)
    let mut target = File::options()
        .write(true)
        .open(target_device)
        .context(format!(
            "Failed to open target device for writing: {}. Are you running with sudo?",
            target_device.display()
        ))?;

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut total_written: u64 = 0;
    let start_time = Instant::now();
    let mut last_progress_time = start_time;

    loop {
        // Read chunk from source
        let bytes_read = source
            .read(&mut buffer)
            .context("Failed to read from source ISO")?;

        if bytes_read == 0 {
            break; // EOF
        }

        // Write chunk to target
        target
            .write_all(&buffer[..bytes_read])
            .context("Failed to write to target device")?;

        total_written += bytes_read as u64;

        // Report progress (throttle to avoid overwhelming UI)
        let now = Instant::now();
        if now.duration_since(last_progress_time).as_millis() >= 100 || total_written == total_size
        {
            let elapsed = now.duration_since(start_time).as_secs_f64();
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let bytes_per_second = if elapsed > 0.0 {
                (total_written as f64 / elapsed) as u64
            } else {
                0
            };
            progress_callback(total_written, total_size, bytes_per_second);
            last_progress_time = now;
        }
    }

    // Sync to ensure all data is written to disk
    target.sync_all().context("Failed to sync data to disk")?;

    // Ensure final progress update is sent
    if total_written > 0 {
        let elapsed = Instant::now().duration_since(start_time).as_secs_f64();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let bytes_per_second = if elapsed > 0.0 {
            (total_written as f64 / elapsed) as u64
        } else {
            0
        };
        progress_callback(total_written, total_size, bytes_per_second);
    }

    Ok(())
}
