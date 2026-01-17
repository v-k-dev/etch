use crate::core::models::{BlockDevice, DeviceConnectionType};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

/// Detect device connection type (Internal, USB, USB Hub)
fn detect_connection_type(device_path: &std::path::Path) -> DeviceConnectionType {
    let device_name = device_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    
    // Check if device is connected via USB by walking up sysfs path
    let sys_path = PathBuf::from("/sys/block").join(device_name);
    
    // Read the device path to find USB indicators
    if let Ok(device_link) = fs::read_link(sys_path) {
        let path_str = device_link.to_string_lossy();
        
        // Check for USB indicators in the device path
        if path_str.contains("/usb") {
            // Count USB interfaces to detect hub
            // If path contains multiple /usb segments, it's likely through a hub
            let usb_count = path_str.matches("/usb").count();
            if usb_count > 1 {
                return DeviceConnectionType::UsbHub;
            }
            return DeviceConnectionType::Usb;
        }
        
        // Check for internal connection types
        if path_str.contains("/ata") || path_str.contains("/nvme") || path_str.contains("/virtio") {
            return DeviceConnectionType::Internal;
        }
    }
    
    // Fallback: check removable flag
    let removable_path = PathBuf::from("/sys/block").join(device_name).join("removable");
    if let Ok(removable) = fs::read_to_string(&removable_path) {
        if removable.trim() == "1" {
            return DeviceConnectionType::Usb;
        }
    }
    
    DeviceConnectionType::Unknown
}

/// Get the device that hosts the root filesystem
fn get_root_device() -> Option<String> {
    // Read /proc/mounts to find root filesystem
    let mounts = fs::read_to_string("/proc/mounts").ok()?;
    
    // Find the device mounted as /
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == "/" {
            let dev_path = PathBuf::from(parts[0]);
            let device_name = dev_path.file_name()?.to_str()?;
            
            // Extract base device name (e.g., /dev/sda1 -> sda, /dev/nvme0n1p1 -> nvme0n1)
            let base_name = if device_name.contains("nvme") {
                // NVMe devices: nvme0n1p1 -> nvme0n1
                device_name.trim_end_matches(|c: char| c.is_numeric() || c == 'p')
            } else {
                // Traditional devices: sda1 -> sda
                device_name.trim_end_matches(char::is_numeric)
            };
            
            return Some(base_name.to_string());
        }
    }
    
    None
}

/// Enumerate all writable block devices on the system (USB, NVMe, SATA, etc.)
/// Excludes loop devices, RAM disks, and system boot disk
#[allow(dead_code)]
pub fn list_removable_devices() -> Result<Vec<BlockDevice>> {
    let sys_block = PathBuf::from("/sys/block");

    if !sys_block.exists() {
        return Ok(Vec::new());
    }
    
    // Get root device to exclude it
    let root_device = get_root_device();

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
            let device_name_str = device_name.to_string_lossy();
            let device_path = entry.path();

            // CRITICAL: Skip root device to prevent system destruction
            if let Some(ref root_dev) = root_device {
                if device_name_str.as_ref() == root_dev {
                    eprintln!("INFO: Skipping root device: {}", device_name_str);
                    return None;
                }
            }

            // Skip loop devices, RAM disks, and other virtual devices
            if device_name_str.starts_with("loop")
                || device_name_str.starts_with("ram")
                || device_name_str.starts_with("dm-")
                || device_name_str.starts_with("sr")
            {
                return None;
            }

            // Check if device is removable (USB flag)
            let removable_path = device_path.join("removable");
            let is_removable = if removable_path.exists() {
                fs::read_to_string(&removable_path)
                    .unwrap_or_default()
                    .trim()
                    .parse::<u8>()
                    .unwrap_or(0)
                    == 1
            } else {
                false
            };
            
            // SAFETY: Only accept devices that are either:
            // 1. Marked as removable (USB devices)
            // 2. Connected via USB (detected by connection type)
            let dev_path_temp = PathBuf::from("/dev").join(&device_name);
            let connection_type = detect_connection_type(&dev_path_temp);
            
            // Skip internal drives (ATA, NVMe, Virtio) - only show USB devices
            if !is_removable && connection_type == DeviceConnectionType::Internal {
                eprintln!("INFO: Skipping internal device: {} ({})", device_name_str, connection_type.as_str());
                return None;
            }
            
            // Skip unknown connection types for safety
            if !is_removable && connection_type == DeviceConnectionType::Unknown {
                eprintln!("INFO: Skipping unknown device: {}", device_name_str);
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
                is_removable,
                connection_type,
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
