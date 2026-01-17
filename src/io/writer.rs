use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::File as AsyncFile;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks
const MIN_CHUNK_SIZE: usize = 64 * 1024; // 64 KB minimum

/// Calculate optimal buffer size based on file size
/// Uses larger buffers for larger files to reduce syscall overhead
fn calculate_buffer_size(file_size: u64) -> usize {
    // For very small files, use minimum buffer size
    if file_size <= (MIN_CHUNK_SIZE as u64) {
        return MIN_CHUNK_SIZE;
    }
    
    // For medium files, use standard chunk size up to CHUNK_SIZE
    if file_size <= (CHUNK_SIZE as u64) {
        return CHUNK_SIZE;
    }
    
    // For large files (greater than CHUNK_SIZE), calculate adaptive buffer size
    // up to a maximum of 4 * CHUNK_SIZE (4 MiB) to balance memory usage and performance.
    let adaptive_size = (file_size / 100).min(4 * CHUNK_SIZE as u64) as usize;
    
    // Ensure the adaptive size is at least CHUNK_SIZE to avoid shrinking below the standard size.
    let buffer_size = adaptive_size.max(CHUNK_SIZE);
    
    // Finally, ensure it's at least the minimum chunk size (though CHUNK_SIZE already satisfies this).
    buffer_size.max(MIN_CHUNK_SIZE)
}

/// Write ISO image to block device asynchronously
/// Must report real progress via callback
#[allow(dead_code)]
pub async fn write_iso_async<F>(
    source_iso: &Path,
    target_device: &Path,
    mut progress_callback: F, // (bytes_written, total_bytes, bytes_per_second)
) -> Result<()>
where
    F: FnMut(u64, u64, u64),
{
    // Open source ISO for reading
    let mut source = AsyncFile::open(source_iso).await.context(format!(
        "Failed to open source ISO: {}",
        source_iso.display()
    ))?;

    let total_size = source
        .metadata()
        .await
        .context("Failed to get source file size")?
        .len();

    // Open target device for writing (requires root/sudo)
    let mut target = tokio::fs::OpenOptions::new()
        .write(true)
        .open(target_device)
        .await
        .context(format!(
            "Failed to open target device for writing: {}. Are you running with sudo?",
            target_device.display()
        ))?;

    // Use adaptive buffer size based on file size
    let buffer_size = calculate_buffer_size(total_size);
    let mut buffer = vec![0u8; buffer_size];
    let mut total_written: u64 = 0;
    let start_time = Instant::now();
    let mut last_progress_time = start_time;

    loop {
        // Read chunk from source
        let bytes_read = source
            .read(&mut buffer)
            .await
            .context("Failed to read from source ISO")?;

        if bytes_read == 0 {
            break; // EOF
        }

        // Write chunk to target
        target
            .write_all(&buffer[..bytes_read])
            .await
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
    target.flush().await.context("Failed to flush data to disk")?;
    target.sync_all().await.context("Failed to sync data to disk")?;

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

/// Write ISO image to block device
/// Must report real progress via callback
#[allow(dead_code)]
pub fn write_iso<F>(
    source_iso: &Path,
    target_device: &Path,
    mut progress_callback: F, // (bytes_written, total_bytes, bytes_per_second)
) -> Result<()>
where
    F: FnMut(u64, u64, u64),
{
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

    // Use adaptive buffer size based on file size
    let buffer_size = calculate_buffer_size(total_size);
    let mut buffer = vec![0u8; buffer_size];
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_buffer_size_small_file() {
        // For very small files, should use minimum buffer size
        assert_eq!(calculate_buffer_size(1024), MIN_CHUNK_SIZE);
        assert_eq!(calculate_buffer_size(MIN_CHUNK_SIZE as u64), MIN_CHUNK_SIZE);
    }

    #[test]
    fn test_calculate_buffer_size_medium_file() {
        // For medium files, should use standard chunk size
        assert_eq!(calculate_buffer_size(CHUNK_SIZE as u64), CHUNK_SIZE);
        assert_eq!(calculate_buffer_size((CHUNK_SIZE * 50) as u64), CHUNK_SIZE);
    }

    #[test]
    fn test_calculate_buffer_size_large_file() {
        // For large files, should calculate optimal size
        let large_file_size = CHUNK_SIZE as u64 * 200; // 200 MB
        let expected_size = (large_file_size / 100) as usize;
        assert_eq!(calculate_buffer_size(large_file_size), expected_size);

        // But should be capped at 4MB
        let very_large_file_size = CHUNK_SIZE as u64 * 100000; // 100GB
        let max_size = 4 * CHUNK_SIZE;
        assert_eq!(calculate_buffer_size(very_large_file_size), max_size);
    }

    #[tokio::test]
    async fn test_write_iso_async_with_temp_files() -> Result<()> {
        // Create temporary directory for test files
        let temp_dir = TempDir::new()?;
        let source_path = temp_dir.path().join("test_source.iso");
        let target_path = temp_dir.path().join("test_target.img");

        // Create a small test file
        let mut source_file = File::create(&source_path)?;
        let test_data = vec![0u8; 1024]; // 1KB of zeros
        source_file.write_all(&test_data)?;
        source_file.flush()?;

        // Create target file
        File::create(&target_path)?;

        // Track if progress callback was called
        let mut progress_called = false;
        
        // Test the async write function
        let result = write_iso_async(
            &source_path,
            &target_path,
            |_written, _total, _bps| {
                progress_called = true;
            }
        ).await;

        // Should succeed
        assert!(result.is_ok());
        // Progress callback should have been called
        assert!(progress_called);

        // Verify the files are identical
        let source_content = fs::read(&source_path)?;
        let target_content = fs::read(&target_path)?;
        assert_eq!(source_content, target_content);

        Ok(())
    }

    #[test]
    fn test_write_iso_with_temp_files() -> Result<()> {
        // Create temporary directory for test files
        let temp_dir = TempDir::new()?;
        let source_path = temp_dir.path().join("test_source.iso");
        let target_path = temp_dir.path().join("test_target.img");

        // Create a small test file
        let mut source_file = File::create(&source_path)?;
        let test_data = vec![0u8; 1024]; // 1KB of zeros
        source_file.write_all(&test_data)?;
        source_file.flush()?;

        // Create target file
        File::create(&target_path)?;

        // Track if progress callback was called
        let mut progress_called = false;
        
        // Test the sync write function
        let result = write_iso(
            &source_path,
            &target_path,
            |_written, _total, _bps| {
                progress_called = true;
            }
        );

        // Should succeed
        assert!(result.is_ok());
        // Progress callback should have been called
        assert!(progress_called);

        // Verify the files are identical
        let source_content = fs::read(&source_path)?;
        let target_content = fs::read(&target_path)?;
        assert_eq!(source_content, target_content);

        Ok(())
    }
}
