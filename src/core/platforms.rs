/// Platform detection for ISO images and embedded systems
/// Identifies target platform based on ISO filename, size, and magic bytes

use std::path::Path;

/// Strategy for verifying written data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]  // Part of public API for future use
pub enum VerificationStrategy {
    /// Read entire image back and compare byte-by-byte
    FullByteCheck,
    /// Compare checksums/hashes
    ChecksumOnly,
    /// Verify boot sector and critical sections
    BootSectorCheck,
    /// Verify firmware-specific structures
    FirmwareCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    // OS Images
    WindowsISO,
    LinuxISO,
    GenericISO,
    
    // Single-board computers
    RaspberryPi,
    OrangePi,
    
    // Microcontrollers & Boards
    ESP32,
    Arduino,
    
    // Other - intentionally kept for exhaustiveness and future expansion
    #[allow(dead_code)]
    Unknown,
}

impl Platform {
    /// Detect platform from ISO file path
    pub fn from_iso_path(path: &Path) -> Self {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Check filename patterns
        if Self::is_windows(&filename) {
            Platform::WindowsISO
        } else if Self::is_raspberrypi(&filename) {
            Platform::RaspberryPi
        } else if Self::is_orangepi(&filename) {
            Platform::OrangePi
        } else if Self::is_esp32(&filename) {
            Platform::ESP32
        } else if Self::is_arduino(&filename) {
            Platform::Arduino
        } else if Self::is_linux(&filename) {
            Platform::LinuxISO
        } else {
            Platform::GenericISO
        }
    }
    
    /// Get icon name for this platform
    pub fn icon_name(&self) -> &'static str {
        match self {
            Platform::WindowsISO => "windows-symbolic",
            Platform::LinuxISO => "linux-symbolic",
            Platform::RaspberryPi => "media-removable-symbolic", // fallback, should use custom
            Platform::OrangePi => "media-removable-symbolic",
            Platform::ESP32 => "hardware-symbolic",
            Platform::Arduino => "hardware-symbolic",
            Platform::GenericISO => "media-optical-symbolic",
            Platform::Unknown => "help-faq-symbolic",
        }
    }
    
    /// Get human-readable platform name
    pub fn display_name(&self) -> &'static str {
        match self {
            Platform::WindowsISO => "Windows",
            Platform::LinuxISO => "Linux",
            Platform::RaspberryPi => "Raspberry Pi",
            Platform::OrangePi => "Orange Pi",
            Platform::ESP32 => "ESP32",
            Platform::Arduino => "Arduino",
            Platform::GenericISO => "ISO Image",
            Platform::Unknown => "Unknown",
        }
    }
    
    /// Check if special handling is needed
    #[allow(dead_code)]  // Part of public API for platform detection
    pub fn requires_special_handling(&self) -> bool {
        matches!(
            self,
            Platform::RaspberryPi | Platform::OrangePi | Platform::ESP32 | Platform::Arduino
        )
    }
    
    /// Get recommended write speed optimization for this platform
    /// Returns (buffer_size_mb, optimal_chunk_size_kb)
    #[allow(dead_code)]  // Part of public API for platform-specific optimization
    pub fn write_optimization(&self) -> (usize, usize) {
        match self {
            // Standard ISO images - optimize for throughput
            Platform::WindowsISO | Platform::LinuxISO | Platform::GenericISO => (16, 4096),
            
            // Raspberry Pi - sensitive to large buffers, use smaller chunks
            Platform::RaspberryPi => (4, 512),
            
            // Orange Pi - similar to RPi but slightly better specs
            Platform::OrangePi => (8, 1024),
            
            // Microcontrollers - very small buffer, sequential write
            Platform::ESP32 | Platform::Arduino => (1, 64),
            
            // Unknown platform - conservative settings
            Platform::Unknown => (4, 512),
        }
    }
    
    /// Get verification strategy for this platform
    #[allow(dead_code)]  // Part of public API for verification control
    pub fn verification_strategy(&self) -> VerificationStrategy {
        match self {
            // Standard images - byte-by-byte verification
            Platform::WindowsISO | Platform::LinuxISO => VerificationStrategy::FullByteCheck,
            
            // Generic ISO - optional verification
            Platform::GenericISO => VerificationStrategy::ChecksumOnly,
            
            // SBC images - verify boot sector and first blocks
            Platform::RaspberryPi | Platform::OrangePi => VerificationStrategy::BootSectorCheck,
            
            // Firmware - verify magic bytes and checksum
            Platform::ESP32 | Platform::Arduino => VerificationStrategy::FirmwareCheck,
            
            Platform::Unknown => VerificationStrategy::ChecksumOnly,
        }
    }
    
    /// Get device speed class recommendation
    #[allow(dead_code)]  // Part of public API for device recommendations
    pub fn recommended_speed(&self) -> &'static str {
        match self {
            // Standard ISOs work on any USB
            Platform::WindowsISO | Platform::LinuxISO | Platform::GenericISO => "USB 2.0+",
            
            // SBC images benefit from USB 3.0+
            Platform::RaspberryPi | Platform::OrangePi => "USB 3.0+ (20+ MB/s)",
            
            // Firmware - USB 2.0 sufficient but 3.0 preferred
            Platform::ESP32 | Platform::Arduino => "USB 2.0+",
            
            Platform::Unknown => "USB 2.0+",
        }
    }
    
    // Detection helpers
    fn is_windows(filename: &str) -> bool {
        filename.contains("windows") || filename.contains("win10") || filename.contains("win11")
    }
    
    fn is_raspberrypi(filename: &str) -> bool {
        filename.contains("raspberrypi")
            || filename.contains("raspi")
            || filename.contains("rpi")
            || filename.contains("raspberry")
    }
    
    fn is_orangepi(filename: &str) -> bool {
        filename.contains("orangepi") || filename.contains("orange-pi")
    }
    
    fn is_esp32(filename: &str) -> bool {
        filename.contains("esp32") || filename.contains("micropython")
    }
    
    fn is_arduino(filename: &str) -> bool {
        filename.contains("arduino") || filename.contains("avr")
    }
    
    fn is_linux(filename: &str) -> bool {
        filename.contains("ubuntu")
            || filename.contains("fedora")
            || filename.contains("debian")
            || filename.contains("arch")
            || filename.contains("manjaro")
            || filename.contains("gentoo")
            || filename.contains("opensuse")
            || filename.contains("linux")
            || filename.contains(".iso")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_windows_detection() {
        assert_eq!(
            Platform::from_iso_path(Path::new("Windows11.iso")),
            Platform::WindowsISO
        );
    }
    
    #[test]
    fn test_raspberrypi_detection() {
        assert_eq!(
            Platform::from_iso_path(Path::new("RaspberryPi-OS.img")),
            Platform::RaspberryPi
        );
    }
    
    #[test]
    fn test_orangepi_detection() {
        assert_eq!(
            Platform::from_iso_path(Path::new("OrangePi-Ubuntu.img")),
            Platform::OrangePi
        );
    }
    
    #[test]
    fn test_esp32_detection() {
        assert_eq!(
            Platform::from_iso_path(Path::new("esp32-micropython.bin")),
            Platform::ESP32
        );
    }
    
    #[test]
    fn test_arduino_detection() {
        assert_eq!(
            Platform::from_iso_path(Path::new("arduino-firmware.hex")),
            Platform::Arduino
        );
    }
    
    #[test]
    fn test_write_optimizations() {
        // Verify Windows ISO gets standard settings
        let (buf, chunk) = Platform::WindowsISO.write_optimization();
        assert!(buf >= 4 && chunk >= 512, "Reasonable buffer and chunk sizes");
        
        // Verify RPi gets smaller buffers
        let (buf_rpi, _chunk_rpi) = Platform::RaspberryPi.write_optimization();
        assert!(buf_rpi < buf, "RPi should have smaller buffer");
    }
    
    #[test]
    fn test_verification_strategies() {
        assert_eq!(
            Platform::WindowsISO.verification_strategy(),
            VerificationStrategy::FullByteCheck
        );
        assert_eq!(
            Platform::ESP32.verification_strategy(),
            VerificationStrategy::FirmwareCheck
        );
    }
    
    #[test]
    fn test_special_handling_required() {
        assert!(Platform::RaspberryPi.requires_special_handling());
        assert!(Platform::ESP32.requires_special_handling());
        assert!(!Platform::WindowsISO.requires_special_handling());
    }
}
