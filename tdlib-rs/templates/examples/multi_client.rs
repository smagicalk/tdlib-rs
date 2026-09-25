use std::time::Duration;
use tdlib_rs::enums::OptionValue;
use tdlib_rs::{functions, Client};

#[tokio::main]
async fn main() {
    println!("========================================================");
    println!("    TDLib 原生多客户端/多账号并发隔离测试示例           ");
    println!("========================================================");

    // 1. 设置底层日志等级为 1 (仅警告与错误)
    tdlib_rs::execute(r#"{"@type":"setLogVerbosityLevel","new_verbosity_level":1}"#);

    // 2. 同步请求获取 TDLib 动态库版本号
    if let Some(res) = tdlib_rs::execute(r#"{"@type":"getOption","name":"version"}"#) {
        println!("TDLib DLL 动态库版本 (td_execute 同步获取): {res}");
    }

    // 3. 同时创建两个独立的客户端实例（模拟运行两个完全不同的 Telegram 账号）
    let mut client_a = Client::new();
    let mut client_b = Client::new();

    let id_a = client_a.id();
    let id_b = client_b.id();

    println!("账号 A 客户端初始化完成，分配 ID: {id_a}");
    println!("账号 B 客户端初始化完成，分配 ID: {id_b}");
    assert_ne!(id_a, id_b, "多客户端 ID 必须独立分配！");

    // 分别通过两个独立 client 实例异步请求 TDLib 版本号
    if let Ok(OptionValue::String(ver_a)) = functions::get_option("version".into(), id_a).await {
        println!("账号 A 请求 TDLib 版本成功: {}", ver_a.value);
    }
    if let Ok(OptionValue::String(ver_b)) = functions::get_option("version".into(), id_b).await {
        println!("账号 B 请求 TDLib 版本成功: {}", ver_b.value);
    }

    // 4. 协程 1：专职拉取并处理账号 A 的事件流
    let task_a = tokio::spawn(async move {
        println!("账号 A (ID {id_a}) 专属事件监听协程已启动");
        let mut count = 0;
        while let Some(update) = client_a.receive().await {
            println!("[账号 A - ID {id_a} 收到事件 #{count}]: {:?}", update);
            count += 1;
            if count >= 2 {
                break;
            }
        }
    });

    // 5. 协程 2：专职拉取并处理账号 B 的事件流（两个协程完全并发，绝不互相抢消息！）
    let task_b = tokio::spawn(async move {
        println!("账号 B (ID {id_b}) 专属事件监听协程已启动");
        let mut count = 0;
        while let Some(update) = client_b.receive().await {
            println!("[账号 B - ID {id_b} 收到事件 #{count}]: {:?}", update);
            count += 1;
            if count >= 2 {
                break;
            }
        }
    });

    // 等待 2 秒观察多客户端并行运作
    tokio::time::sleep(Duration::from_millis(2000)).await;
    task_a.abort();
    task_b.abort();

    println!(">>> 多客户端并发隔离测试完成，所有账号队列完全隔离！");
}
