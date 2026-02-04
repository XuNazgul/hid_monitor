# HID Monitor SDK
 
一个跨平台的HID设备监控库，支持Windows和macOS系统。提供Rust原生接口，可以监控HID设备的插拔事件并列出当前连接的设备。

## 功能特性

- 🔌 实时监控HID设备插拔事件
- 📋 列出当前连接的HID设备
- 🖥️ 支持Windows和macOS平台
- 🦀 提供Rust原生接口
- 🔗 提供C FFI接口，支持其他语言调用
- 📦 支持静态库和动态库编译

## 编译

### 编译所有目标

```bash
cargo build --release
```

### 仅编译库文件

```bash
cargo build --release --lib
```

### 仅编译示例程序
 
```bash
cargo build --release --bin hid_monitor_example
```

## 生成的文件

编译完成后，在 `target/release/` 目录下会生成以下文件：

- `libhid_monitor.rlib` - Rust 库
- `hid_monitor_example` / `hid_monitor_example.exe` - 示例程序

## 使用方法

### Rust接口

```rust
use hid_monitor::{start_hid_monitor, list_devices, HidEvent};

fn main() {
    // 列出当前设备
    for device in list_devices() {
        println!("Device: path={}, vid={:?}, pid={:?}", 
                 device.path, device.vid, device.pid);
    }

    // 启动监听器
    let rx = start_hid_monitor();
    
    // 接收事件
    loop {
        match rx.recv() {
            Ok(HidEvent::Arrived(info)) => {
                println!("Device arrived: {:?}", info);
            }
            Ok(HidEvent::Removed(info)) => {
                println!("Device removed: {:?}", info);
            }
            Err(_) => break,
        }
    }
}
```

### C 接口

目前本库**不再提供** C 语言 FFI 接口与头文件，仅作为 Rust 库使用。如果后续需要 C 接口，可以在新的版本中单独设计和实现 FFI 层。

## 注意事项

1. **内存管理**: 使用C接口时，必须调用相应的释放函数来避免内存泄漏
2. **线程安全**: 库内部使用了线程安全的设计，可以在多线程环境中使用
3. **平台兼容**: 在不支持的平台上，函数会返回空结果但不会崩溃
4. **权限要求**: 在某些系统上可能需要管理员权限来监控HID设备

## 许可证

本项目采用MIT许可证。详见LICENSE文件。