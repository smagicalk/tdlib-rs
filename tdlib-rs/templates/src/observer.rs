// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use futures_channel::{mpsc, oneshot};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// 异步请求取消守卫：利用 RAII 机制，当 Future 因超时（如 timeout）或被取消而 Drop 时，
/// 自动从 Dispatcher 请求表中注销对应的 extra，彻底根治内存泄漏。
pub(crate) struct RequestGuard {
    extra: u32,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        crate::DISPATCHER.unsubscribe(self.extra);
    }
}

/// 事件解复用中心（Dispatcher）
///
/// 1. 负责将带 `@extra` 的响应路由回对应的异步请求调用者（oneshot）；
/// 2. 负责将服务端推送的不带 `@extra` 的更新（Update），精准投递给指定 `@client_id` 的客户端私有队列。
pub(crate) struct Dispatcher {
    /// 存储每个 `@extra` 请求标识对应的 oneshot 发送端
    requests: Mutex<HashMap<u32, oneshot::Sender<Value>>>,
    /// 存储每个 `client_id` 对应的推送更新通道发送端
    clients: Mutex<HashMap<i32, mpsc::UnboundedSender<crate::enums::Update>>>,
}

impl Dispatcher {
    /// 创建新的事件解复用中心实例
    pub fn new() -> Self {
        Self {
            requests: Mutex::default(),
            clients: Mutex::default(),
        }
    }

    /// 订阅指定 extra 标识的请求响应，同时返回 RAII 取消守卫
    pub fn subscribe(&self, extra: u32) -> (oneshot::Receiver<Value>, RequestGuard) {
        let (sender, receiver) = oneshot::channel();
        if let Ok(mut requests) = self.requests.lock() {
            requests.insert(extra, sender);
        }
        (receiver, RequestGuard { extra })
    }

    /// 注销指定 extra 请求（内部或 Guard 触发）
    pub fn unsubscribe(&self, extra: u32) {
        if let Ok(mut requests) = self.requests.lock() {
            requests.remove(&extra);
        }
    }

    /// 为指定客户端注册专属的更新接收通道
    pub fn register_client(&self, client_id: i32) -> mpsc::UnboundedReceiver<crate::enums::Update> {
        let (tx, rx) = mpsc::unbounded();
        if let Ok(mut clients) = self.clients.lock() {
            clients.insert(client_id, tx);
        }
        rx
    }

    /// 注销指定客户端的所有接收通道（客户端 Drop 时触发）
    pub fn unregister_client(&self, client_id: i32) {
        if let Ok(mut clients) = self.clients.lock() {
            clients.remove(&client_id);
        }
    }

    /// 处理底层全局事件循环收到的单条 JSON 响应/更新，完成无锁解复用与分发
    pub fn dispatch(&self, response: Value) {
        // 1. 优先检查是否带有 "@extra" 字段（API 请求的响应）
        if let Some(extra_val) = response.get("@extra") {
            let extra = extra_val.as_u64().map(|v| v as u32);
            if let Some(extra) = extra {
                let sender = if let Ok(mut requests) = self.requests.lock() {
                    requests.remove(&extra)
                } else {
                    None
                };

                if let Some(sender) = sender {
                    let _ = sender.send(response);
                } else {
                    log::debug!("Got a response for an unrequested or cancelled extra: {extra}");
                }
                return;
            }
        }

        // 2. 无 "@extra" 则为推送更新（Push Update）
        let client_id = response
            .get("@client_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        match serde_json::from_value::<crate::enums::Update>(response) {
            Ok(update) => {
                let sender = if let Ok(clients) = self.clients.lock() {
                    clients.get(&client_id).cloned()
                } else {
                    None
                };

                if let Some(sender) = sender {
                    let _ = sender.unbounded_send(update);
                } else {
                    log::trace!("No active listener for client_id {client_id}");
                }
            }
            Err(e) => {
                log::warn!("Failed to deserialize TDLib update for client_id {client_id}: {e}");
            }
        }
    }
}
