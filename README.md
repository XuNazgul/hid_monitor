# HID Monitor SDK
 
一个跨平台的HID设备监控库，支持Windows和macOS系统。提供Rust原生接口和C FFI接口，可以监控HID设备的插拔事件并列出当前连接的设备。

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

### Windows平台
- `hid_monitor.dll` - 动态链接库
- `hid_monitor.lib` - 静态库
- `hid_monitor.dll.lib` - 导入库（用于链接DLL）
- `libhid_monitor.rlib` - Rust库格式

### macOS平台
- `libhid_monitor.dylib` - 动态链接库
- `libhid_monitor.a` - 静态库
- `libhid_monitor.rlib` - Rust库格式

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

### C接口

#### 头文件

包含头文件：
```c
#include "hid_monitor.h"
```

#### 基本使用

```c
#include <stdio.h>
#include "yjs_hid_monitor.h"

int main() {
    // 列出当前设备
    uint32_t count = 0;
    CDeviceInfo* devices = yjs_hid_list_devices(&count);
    
    if (devices) {
        for (uint32_t i = 0; i < count; i++) {
            printf("Device: %s, VID: 0x%04X, PID: 0x%04X\\n",
                   devices[i].path, devices[i].vid, devices[i].pid);
        }
        yjs_hid_free_device_list(devices, count);
    }

    // 启动监听器
    uint32_t monitor_id = yjs_hid_start_monitor();
    if (monitor_id == 0) {
        printf("Failed to start monitor\\n");
        return 1;
    }

    // 接收事件
    CHidEvent event;
    while (1) {
        int result = yjs_hid_try_recv_event(monitor_id, &event);
        if (result == 1) {
            if (event.event_type == YJS_HID_EVENT_ARRIVED) {
                printf("Device arrived: %s\\n", event.device.path);
            } else {
                printf("Device removed: %s\\n", event.device.path);
            }
            yjs_hid_free_device_info(&event.device);
        } else if (result == 0) {
            // 没有事件，等待
            usleep(100000); // 100ms
        } else {
            // 错误或断开连接
            break;
        }
    }

    // 停止监听器
    yjs_hid_stop_monitor(monitor_id);
    return 0;
}
```

#### 编译C示例

##### Windows (使用MSVC)
```cmd
cl example.c /I. hid_monitor.dll.lib /Fe:example.exe
```

##### Windows (使用MinGW)
```bash
gcc example.c -I. -L. -lhid_monitor -o example.exe
```

##### macOS
```bash
gcc example.c -I. -L. -lhid_monitor -o example
```

## API参考

### C接口函数

#### 设备管理
- `CDeviceInfo* hid_list_devices(uint32_t* count)` - 列出当前设备
- `void hid_free_device_list(CDeviceInfo* devices, uint32_t count)` - 释放设备列表

#### 监听器管理
- `uint32_t hid_start_monitor(void)` - 启动监听器
- `int32_t hid_stop_monitor(uint32_t monitor_id)` - 停止监听器

#### 事件接收
- `int32_t hid_try_recv_event(uint32_t monitor_id, CHidEvent* event)` - 非阻塞接收事件
- `int32_t hid_recv_event(uint32_t monitor_id, CHidEvent* event)` - 阻塞接收事件

#### 内存管理
- `void yjs_hid_free_string(char* ptr)` - 释放字符串
- `void yjs_hid_free_device_info(CDeviceInfo* device)` - 释放设备信息

### 数据结构

#### CDeviceInfo
```c
typedef struct {
    char* path;           // 设备路径
    uint32_t vid;         // 厂商ID
    uint32_t pid;         // 产品ID
    int32_t has_vid;      // 是否有厂商ID
    int32_t has_pid;      // 是否有产品ID
} CDeviceInfo;
```

#### CHidEvent
```c
typedef struct {
    CEventType event_type;      // 事件类型
    CDeviceInfo device;         // 设备信息
} CHidEvent;
```

#### CEventType
```c
typedef enum {
    HID_EVENT_ARRIVED = 0,  // 设备插入
    HID_EVENT_REMOVED = 1   // 设备移除
} CEventType;
```

## 返回值说明

### 监听器函数返回值
- `hid_start_monitor()`: 返回监听器ID，0表示失败
- `hid_stop_monitor()`: 1=成功, 0=监听器不存在, -1=错误

### 事件接收函数返回值
- `1`: 成功接收到事件
- `0`: 没有事件（仅非阻塞模式）
- `-1`: 参数错误
- `-2`: 连接断开
- `-3`: 监听器不存在

## 注意事项

1. **内存管理**: 使用C接口时，必须调用相应的释放函数来避免内存泄漏
2. **线程安全**: 库内部使用了线程安全的设计，可以在多线程环境中使用
3. **平台兼容**: 在不支持的平台上，函数会返回空结果但不会崩溃
4. **权限要求**: 在某些系统上可能需要管理员权限来监控HID设备

## 许可证

本项目采用MIT许可证。详见LICENSE文件。