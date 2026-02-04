## hid_monitor

一个用于 **监控 HID 设备插拔事件并枚举当前 HID 设备** 的跨平台 Rust 库，当前支持 **Windows** 和 **macOS**。  
This is a cross-platform Rust library for **monitoring HID device plug/unplug events and listing current HID devices**, currently supporting **Windows** and **macOS**.

---

## 特性 Features

- **设备枚举 Device enumeration**：列出当前所有 HID 设备（含路径、VID、PID 等信息）。  
  List all currently connected HID devices (path, VID, PID, etc.).
- **事件监听 Event monitoring**：实时监听 HID 设备的插入和移除事件。  
  Receive real-time events when HID devices are plugged in or removed.
- **跨平台 Cross-platform**：统一 API，内部根据目标平台调用 Windows / macOS 实现。  
  Unified API with platform-specific implementations under the hood.
- **纯 Rust 接口 Pure Rust API**：当前版本仅提供 Rust API，不再暴露 C FFI 接口。  
  This version only exposes a Rust API; C FFI bindings are not provided.

---

## 安装 Installation

在你的 `Cargo.toml` 中添加：  
Add this to your `Cargo.toml`:

```toml
[dependencies]
hid_monitor = "0.1"
```

> 注意：版本号仅为示例，请以 crates.io 上实际发布的版本为准。  
> Note: The version is an example, please use the actual version published on crates.io.

---

## 快速上手 Quick Start

### 列出当前 HID 设备  
### List current HID devices

```rust
use hid_monitor::{list_devices};

fn main() {
    for device in list_devices() {
        println!(
            "Device: path={} vid={:?} pid={:?}",
            device.path,
            device.vid,
            device.pid,
        );
    }
}
```

### 监听插拔事件  
### Monitor plug/unplug events

```rust
use hid_monitor::{start_hid_monitor, HidEvent};

fn main() {
    let rx = start_hid_monitor();
    println!("HID monitor started. Plug/unplug devices to see events...");

    loop {
        match rx.recv() {
            Ok(HidEvent::Arrived(info)) => {
                println!(
                    "Device arrived: path={} vid={:?} pid={:?}",
                    info.path, info.vid, info.pid
                );
            }
            Ok(HidEvent::Removed(info)) => {
                println!(
                    "Device removed: path={} vid={:?} pid={:?}",
                    info.path, info.vid, info.pid
                );
            }
            Err(e) => {
                eprintln!("monitor channel error: {e}");
                break;
            }
        }
    }
}
```

---

## 平台支持与注意事项 Platform Support & Notes

- **Windows**：基于 `windows` crate 调用 Win32 API。  
  Uses Win32 APIs via the `windows` crate.
- **macOS**：基于 `core-foundation` 与 `io-kit-sys`。  
  Uses `core-foundation` and `io-kit-sys` on macOS.
- **其他平台 Other platforms**：当前返回空设备列表，且不产生事件。  
  On unsupported platforms, the library returns an empty device list and no events.

> 某些系统/环境下，访问 HID 设备可能需要较高权限（例如管理员权限）。  
> On some systems/environments, elevated privileges (e.g. administrator) may be required to access HID devices.

---

## 许可证 License

本项目采用 **MIT License**。  
This project is licensed under the **MIT License**.

