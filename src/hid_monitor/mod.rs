use std::sync::mpsc::Receiver;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
}

#[derive(Debug, Clone)]
pub enum HidEvent {
    Arrived(DeviceInfo),
    Removed(DeviceInfo),
}

pub fn start_hid_monitor() -> Receiver<HidEvent> {
    #[cfg(target_os = "windows")]
    {
        super::hid_monitor::windows::start_hid_monitor_windows()
    }
    #[cfg(target_os = "macos")]
    {
        super::hid_monitor::macos::start_hid_monitor_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let (_tx, rx) = std::sync::mpsc::channel();
        rx
    }
}

pub fn list_devices() -> Vec<DeviceInfo> {
    #[cfg(target_os = "windows")]
    {
        super::hid_monitor::windows::list_devices_windows()
    }
    #[cfg(target_os = "macos")]
    {
        super::hid_monitor::macos::list_devices_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;