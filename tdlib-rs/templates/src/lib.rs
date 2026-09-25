// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! TDLib (Telegram Database Library) Rust 客户端核心运行时
//!
//! 具备以下企业级特性：
//! - **原生多客户端/多账号并发**：各客户端拥有完全隔离的私有更新队列，绝不串号、不抢消息；
//! - **全异步、防卡死运行时**：后台专有独立 OS 守护线程接管 C 阻塞轮询，绝对不阻塞 Tokio Worker 线程；
//! - **自动事件泵驱动**：发起 API 请求无需手动编写轮询循环驱动，调用 `await` 即刻唤醒返回；
//! - **超时与取消安全**：支持配合 `tokio::time::timeout` 使用，Future 取消自动清理内存请求表；
//! - **100% 协议与 JSON Wire 一致性**。

// 内部模块：异步响应与更新解复用分发器
mod observer;
// 内部模块：TDLib C JSON 接口 FFI 绑定
mod tdjson;

// 公开模块：生成的联合类型枚举定义
pub mod enums;
// 公开模块：生成的 TDLib 异步 API 请求函数
pub mod functions;
// 公开模块：生成的结构体数据类型
pub mod types;

pub use crate::enums::Update;
pub use crate::tdjson::execute;

use futures_channel::mpsc;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Once;

/// 全局自增请求计数器，用于为每一个发起的 API 请求分配全局唯一的 `@extra` 标识符
static EXTRA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// 全局懒加载单例事件解复用分发中心
pub(crate) static DISPATCHER: Lazy<observer::Dispatcher> = Lazy::new(observer::Dispatcher::new);

/// 保证独立的后台 OS 守护线程启动（只启动一次，绝不卡死 Tokio Worker 线程）
fn ensure_event_loop_started() {
    static START_ONCE: Once = Once::new();
    START_ONCE.call_once(|| {
        std::thread::Builder::new()
            .name("tdlib-event-loop".into())
            .spawn(|| {
                loop {
                    // 在专用独立 OS 线程中进行底层阻塞轮询，完全与外部异步调度器隔离
                    if let Some(json_str) = tdjson::receive(2.0) {
                        if let Ok(val) = serde_json::from_str(&json_str) {
                            DISPATCHER.dispatch(val);
                        } else {
                            log::warn!("Received invalid JSON from TDLib: {json_str}");
                        }
                    }
                }
            })
            .expect("Failed to spawn tdlib event loop thread");
    });
}

/// 代表一个独立的 TDLib 客户端实例（对应一个独立的 Telegram 账号）
///
/// 具备独立的更新接收通道，多个 `Client` 实例可完全并发运行。
///
/// # 示例
///
/// ```rust,no_run
/// use tdlib_rs::Client;
///
/// #[tokio::main]
/// async fn main() {
///     let mut client = Client::new();
///     println!("Client created with id: {}", client.id());
///     
///     while let Some(update) = client.receive().await {
///         println!("Received update: {:?}", update);
///     }
/// }
/// ```
pub struct Client {
    id: i32,
    updates: mpsc::UnboundedReceiver<Update>,
}

impl Client {
    /// 创建一个新的独立客户端，自动启动后台事件泵驱动
    pub fn new() -> Self {
        ensure_event_loop_started();
        let id = tdjson::create_client();
        let updates = DISPATCHER.register_client(id);
        Self { id, updates }
    }

    /// 获取分配给该客户端的唯一数值标识符（client_id）
    pub fn id(&self) -> i32 {
        self.id
    }

    /// 纯异步接收该客户端专属的推送更新（Update）。
    ///
    /// 无新消息时当前协程挂起让出 CPU，有消息到达时微秒级唤醒，绝不卡死 Tokio 调度器。
    pub async fn receive(&mut self) -> Option<Update> {
        self.updates.next().await
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        DISPATCHER.unregister_client(self.id);
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建一个新的 TDLib 客户端实例并返回分配的 client_id（旧版兼容接口）
pub fn create_client() -> i32 {
    ensure_event_loop_started();
    tdjson::create_client()
}

/// 从底层事件队列拉取单条更新（旧版兼容接口）
///
/// 提示：在多账号或现代异步工程中，推荐使用 `Client::new()` 与 `client.receive().await`。
pub fn receive() -> Option<(Update, i32)> {
    let response_str = tdjson::receive(2.0)?;
    let response: Value = serde_json::from_str(&response_str).ok()?;

    if response.get("@extra").is_some() {
        DISPATCHER.dispatch(response);
        None
    } else {
        let client_id = response
            .get("@client_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        match serde_json::from_value::<Update>(response) {
            Ok(update) => Some((update, client_id)),
            Err(e) => {
                log::warn!("Unrecognized update: {response_str}\nError: {e}");
                None
            }
        }
    }
}

/// 底层通用异步请求发送函数（对生成的 1000+ API 函数 100% 保持签名兼容）
pub(crate) async fn send_request(client_id: i32, mut request: Value) -> Value {
    ensure_event_loop_started();
    let extra = EXTRA_COUNTER.fetch_add(1, Ordering::Relaxed);
    request["@extra"] = serde_json::to_value(extra).unwrap_or(Value::Null);

    // 订阅响应并获取 RAII 守卫（请求超时或 Future 取消自动注销）
    let (receiver, _guard) = DISPATCHER.subscribe(extra);
    tdjson::send(client_id, &request.to_string());

    // 协程挂起，等待后台单线程抓取到响应后唤醒
    receiver.await.unwrap_or(Value::Null)
}
