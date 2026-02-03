use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, LazyLock};
use std::collections::HashMap;
use std::ptr;

use crate::hid_monitor::{HidEvent, DeviceInfo, start_hid_monitor, list_devices};

// C兼容的设备信息结构
#[repr(C)]
pub struct CDeviceInfo {
    pub path: *mut c_char,
    pub vid: c_uint,
    pub pid: c_uint,
    pub has_vid: c_int,
    pub has_pid: c_int,
}

// C兼容的事件类型
#[repr(C)]
pub enum CEventType {
    Arrived = 0,
    Removed = 1,
}

// C兼容的事件结构
#[repr(C)]
pub struct CHidEvent {
    pub event_type: CEventType,
    pub device: CDeviceInfo,
}

// 全局监听器管理
static mut MONITOR_ID_COUNTER: u32 = 0;
static MONITORS: LazyLock<Mutex<HashMap<u32, Receiver<HidEvent>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// 释放C字符串内存
#[no_mangle]
pub extern "C" fn hid_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// 释放设备信息内存
#[no_mangle]
pub extern "C" fn hid_free_device_info(device: *mut CDeviceInfo) {
    if !device.is_null() {
        unsafe {
            let device_ref = &mut *device;
            if !device_ref.path.is_null() {
                hid_free_string(device_ref.path);
            }
        }
    }
}

// 释放设备列表内存
#[no_mangle]
pub extern "C" fn hid_free_device_list(devices: *mut CDeviceInfo, count: c_uint) {
    if !devices.is_null() {
        unsafe {
            for i in 0..count {
                let device = devices.add(i as usize);
                hid_free_device_info(device);
            }
            let _ = Vec::from_raw_parts(devices, count as usize, count as usize);
        }
    }
}

// 将Rust DeviceInfo转换为C DeviceInfo
fn device_info_to_c(info: &DeviceInfo) -> CDeviceInfo {
    let path_cstring = CString::new(info.path.clone()).unwrap_or_else(|_| CString::new("").unwrap());
    
    CDeviceInfo {
        path: path_cstring.into_raw(),
        vid: info.vid.unwrap_or(0) as c_uint,
        pid: info.pid.unwrap_or(0) as c_uint,
        has_vid: if info.vid.is_some() { 1 } else { 0 },
        has_pid: if info.pid.is_some() { 1 } else { 0 },
    }
}

// 列出当前设备
#[no_mangle]
pub extern "C" fn hid_list_devices(count: *mut c_uint) -> *mut CDeviceInfo {
    if count.is_null() {
        return ptr::null_mut();
    }

    let devices = list_devices();
    let device_count = devices.len();
    
    unsafe {
        *count = device_count as c_uint;
    }

    if device_count == 0 {
        return ptr::null_mut();
    }

    let mut c_devices = Vec::with_capacity(device_count);
    for device in devices {
        c_devices.push(device_info_to_c(&device));
    }

    let ptr = c_devices.as_mut_ptr();
    std::mem::forget(c_devices);
    ptr
}

// 启动HID监听器
#[no_mangle]
pub extern "C" fn hid_start_monitor() -> c_uint {
    let receiver = start_hid_monitor();
    
    unsafe {
        MONITOR_ID_COUNTER += 1;
        let monitor_id = MONITOR_ID_COUNTER;
        
        if let Ok(mut monitors) = MONITORS.lock() {
            monitors.insert(monitor_id, receiver);
            monitor_id
        } else {
            0 // 错误情况返回0
        }
    }
}

// 停止HID监听器
#[no_mangle]
pub extern "C" fn hid_stop_monitor(monitor_id: c_uint) -> c_int {
    if let Ok(mut monitors) = MONITORS.lock() {
        if monitors.remove(&monitor_id).is_some() {
            1 // 成功
        } else {
            0 // 未找到监听器
        }
    } else {
        -1 // 锁定失败
    }
}

// 接收HID事件（非阻塞）
#[no_mangle]
pub extern "C" fn hid_try_recv_event(monitor_id: c_uint, event: *mut CHidEvent) -> c_int {
    if event.is_null() {
        return -1;
    }

    if let Ok(mut monitors) = MONITORS.lock() {
        if let Some(receiver) = monitors.get_mut(&monitor_id) {
            match receiver.try_recv() {
                Ok(hid_event) => {
                    unsafe {
                        match hid_event {
                            HidEvent::Arrived(info) => {
                                (*event).event_type = CEventType::Arrived;
                                (*event).device = device_info_to_c(&info);
                            }
                            HidEvent::Removed(info) => {
                                (*event).event_type = CEventType::Removed;
                                (*event).device = device_info_to_c(&info);
                            }
                        }
                    }
                    1 // 成功接收到事件
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => 0, // 没有事件
                Err(std::sync::mpsc::TryRecvError::Disconnected) => -2, // 连接断开
            }
        } else {
            -3 // 监听器不存在
        }
    } else {
        -1 // 锁定失败
    }
}

// 接收HID事件（阻塞）
#[no_mangle]
pub extern "C" fn hid_recv_event(monitor_id: c_uint, event: *mut CHidEvent) -> c_int {
    if event.is_null() {
        return -1;
    }

    if let Ok(mut monitors) = MONITORS.lock() {
        if let Some(receiver) = monitors.get_mut(&monitor_id) {
            match receiver.recv() {
                Ok(hid_event) => {
                    unsafe {
                        match hid_event {
                            HidEvent::Arrived(info) => {
                                (*event).event_type = CEventType::Arrived;
                                (*event).device = device_info_to_c(&info);
                            }
                            HidEvent::Removed(info) => {
                                (*event).event_type = CEventType::Removed;
                                (*event).device = device_info_to_c(&info);
                            }
                        }
                    }
                    1 // 成功接收到事件
                }
                Err(_) => -2, // 连接断开
            }
        } else {
            -3 // 监听器不存在
        }
    } else {
        -1 // 锁定失败
    }
}