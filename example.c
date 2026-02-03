#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "hid_monitor.h"

#ifdef _WIN32
#include <windows.h>
#define SLEEP(ms) Sleep(ms)
#else
#include <unistd.h>
#define SLEEP(ms) usleep((ms) * 1000)
#endif

void print_device_info(const CDeviceInfo* device) {
    printf("  Path: %s\n", device->path ? device->path : "NULL");
    if (device->has_vid) {
        printf("  VID: 0x%04X\n", device->vid);
    } else {
        printf("  VID: N/A\n");
    }
    if (device->has_pid) {
        printf("  PID: 0x%04X\n", device->pid);
    } else {
        printf("  PID: N/A\n");
    }
}

int main() {
    printf("=== HID Monitor C Example ===\n\n");

    // 1. 列出当前设备
    printf("1. Listing current HID devices:\n");
    uint32_t device_count = 0;
    CDeviceInfo* devices = hid_list_devices(&device_count);
    
    if (devices && device_count > 0) {
        printf("Found %u devices:\n", device_count);
        for (uint32_t i = 0; i < device_count; i++) {
            printf("Device %u:\n", i + 1);
            print_device_info(&devices[i]);
            printf("\n");
        }
        // 释放设备列表内存
        hid_free_device_list(devices, device_count);
    } else {
        printf("No HID devices found.\n");
    }

    // 2. 启动HID监听器
    printf("\n2. Starting HID monitor...\n");
    uint32_t monitor_id = hid_start_monitor();
    
    if (monitor_id == 0) {
        printf("Failed to start HID monitor!\n");
        return 1;
    }
    
    printf("HID monitor started (ID: %u)\n", monitor_id);
    printf("Please plug/unplug HID devices to see events...\n");
    printf("Press Ctrl+C to exit or wait 30 seconds for auto-exit.\n\n");

    // 3. 监听事件 (30秒)
    int event_count = 0;
    int max_events = 100; // 最多接收100个事件
    int timeout_seconds = 30;
    
    for (int i = 0; i < timeout_seconds * 10 && event_count < max_events; i++) {
        CHidEvent event;
        int result = hid_try_recv_event(monitor_id, &event);
        
        if (result == 1) {
            // 成功接收到事件
            event_count++;
            printf("Event %d: ", event_count);
            
            switch (event.event_type) {
                case HID_EVENT_ARRIVED:
                    printf("Device ARRIVED\n");
                    break;
                case HID_EVENT_REMOVED:
                    printf("Device REMOVED\n");
                    break;
                default:
                    printf("Unknown event type\n");
                    break;
            }
            
            print_device_info(&event.device);
            printf("\n");
            
            // 释放事件中的设备信息内存
            hid_free_device_info(&event.device);
        } else if (result == 0) {
            // 没有事件，等待100ms
            SLEEP(100);
        } else if (result == -2) {
            printf("Monitor disconnected!\n");
            break;
        } else if (result == -3) {
            printf("Monitor not found!\n");
            break;
        } else {
            printf("Error receiving event: %d\n", result);
            break;
        }
    }

    // 4. 停止监听器
    printf("\n3. Stopping HID monitor...\n");
    int stop_result = hid_stop_monitor(monitor_id);
    
    if (stop_result == 1) {
        printf("HID monitor stopped successfully.\n");
    } else {
        printf("Failed to stop HID monitor (result: %d)\n", stop_result);
    }

    printf("\nTotal events received: %d\n", event_count);
    printf("Example completed.\n");
    
    return 0;
}