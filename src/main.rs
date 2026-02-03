use hid_monitor::{start_hid_monitor, list_devices, HidEvent};

fn main() {
    // Print current devices without sending HID GET_DESCRIPTOR
    for d in list_devices() {
        println!("present: path={} vid={:?} pid={:?}", d.path, d.vid, d.pid);
    }

    // Start HID monitor
    let rx = start_hid_monitor();
    println!("HID monitor started. Plug/unplug devices to see events...");
    // Block and print events
    loop {
        match rx.recv() {
            Ok(HidEvent::Arrived(info)) => {
                println!("arrived: path={} vid={:?} pid={:?}", info.path, info.vid, info.pid);
            }
            Ok(HidEvent::Removed(info)) => {
                println!("removed: path={} vid={:?} pid={:?}", info.path, info.vid, info.pid);
            }
            Err(e) => {
                eprintln!("monitor channel error: {}", e);
                break;
            }
        }
    }
}
