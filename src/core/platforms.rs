/// Platform detection for ISO images and embedded systems
/// Identifies target platform based on ISO filename, size, and magic bytes

use std::path::Path;

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
    
    // Other
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
    pub fn requires_special_handling(&self) -> bool {
        matches!(
            self,
            Platform::RaspberryPi | Platform::OrangePi | Platform::ESP32 | Platform::Arduino
        )
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
}
