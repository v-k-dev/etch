use anyhow::{Context, Result};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

const CHUNK_SIZE: usize = 1024 * 1024;
const VERSION: &str = "1.0.0";

fn main() {
    if let Err(err) = run() {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let iso_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Missing ISO path argument"))?;
    let device_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Missing device path argument"))?;

    if args.next().is_some() {
        anyhow::bail!("Too many arguments. Usage: etch-helper <iso_path> <device_path>");
    }

    if !iso_path.exists() {
        anyhow::bail!("Source ISO does not exist: {}", iso_path.display());
    }

    let iso_metadata = iso_path
        .metadata()
        .context("Failed to read ISO metadata")?;
    if !iso_metadata.is_file() {
        anyhow::bail!("Source is not a regular file: {}", iso_path.display());
    }

    let device_metadata = device_path
        .metadata()
        .context("Failed to read device metadata")?;
    if !device_metadata.file_type().is_block_device() {
        anyhow::bail!("Target is not a block device: {}", device_path.display());
    }

    if !matches!(device_path.parent(), Some(parent) if parent == Path::new("/dev")) {
        anyhow::bail!("Refusing to operate on non-/dev paths");
    }

    let mut source = File::open(&iso_path)
        .context("Failed to open ISO for reading")?;
    let mut target = OpenOptions::new()
        .write(true)
        .open(&device_path)
        .context("Failed to open device for writing")?;

    let total_size = iso_metadata.len();
    println!("READY {total_size}");
    io::stdout().flush().ok();

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut total_written: u64 = 0;
    let start_time = Instant::now();
    let mut last_progress_time = start_time;

    loop {
        let bytes_read = source
            .read(&mut buffer)
            .context("Failed to read from ISO")?;

        if bytes_read == 0 {
            break;
        }

        target
            .write_all(&buffer[..bytes_read])
            .context("Failed to write to device")?;

        total_written += bytes_read as u64;

        let now = Instant::now();
        if now.duration_since(last_progress_time).as_millis() >= 100 || total_written == total_size
        {
            let elapsed = now.duration_since(start_time).as_secs_f64();
            let bps = if elapsed > 0.0 {
                (total_written as f64 / elapsed) as u64
            } else {
                0
            };
            println!("PROGRESS {total_written} {bps}");
            io::stdout().flush().ok();
            last_progress_time = now;
        }
    }

    target
        .sync_all()
        .context("Failed to sync device")?;

    println!("DONE");
    io::stdout().flush().ok();

    // Verification phase - reopen both files for verification
    println!("VERIFY_START {total_size}");
    io::stdout().flush().ok();

    let mut source = File::open(&iso_path)
        .context("Failed to reopen ISO for verification")?;
    let mut target = File::open(&device_path)
        .context("Failed to reopen device for verification")?;

    let mut source_buffer = vec![0u8; CHUNK_SIZE];
    let mut target_buffer = vec![0u8; CHUNK_SIZE];
    let mut total_verified: u64 = 0;
    let verify_start_time = Instant::now();
    let mut last_progress_time = verify_start_time;

    loop {
        let source_bytes_read = source
            .read(&mut source_buffer)
            .context("Failed to read from ISO during verification")?;

        if source_bytes_read == 0 {
            break;
        }

        let target_bytes_read = target
            .read(&mut target_buffer[..source_bytes_read])
            .context("Failed to read from device during verification")?;

        if source_bytes_read != target_bytes_read {
            anyhow::bail!(
                "Verification failed: size mismatch at offset {total_verified}. Expected {source_bytes_read} bytes, got {target_bytes_read} bytes"
            );
        }

        if source_buffer[..source_bytes_read] != target_buffer[..target_bytes_read] {
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

        let now = Instant::now();
        if now.duration_since(last_progress_time).as_millis() >= 100 || total_verified == total_size
        {
            let elapsed = now.duration_since(verify_start_time).as_secs_f64();
            let bps = if elapsed > 0.0 {
                (total_verified as f64 / elapsed) as u64
            } else {
                0
            };
            println!("VERIFY_PROGRESS {total_verified} {bps}");
            io::stdout().flush().ok();
            last_progress_time = now;
        }
    }

    println!("VERIFY_DONE");
    io::stdout().flush().ok();

    // Output final metrics
    let total_elapsed = Instant::now().duration_since(start_time).as_secs_f64();
    let avg_write_speed = (total_size as f64 / total_elapsed) / 1_000_000.0;
    println!("METRICS total_time={:.2}s avg_speed={:.2}MB/s total_bytes={} version={}", 
             total_elapsed, avg_write_speed, total_size, VERSION);
    io::stdout().flush().ok();

    Ok(())
}
