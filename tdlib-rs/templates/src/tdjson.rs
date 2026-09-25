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

    /// 同步立即执行一个无需网络通信的 TDLib 本地请求并返回 JSON 结果（如获取版本、验证语法等）
    fn td_execute(request: *const c_char) -> *const c_char;
}

/// 创建客户端的安全 Rust 封装函数
pub(crate) fn create_client() -> i32 {
    unsafe { td_create_client_id() }
}

/// 发送请求的安全 Rust 封装函数
///
/// # 参数
/// * `client_id` - 目标客户端标识
/// * `request` - JSON 格式的字符串请求体
pub(crate) fn send(client_id: i32, request: &str) {
    if let Ok(cstring) = CString::new(request) {
        unsafe { td_send(client_id, cstring.as_ptr()) }
    } else {
        log::error!("tdjson::send: failed to construct CString (contained null byte)");
    }
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
        td_receive(timeout)
            .as_ref()
            .map(|response| CStr::from_ptr(response).to_string_lossy().into_owned())
    }
}

/// 同步立即执行请求的安全 Rust 封装函数
///
/// 仅用于 TDLib 支持的同步查询（如获取选项、格式化纯文本等本地操作）。
pub fn execute(request: &str) -> Option<String> {
    if let Ok(cstring) = CString::new(request) {
        unsafe {
            td_execute(cstring.as_ptr())
                .as_ref()
                .map(|response| CStr::from_ptr(response).to_string_lossy().into_owned())
        }
    } else {
        None
    }
}
