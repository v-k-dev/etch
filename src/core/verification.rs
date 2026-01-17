use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks

/// Verify written data matches source ISO
#[allow(dead_code)]
pub fn verify_write(
    source_iso: &Path,
    target_device: &Path,
    progress_callback: impl Fn(u64, u64, u64), // (bytes_verified, total_bytes, bytes_per_second)
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

    // Open target device for reading
    let mut target = File::open(target_device).context(format!(
        "Failed to open target device for reading: {}",
        target_device.display()
    ))?;

    // Allocate buffers once outside loop for memory efficiency
    let mut source_buffer = vec![0u8; CHUNK_SIZE];
    let mut target_buffer = vec![0u8; CHUNK_SIZE];
    let mut total_verified: u64 = 0;
    let start_time = Instant::now();
    let mut last_progress_time = start_time;

    loop {
        // Read chunk from source
        let source_bytes_read = source
            .read(&mut source_buffer)
            .context("Failed to read from source ISO")?;

        if source_bytes_read == 0 {
            break; // EOF
        }

        // Read same amount from target
        let target_bytes_read = target
            .read(&mut target_buffer[..source_bytes_read])
            .context("Failed to read from target device")?;

        // Verify we read the same amount
        if source_bytes_read != target_bytes_read {
            anyhow::bail!(
                "Verification failed: size mismatch at offset {total_verified}. Expected {source_bytes_read} bytes, got {target_bytes_read} bytes."
            );
        }

        // Compare buffers byte-by-byte
        if source_buffer[..source_bytes_read] != target_buffer[..target_bytes_read] {
            // Find the first differing byte for detailed error message
            for (i, (s, t)) in source_buffer[..source_bytes_read]
                .iter()
                .zip(target_buffer[..target_bytes_read].iter())
                .enumerate()
            {
                if s != t {
                    anyhow::bail!(
                        "Verification failed: data mismatch at byte offset {}. Source: 0x{:02x}, Target: 0x{:02x}",
                        total_verified + i as u64,
                        s,
                        t
                    );
                }
            }
        }

        total_verified += source_bytes_read as u64;

        // Report progress (throttle to avoid overwhelming UI)
        let now = Instant::now();
        if now.duration_since(last_progress_time).as_millis() >= 100 || total_verified == total_size
        {
            let elapsed = now.duration_since(start_time).as_secs_f64();
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let bytes_per_second = if elapsed > 0.0 {
                (total_verified as f64 / elapsed) as u64
            } else {
                0
            };
            progress_callback(total_verified, total_size, bytes_per_second);
            last_progress_time = now;
        }
    }

    // Ensure final progress update is sent
    if total_verified > 0 {
        let elapsed = Instant::now().duration_since(start_time).as_secs_f64();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let bytes_per_second = if elapsed > 0.0 {
            (total_verified as f64 / elapsed) as u64
        } else {
            0
        };
        progress_callback(total_verified, total_size, bytes_per_second);
    }

    Ok(())
}
