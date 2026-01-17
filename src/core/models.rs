use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceConnectionType {
    Internal,
    Usb,
    UsbHub,
    Unknown,
}

impl DeviceConnectionType {
    pub fn as_str(&self) -> &str {
        match self {
            DeviceConnectionType::Internal => "Internal",
            DeviceConnectionType::Usb => "USB",
            DeviceConnectionType::UsbHub => "USB Hub",
            DeviceConnectionType::Unknown => "Unknown",
        }
    }
}

/// Represents a block device suitable for ISO writing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlockDevice {
    pub path: PathBuf,
    pub model: String,
    pub vendor: String,
    pub capacity_bytes: u64,
    pub is_removable: bool,
    pub connection_type: DeviceConnectionType,
}

#[allow(dead_code)]
impl BlockDevice {
    /// Human-readable capacity (e.g., "16.0 GB")
    #[allow(clippy::cast_precision_loss)] // Acceptable for human-readable display
    pub fn capacity_human(&self) -> String {
        let gb = self.capacity_bytes as f64 / 1_000_000_000.0;
        format!("{gb:.1} GB")
    }
}

/// Progress information for write or verification operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Progress {
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub bytes_per_second: u64,
}

#[allow(dead_code)]
impl Progress {
    /// Percentage complete (0-100)
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn percentage(&self) -> u8 {
        if self.total_bytes == 0 {
            return 0;
        }
        ((self.bytes_processed as f64 / self.total_bytes as f64) * 100.0) as u8
    }

    /// Estimated seconds remaining
    pub const fn eta_seconds(&self) -> Option<u64> {
        if self.bytes_per_second == 0 {
            return None;
        }
        let remaining = self.total_bytes.saturating_sub(self.bytes_processed);
        Some(remaining / self.bytes_per_second)
    }

    /// Human-readable throughput (e.g., "12.5 MB/s")
    #[allow(clippy::cast_precision_loss)] // Acceptable for human-readable display
    pub fn throughput_human(&self) -> String {
        let mb_per_sec = self.bytes_per_second as f64 / 1_000_000.0;
        format!("{mb_per_sec:.1} MB/s")
    }
}

/// Current operation state
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum OperationState {
    Idle,
    Writing,
    Verifying,
    Complete,
    Failed(String),
}
