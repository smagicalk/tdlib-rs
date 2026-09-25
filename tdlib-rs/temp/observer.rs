// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use futures_channel::oneshot;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

/// 异步响应分发观察者：基于 oneshot 通道实现请求与响应的多线程异步匹配
pub(super) struct Observer {
    /// 存储每个 `@extra` 请求标识对应的 oneshot 发送端
    /// 使用读写锁保证跨线程并发注册与通知的安全
    requests: RwLock<HashMap<u32, oneshot::Sender<Value>>>,
}

impl Observer {
    /// 创建新的观察者实例
    pub fn new() -> Self {
        Observer {
            // 初始化空的请求哈希表读写锁
            requests: RwLock::default(),
        }
    }

    /// 订阅指定 extra 标识的请求响应
    ///
    /// # 参数
    /// * `extra` - 请求分配的唯一序号
    ///
    /// # 返回
    /// 用于异步 await 结果的 oneshot::Receiver
    pub fn subscribe(&self, extra: u32) -> oneshot::Receiver<Value> {
        // 创建一对 oneshot 通道 (发送端, 接收端)
        let (sender, receiver) = oneshot::channel();
        // 获取写锁并将发送端存入哈希表中，键为 extra
        self.requests.write().unwrap().insert(extra, sender);
        // 返回接收端给调用者进行异步等待
        receiver
    }

    /// 当 receive() 收到带 @extra 的响应时，触发通知唤醒对应异步请求
    ///
    /// # 参数
    /// * `response` - 从 TDLib 获取到的完整 JSON 响应对象
    pub fn notify(&self, response: Value) {
        // 从响应中提取 @extra 标识符并转换为 u32
        let extra = response["@extra"].as_u64().unwrap() as u32;

        // 获取写锁并将该 extra 对应的发送端从表中取出（请求一次性消费）
        match self.requests.write().unwrap().remove(&extra) {
            Some(sender) => {
                // 将响应数据通过 oneshot 通道发回等待的协程
                if sender.send(response).is_err() {
                    // 若接收端已被提前丢弃（例如调用方超时取消），记录警告日志
                    log::warn!("Got a response of an unaccessible request");
                }
            }
            None => {
                // 如果未找到对应的 extra，可能该请求超时已被清理或来自未知源
                log::warn!("Got a response of an unknown request");
            }
        }
    }
}
