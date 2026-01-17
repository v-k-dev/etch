use crate::core::models::BlockDevice;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

/// Enumerate all removable block devices on the system
#[allow(dead_code)]
pub fn list_removable_devices() -> Result<Vec<BlockDevice>> {
    let sys_block = PathBuf::from("/sys/block");

    if !sys_block.exists() {
        return Ok(Vec::new());
    }

    // Collect all entries first to enable parallel processing
    let entries: Vec<_> = fs::read_dir(&sys_block)
        .context("Failed to read /sys/block")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to read directory entries")?;

    // Process entries in parallel
    let devices: Vec<BlockDevice> = entries
        .into_par_iter()
        .filter_map(|entry| {
            let device_name = entry.file_name();
            let device_path = entry.path();

            // Check if device is removable
            let removable_path = device_path.join("removable");
            if !removable_path.exists() {
                return None;
            }

            let removable = fs::read_to_string(&removable_path)
                .unwrap_or_default()
                .trim()
                .parse::<u8>()
                .unwrap_or(0);

            if removable != 1 {
                return None;
            }

            // Read device information
            let model = read_sys_file(&device_path.join("device/model"))
                .unwrap_or_else(|| "Unknown".to_string());
            let vendor = read_sys_file(&device_path.join("device/vendor"))
                .unwrap_or_else(|| "Unknown".to_string());

            // Read capacity in 512-byte sectors
            let size_str = read_sys_file(&device_path.join("size")).unwrap_or_else(|| "0".to_string());
            let sectors: u64 = size_str.parse().unwrap_or(0);
            let capacity_bytes = sectors * 512;

            // Skip devices with zero capacity
            if capacity_bytes == 0 {
                return None;
            }

            let dev_path = PathBuf::from("/dev").join(&device_name);

            Some(BlockDevice {
                path: dev_path,
                model: model.trim().to_string(),
                vendor: vendor.trim().to_string(),
                capacity_bytes,
                is_removable: true,
            })
        })
        .collect();

    Ok(devices)
}

/// Read and trim a sysfs file, return None if it doesn't exist or can't be read
fn read_sys_file(path: &PathBuf) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Verify that a device path is valid and safe to write to
#[allow(dead_code)]
pub fn validate_device(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::FileTypeExt;

    // Check device exists
    let metadata =
        fs::metadata(path).context(format!("Device {} does not exist", path.display()))?;

    // Check it's a block device
    if !metadata.file_type().is_block_device() {
        anyhow::bail!("{} is not a block device", path.display());
    }

    // Check if any partition is mounted
    let mounts = fs::read_to_string("/proc/mounts").context("Failed to read /proc/mounts")?;

    let device_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?;

    for line in mounts.lines() {
        if line.starts_with(&format!("/dev/{device_name}")) {
            anyhow::bail!(
                "Device or one of its partitions is currently mounted. Unmount it first."
            );
        }
    }

    // Note: We do NOT check write permissions here because the write operation
    // will be performed by etch-helper via pkexec, which will elevate privileges.
    // Checking permissions here would always fail for non-root users.

    Ok(())
}
