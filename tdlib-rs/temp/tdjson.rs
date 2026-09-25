// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int};

// 声明外部 C 动态链接库 "tdjson"
#[link(name = "tdjson")]
unsafe extern "C" {
    /// 创建一个新的 TDLib 客户端实例，并返回该客户端的唯一数值 ID
    fn td_create_client_id() -> c_int;

    /// 异步向指定 client_id 发送 JSON 格式的请求命令（非阻塞）
    fn td_send(client_id: c_int, request: *const c_char);

    /// 从底层全局事件队列中接收下一条 JSON 消息（更新或响应），最多等待 timeout 秒
    /// 若超时时间内无任何消息返回 NULL 指针
    fn td_receive(timeout: c_double) -> *const c_char;
}

/// 创建客户端的安全 Rust 封装函数
pub(crate) fn create_client() -> i32 {
    // 调用底层 C 接口
    unsafe { td_create_client_id() }
}

/// 发送请求的安全 Rust 封装函数
///
/// # 参数
/// * `client_id` - 目标客户端标识
/// * `request` - JSON 格式的字符串请求体
pub(crate) fn send(client_id: i32, request: String) {
    // 将 Rust 字符串转换为以 '\0' 结尾的 C 兼容字符串指针
    let cstring = CString::new(request).unwrap();
    // 调用底层 C FFI 接口发送请求
    unsafe { td_send(client_id, cstring.as_ptr()) }
}

/// 接收更新/响应的安全 Rust 封装函数
///
/// # 参数
/// * `timeout` - 接收超时时间（单位：秒）
///
/// # 返回
/// 成功接收到消息返回 `Some(String)`，超时或无消息返回 `None`
pub(crate) fn receive(timeout: f64) -> Option<String> {
    unsafe {
        // 调用底层 C 接口获取消息指针
        td_receive(timeout)
            // 检查指针非空并转换为引用
            .as_ref()
            // 将 C 风格字符串转换为 Rust 的独立 String 所有权对象
            .map(|response| CStr::from_ptr(response).to_string_lossy().into_owned())
    }
}
