// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// TDLib Rust 客户端核心入口库

// 内部模块：异步响应分发观察者模式实现
mod observer;
// 内部模块：TDLib C JSON 接口 FFI 绑定
mod tdjson;

// 公开模块：生成的联合类型枚举定义
pub mod enums;
// 公开模块：生成的 TDLib 异步 API 请求函数
pub mod functions;
// 公开模块：生成的结构体数据类型
pub mod types;

use crate::enums::Update;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};

/// 全局自增请求计数器，用于为每一个发起的 API 请求分配全局唯一的 `@extra` 标识符
static EXTRA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// 全局懒加载单例观察者，用于协调异步 API 请求与后台响应消息匹配
static OBSERVER: Lazy<observer::Observer> = Lazy::new(observer::Observer::new);

/// 创建一个新的 TDLib 客户端实例并返回分配的 client_id。
/// 注意：客户端刚创建后，必须先向其发送至少一个请求（例如设置参数或验证），TDLib 才会开始为其派发 Update 更新。
pub fn create_client() -> i32 {
    // 调用底层 C 接口分配客户端 ID
    tdjson::create_client()
}

/// 从 TDLib 底层事件队列中拉取单条更新消息或异步调用响应。
/// 
/// # 返回
/// - 若为服务器推送的事件更新（Push Update），则返回 `Some((Update, client_id))` 元组；
/// - 若为特定请求的响应结果（含 `@extra`），则会自动分发唤醒等待中的对应 async Future，此处返回 `None`；
/// - 若在超时时间内无任何消息到达，亦返回 `None`。
pub fn receive() -> Option<(Update, i32)> {
    // 从 tdjson 底层拉取 JSON 字符串，超时等待时间设为 2.0 秒
    let response = tdjson::receive(2.0);

    // 如果接收到了有效的响应字符串
    if let Some(response_str) = response {
        // 将 JSON 字符串解析为动态 serde_json::Value
        let response: Value = match serde_json::from_str(&response_str) {
            Ok(val) => val,
            Err(e) => {
                log::error!("解析 TDLib 响应 JSON 失败: {e}, 原文: {response_str}");
                return None;
            }
        };

        // 检查响应中是否包含 "@extra" 字段
        match response.get("@extra") {
            // 如果含有 "@extra"，说明该条消息是此前某次 API 调用的直接返回结果
            Some(_) => {
                // 通过观察者路由将结果通知给正在 await 该请求的异步任务
                OBSERVER.notify(response);
            }
            // 如果不含有 "@extra"，说明这是 Telegram 服务端推送的异步状态更新事件（Update）
            None => {
                // 提取归属的客户端 ID
                let client_id = response["@client_id"].as_i64().unwrap_or(0) as i32;
                // 将 JSON 转换为具体的 Update 强类型枚举
                match serde_json::from_value::<Update>(response) {
                    Ok(update) => {
                        // 成功解析为事件，向外返回
                        return Some((update, client_id));
                    }
                    Err(e) => {
                        // 遇到未识别或反序列化失败的更新时记录日志（通常是新版协议未覆盖的 Update）
                        log::warn!("收到未识别的 TDLib 更新: {response_str}\n原因: {e}");
                    }
                }
            }
        }
    }

    // 默认返回 None
    None
}

/// 底层通用异步请求发送函数
///
/// # 参数
/// * `client_id` - 目标客户端标识
/// * `mut request` - 准备发送的 JSON 请求数据（将被自动注入 `@extra` 字段）
///
/// # 返回
/// 异步返回服务端对应的原始 JSON 响应值
pub(crate) async fn send_request(client_id: i32, mut request: Value) -> Value {
    // 分配唯一的自增 extra 编号
    let extra = EXTRA_COUNTER.fetch_add(1, Ordering::Relaxed);
    // 将 @extra 写入 JSON 请求体以便服务端回传该标识
    request["@extra"] = serde_json::to_value(extra).unwrap();

    // 在观察者中注册对应 extra 的 oneshot 接收通道
    let receiver = OBSERVER.subscribe(extra);
    // 通过 C FFI 向 tdjson 发送 JSON 字符串
    tdjson::send(client_id, request.to_string());

    // 挂起当前协程，等待后台 receive() 线程捕获到响应并通过 channel 唤醒
    receiver.await.unwrap()
}

