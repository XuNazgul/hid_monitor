use std::sync::mpsc::{self, Receiver, Sender};

use super::{DeviceInfo, HidEvent};

use std::collections::HashSet;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::ptr;
use std::ffi::CString;

// Minimal FFI bindings we need (avoid external wrappers for simplicity)
#[allow(non_camel_case_types)]
type IOHIDManagerRef = *mut c_void;
#[allow(non_camel_case_types)]
type IOHIDDeviceRef = *mut c_void;
#[allow(non_camel_case_types)]
type IOHIDElementRef = *mut c_void;
#[allow(non_camel_case_types)]
type CFAllocatorRef = *const c_void;
#[allow(non_camel_case_types)]
type CFDictionaryRef = *const c_void;
#[allow(non_camel_case_types)]
type CFSetRef = *const c_void;
#[allow(non_camel_case_types)]
type CFArrayRef = *const c_void;
#[allow(non_camel_case_types)]
type CFRunLoopRef = *mut c_void;
#[allow(non_camel_case_types)]
type CFStringRef = *const c_void;
#[allow(non_camel_case_types)]
type CFTypeRef = *const c_void;
#[allow(non_camel_case_types)]
type CFNumberRef = *const c_void;
#[allow(non_camel_case_types)]
type IOReturn = i32;
#[allow(non_camel_case_types)]
type io_registry_entry_t = u32;
#[allow(non_camel_case_types)]
type kern_return_t = i32;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOHIDManagerCreate(allocator: CFAllocatorRef, options: u32) -> IOHIDManagerRef;
    fn IOHIDManagerSetDeviceMatching(manager: IOHIDManagerRef, matching: CFDictionaryRef);
    fn IOHIDManagerOpen(manager: IOHIDManagerRef, options: u32) -> IOReturn;
    fn IOHIDManagerCopyDevices(manager: IOHIDManagerRef) -> CFSetRef;
    fn IOHIDManagerScheduleWithRunLoop(manager: IOHIDManagerRef, runLoop: CFRunLoopRef, runLoopMode: CFStringRef);
    fn IOHIDManagerRegisterDeviceMatchingCallback(manager: IOHIDManagerRef, callback: extern "C" fn(*mut c_void, i32, *mut c_void, IOHIDDeviceRef), context: *mut c_void);
    fn IOHIDManagerRegisterDeviceRemovalCallback(manager: IOHIDManagerRef, callback: extern "C" fn(*mut c_void, i32, *mut c_void, IOHIDDeviceRef), context: *mut c_void);

    fn IOHIDDeviceGetProperty(device: IOHIDDeviceRef, key: CFStringRef) -> CFTypeRef;
    fn IOHIDDeviceGetService(device: IOHIDDeviceRef) -> io_registry_entry_t;
    fn IOHIDDeviceCopyMatchingElements(device: IOHIDDeviceRef, matching: CFDictionaryRef, options: u32) -> CFArrayRef;

    fn IOHIDElementGetType(element: IOHIDElementRef) -> u32;
    fn IOHIDElementGetCollectionType(element: IOHIDElementRef) -> u32;
    fn IOHIDElementGetUsagePage(element: IOHIDElementRef) -> u32;
    fn IOHIDElementGetUsage(element: IOHIDElementRef) -> u32;
    fn IOHIDElementGetCookie(element: IOHIDElementRef) -> u32;

    fn IORegistryEntryGetPath(entry: io_registry_entry_t, plane: *const c_char, path: *mut c_char, path_size: u32) -> i32;
    fn IORegistryEntryGetRegistryEntryID(entry: io_registry_entry_t, entry_id: *mut u64) -> kern_return_t;

    // CFString constants from IOKit HID headers
    // static kIOHIDVendorIDKey: CFStringRef;
    // static kIOHIDProductIDKey: CFStringRef;
    // static kIOHIDLocationIDKey: CFStringRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFAllocatorDefault: CFAllocatorRef;
    static kCFRunLoopDefaultMode: CFStringRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopRun();
    fn CFRelease(cf: *const c_void);
    fn CFSetGetCount(set: CFSetRef) -> isize;
    fn CFSetGetValues(set: CFSetRef, values: *mut *const c_void);
    fn CFArrayGetCount(array: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: isize) -> *const c_void;
    fn CFNumberGetType(number: CFNumberRef) -> i32;
    fn CFNumberGetValue(number: CFNumberRef, theType: i32, valuePtr: *mut i32) -> bool;
    fn CFStringCreateWithCString(alloc: CFAllocatorRef, cStr: *const c_char, encoding: u32) -> CFStringRef;
}

const K_CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

const K_IOHID_ELEMENT_TYPE_COLLECTION: u32 = 513;
const K_IOHID_COLLECTION_TYPE_APPLICATION: u32 = 1;

unsafe fn cfnumber_to_u16(n: CFTypeRef) -> Option<u16> {
    if n.is_null() { return None; }
    let num = n as CFNumberRef;
    let ty = CFNumberGetType(num);
    let mut v: i32 = 0;
    if CFNumberGetValue(num, ty, &mut v as *mut i32) { Some(v as u16) } else { None }
}

unsafe fn cfstring_key(name: &str) -> CFStringRef {
    let c = CString::new(name).unwrap();
    CFStringCreateWithCString(kCFAllocatorDefault, c.as_ptr(), K_CFSTRING_ENCODING_UTF8)
}

unsafe fn get_u16_prop(dev: IOHIDDeviceRef, key_name: &str) -> Option<u16> {
    let key = cfstring_key(key_name);
    let v = IOHIDDeviceGetProperty(dev, key);
    CFRelease(key as *const c_void);
    cfnumber_to_u16(v)
}

unsafe fn device_base_path(dev: IOHIDDeviceRef) -> String {
    let service = IOHIDDeviceGetService(dev);
    if service != 0 {
        let mut entry_id: u64 = 0;
        if IORegistryEntryGetRegistryEntryID(service, &mut entry_id as *mut u64) == 0 {
            return format!("DevSrvsID:{entry_id}");
        }

        let mut buf = [0i8; 512];
        let plane = CString::new("IOService").unwrap();
        if IORegistryEntryGetPath(service, plane.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) == 0 {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let bytes: Vec<u8> = buf[..len].iter().map(|&c| c as u8).collect();
            return String::from_utf8_lossy(&bytes).into_owned();
        }
    }

    format!("macos-hid:svc={service}")
}

unsafe fn device_to_collection_infos(dev: IOHIDDeviceRef) -> Vec<DeviceInfo> {
    let vid = get_u16_prop(dev, "VendorID");
    let pid = get_u16_prop(dev, "ProductID");
    let base_path = device_base_path(dev);

    let mut infos: Vec<DeviceInfo> = Vec::new();
    let mut seen: HashSet<u32> = HashSet::new();

    let elements = IOHIDDeviceCopyMatchingElements(dev, ptr::null(), 0);
    if !elements.is_null() {
        let count = CFArrayGetCount(elements);
        if count > 0 {
            for i in 0..count {
                let p = CFArrayGetValueAtIndex(elements, i);
                if p.is_null() {
                    continue;
                }
                let el = p as IOHIDElementRef;
                let ty = IOHIDElementGetType(el);
                if ty != K_IOHID_ELEMENT_TYPE_COLLECTION {
                    continue;
                }
                let ct = IOHIDElementGetCollectionType(el);
                if ct != K_IOHID_COLLECTION_TYPE_APPLICATION {
                    continue;
                }
                let cookie = IOHIDElementGetCookie(el);
                if !seen.insert(cookie) {
                    continue;
                }
                let usage_page = IOHIDElementGetUsagePage(el);
                let usage = IOHIDElementGetUsage(el);
                infos.push(DeviceInfo {
                    path: base_path.clone(),
                    vid,
                    pid,
                    usage_page: u16::try_from(usage_page).ok(),
                    usage: u16::try_from(usage).ok(),
                });
            }
        }
        CFRelease(elements as *const c_void);
    }

    if infos.is_empty() {
        let usage_page = get_u16_prop(dev, "PrimaryUsagePage");
        let usage = get_u16_prop(dev, "PrimaryUsage");
        infos.push(DeviceInfo {
            path: base_path,
            vid,
            pid,
            usage_page,
            usage,
        });
    }

    infos
}

// Implement device enumeration via IOHIDManagerCopyDevices
pub fn list_devices_macos() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    unsafe {
        let mgr = IOHIDManagerCreate(kCFAllocatorDefault, 0);
        if mgr.is_null() { return devices; }
        IOHIDManagerSetDeviceMatching(mgr, ptr::null());
        let _ = IOHIDManagerOpen(mgr, 0);
        let set = IOHIDManagerCopyDevices(mgr);
        if !set.is_null() {
            let count = CFSetGetCount(set);
            if count > 0 {
                let mut arr: Vec<*const c_void> = vec![ptr::null(); count as usize];
                CFSetGetValues(set, arr.as_mut_ptr());
                for p in arr {
                    if !p.is_null() {
                        let dev = p as IOHIDDeviceRef;
                        devices.extend(device_to_collection_infos(dev));
                    }
                }
            }
            CFRelease(set as *const c_void);
        }
        CFRelease(mgr as *const c_void);
    }
    devices
}

// Callbacks forward HID events into a Sender
extern "C" fn on_match(ctx: *mut c_void, _result: i32, _sender: *mut c_void, dev: IOHIDDeviceRef) {
    unsafe {
        if ctx.is_null() { return; }
        let tx = &*(ctx as *mut Sender<HidEvent>);
        for info in device_to_collection_infos(dev) {
            let _ = tx.send(HidEvent::Arrived(info));
        }
    }
}

extern "C" fn on_remove(ctx: *mut c_void, _result: i32, _sender: *mut c_void, dev: IOHIDDeviceRef) {
    unsafe {
        if ctx.is_null() { return; }
        let tx = &*(ctx as *mut Sender<HidEvent>);
        for info in device_to_collection_infos(dev) {
            let _ = tx.send(HidEvent::Removed(info));
        }
    }
}

// Start HID monitor using IOHIDManager callbacks + CFRunLoop
pub fn start_hid_monitor_macos() -> Receiver<HidEvent> {
    let (tx, rx) = mpsc::channel::<HidEvent>();
    std::thread::spawn(move || unsafe {
        let mgr = IOHIDManagerCreate(kCFAllocatorDefault, 0);
        if mgr.is_null() { return; }
        IOHIDManagerSetDeviceMatching(mgr, ptr::null());
        let _ = IOHIDManagerOpen(mgr, 0);
        let ctx = Box::into_raw(Box::new(tx)) as *mut c_void;
        IOHIDManagerRegisterDeviceMatchingCallback(mgr, on_match, ctx);
        IOHIDManagerRegisterDeviceRemovalCallback(mgr, on_remove, ctx);
        let rl = CFRunLoopGetCurrent();
        IOHIDManagerScheduleWithRunLoop(mgr, rl, kCFRunLoopDefaultMode);
        CFRunLoopRun();
        // We may never get here under normal conditions; if we do, clean up.
        CFRelease(mgr as *const c_void);
        let _ = Box::from_raw(ctx as *mut Sender<HidEvent>);
    });
    rx
}
