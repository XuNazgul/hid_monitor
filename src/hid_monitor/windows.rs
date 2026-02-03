use std::ffi::c_void;
use std::mem::{size_of};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;
use std::thread;

use windows::core::PCWSTR;
use windows::Win32::Devices::HumanInterfaceDevice::HidD_GetHidGuid;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG, WNDCLASSW, WM_DEVICECHANGE,
    RegisterDeviceNotificationW, HDEVNOTIFY, DEV_BROADCAST_DEVICEINTERFACE_W, DEV_BROADCAST_HDR,
    DEVICE_NOTIFY_WINDOW_HANDLE, DBT_DEVTYP_DEVICEINTERFACE, DBT_DEVICEARRIVAL, DBT_DEVICEREMOVECOMPLETE,
};

use super::{DeviceInfo, HidEvent};

pub fn start_hid_monitor_windows() -> Receiver<HidEvent> {
    let (tx, rx) = mpsc::channel::<HidEvent>();

    thread::spawn(move || unsafe {
        let hinstance = GetModuleHandleW(None).unwrap();

        // Register window class
        let class_name: Vec<u16> = "YjsHidMonitorWnd".encode_utf16().chain(std::iter::once(0)).collect();
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
        };
        RegisterClassW(&wnd_class);

        // Initialize global sender for callbacks
        _init_global_tx(tx);

        // Create hidden window
        let hwnd = CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            Default::default(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND(std::ptr::null_mut()),
            None,
            hinstance,
            None,
        ).unwrap();

        // Register for HID device interface notifications
        let hid_guid = HidD_GetHidGuid();
        let mut filter = DEV_BROADCAST_DEVICEINTERFACE_W::default();
        filter.dbcc_size = size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>() as u32;
        filter.dbcc_devicetype = DBT_DEVTYP_DEVICEINTERFACE.0;
        filter.dbcc_classguid = hid_guid;

        let _notify: HDEVNOTIFY = RegisterDeviceNotificationW(
            hwnd,
            (&filter as *const DEV_BROADCAST_DEVICEINTERFACE_W) as *const c_void,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        ).unwrap();

        // Message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });

    rx
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DEVICECHANGE => {
            match wparam.0 as u32 {
                DBT_DEVICEARRIVAL => {
                    handle_device_change(hwnd, lparam, true);
                }
                DBT_DEVICEREMOVECOMPLETE => {
                    handle_device_change(hwnd, lparam, false);
                }
                _ => {}
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

// Use OnceLock to avoid mutable static reference warnings
static GLOBAL_TX: OnceLock<Mutex<Sender<HidEvent>>> = OnceLock::new();
use std::sync::Mutex;

pub fn _init_global_tx(tx: Sender<HidEvent>) {
    let _ = GLOBAL_TX.set(Mutex::new(tx));
}

unsafe fn send_event(event: HidEvent) {
    if let Some(m) = GLOBAL_TX.get() {
        let _ = m.lock().unwrap().send(event);
    }
}

unsafe fn parse_device_interface(lparam: LPARAM) -> Option<String> {
    if lparam.0 == 0 { return None; }
    let hdr = lparam.0 as *const DEV_BROADCAST_HDR;
    if hdr.is_null() { return None; }
    if (*hdr).dbch_devicetype != DBT_DEVTYP_DEVICEINTERFACE { return None; }
    let di = hdr as *const DEV_BROADCAST_DEVICEINTERFACE_W;
    if di.is_null() { return None; }
    let name_ptr = (*di).dbcc_name.as_ptr();
    if name_ptr.is_null() { return None; }
    // Convert wide string
    let mut len = 0usize;
    loop {
        let ch = *name_ptr.add(len);
        if ch == 0 { break; }
        len += 1;
    }
    let slice = std::slice::from_raw_parts(name_ptr, len);
    Some(String::from_utf16_lossy(slice))
}

unsafe fn handle_device_change(_hwnd: HWND, lparam: LPARAM, arrival: bool) {
    if let Some(path) = parse_device_interface(lparam) {
        let (vid, pid) = parse_vid_pid_from_path(&path);
        let info = DeviceInfo { path, vid, pid };
        if arrival {
            send_event(HidEvent::Arrived(info));
        } else {
            send_event(HidEvent::Removed(info));
        }
    }
}

fn parse_vid_pid_from_path(path: &str) -> (Option<u16>, Option<u16>) {
    // Typical path contains vid_XXXX&pid_YYYY
    let lower = path.to_ascii_lowercase();
    let mut vid = None;
    let mut pid = None;
    for part in lower.split(&['#', '&'][..]) {
        if let Some(hex) = part.strip_prefix("vid_") {
            if let Ok(v) = u16::from_str_radix(hex, 16) { vid = Some(v); }
        }
        if let Some(hex) = part.strip_prefix("pid_") {
            if let Ok(p) = u16::from_str_radix(hex, 16) { pid = Some(p); }
        }
    }
    (vid, pid)
}

pub fn list_devices_windows() -> Vec<DeviceInfo> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiGetClassDevsW, SetupDiEnumDeviceInterfaces, SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE,
        DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W, HDEVINFO,
    };

    let mut devices = Vec::new();
    unsafe {
        let hid_guid = HidD_GetHidGuid();
        let hdevinfo: HDEVINFO = SetupDiGetClassDevsW(Some(&hid_guid), PCWSTR::null(), HWND(std::ptr::null_mut()), DIGCF_DEVICEINTERFACE | DIGCF_PRESENT).unwrap();
        if hdevinfo.is_invalid() {
            return devices;
        }
        let mut index = 0u32;
        loop {
            let mut iface_data = SP_DEVICE_INTERFACE_DATA { cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInterfaces(hdevinfo, None, &hid_guid, index, &mut iface_data).is_err() {
                break;
            }

            // Query required buffer size
            let mut required = 0u32;
            let _ = SetupDiGetDeviceInterfaceDetailW(hdevinfo, &iface_data, None, 0, Some(&mut required), None);
            if required == 0 { index += 1; continue; }

            // Allocate detail buffer
            let mut detail: Vec<u8> = vec![0u8; required as usize];
            // Cast to struct and set cbSize
            let p_detail = detail.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            (*p_detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

            if SetupDiGetDeviceInterfaceDetailW(hdevinfo, &iface_data, Some(p_detail), required, Some(&mut required), None).is_ok() {
                // Extract device path
                let ptr = (*p_detail).DevicePath.as_ptr();
                // Convert to String
                let mut len = 0usize;
                while *ptr.add(len) != 0 { len += 1; }
                let slice = std::slice::from_raw_parts(ptr, len);
                let path = String::from_utf16_lossy(slice);
                let (vid, pid) = parse_vid_pid_from_path(&path);
                devices.push(DeviceInfo { path, vid, pid });
            }
            index += 1;
        }
    }
    devices
}