#ifndef HID_MONITOR_H
#define HID_MONITOR_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

// 设备信息结构
typedef struct {
    char* path;           // 设备路径
    uint32_t vid;         // 厂商ID
    uint32_t pid;         // 产品ID
    int32_t has_vid;      // 是否有厂商ID (1=有, 0=无)
    int32_t has_pid;      // 是否有产品ID (1=有, 0=无)
} CDeviceInfo;

// 事件类型枚举
typedef enum {
    HID_EVENT_ARRIVED = 0,  // 设备插入
    HID_EVENT_REMOVED = 1   // 设备移除
} CEventType;

// HID事件结构
typedef struct {
    CEventType event_type;      // 事件类型
    CDeviceInfo device;         // 设备信息
} CHidEvent;

// 内存管理函数
/**
 * 释放字符串内存
 * @param ptr 要释放的字符串指针
 */
void hid_free_string(char* ptr);

/**
 * 释放设备信息内存
 * @param device 要释放的设备信息指针
 */
void hid_free_device_info(CDeviceInfo* device);

/**
 * 释放设备列表内存
 * @param devices 设备列表指针
 * @param count 设备数量
 */
void hid_free_device_list(CDeviceInfo* devices, uint32_t count);

// 设备管理函数
/**
 * 列出当前连接的HID设备
 * @param count 输出参数，返回设备数量
 * @return 设备信息数组指针，使用完毕后需要调用 hid_free_device_list 释放内存
 */
CDeviceInfo* hid_list_devices(uint32_t* count);

// 监听器管理函数
/**
 * 启动HID设备监听器
 * @return 监听器ID，失败返回0
 */
uint32_t hid_start_monitor(void);

/**
 * 停止HID设备监听器
 * @param monitor_id 监听器ID
 * @return 1=成功, 0=监听器不存在, -1=错误
 */
int32_t hid_stop_monitor(uint32_t monitor_id);

// 事件接收函数
/**
 * 非阻塞方式接收HID事件
 * @param monitor_id 监听器ID
 * @param event 输出参数，接收到的事件信息
 * @return 1=成功接收到事件, 0=没有事件, -1=参数错误, -2=连接断开, -3=监听器不存在
 */
int32_t hid_try_recv_event(uint32_t monitor_id, CHidEvent* event);

/**
 * 阻塞方式接收HID事件
 * @param monitor_id 监听器ID
 * @param event 输出参数，接收到的事件信息
 * @return 1=成功接收到事件, -1=参数错误, -2=连接断开, -3=监听器不存在
 */
int32_t hid_recv_event(uint32_t monitor_id, CHidEvent* event);

#ifdef __cplusplus
}
#endif

#endif // HID_MONITOR_H

