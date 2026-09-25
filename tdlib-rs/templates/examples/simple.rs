use std::time::Duration;
use tdlib_rs::enums::OptionValue;
use tdlib_rs::{functions, Client};

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("       TDLib 单客户端快速起步示例       ");
    println!("========================================");

    // 1. 同步执行底层本地命令：设置日志冗余等级为 1 (仅致命错误与常规警告，减少终端杂乱输出)
    let log_request = r#"{"@type":"setLogVerbosityLevel","new_verbosity_level":1}"#;
    if let Some(res) = tdlib_rs::execute(log_request) {
        println!("设置 TDLib 日志等级成功: {res}");
    }

    // 2. 同步请求获取 TDLib 动态库版本号 (无需创建 Client 即可获取)
    if let Some(res) = tdlib_rs::execute(r#"{"@type":"getOption","name":"version"}"#) {
        println!("TDLib DLL 版本 (通过 td_execute 同步获取): {res}");
    }

    // 3. 创建现代面向对象 Client 实例
    let mut client = Client::new();
    println!("成功创建客户端实例，分配 client_id: {}", client.id());

    // 4. 异步请求获取 TDLib DLL 版本号及 commit_hash
    match functions::get_option("version".into(), client.id()).await {
        Ok(OptionValue::String(val)) => {
            println!("TDLib DLL 核心版本号 (通过 get_option 异步请求获取): {}", val.value);
        }
        Ok(other) => println!("TDLib DLL 版本信息: {:?}", other),
        Err(e) => eprintln!("异步获取 TDLib 版本失败: {}: {}", e.code, e.message),
    }

    if let Ok(OptionValue::String(commit)) =
        functions::get_option("commit_hash".into(), client.id()).await
    {
        println!("TDLib DLL Commit Hash: {}", commit.value);
    }

    // 5. 启动异步协程监听并处理服务端推送的更新事件
    let handle = tokio::spawn(async move {
        println!("后台更新事件监听任务已启动...");
        let mut count = 0;
        while let Some(update) = client.receive().await {
            println!("[收到更新 #{count}]: {:?}", update);
            count += 1;
            if count >= 2 {
                println!("已收到足够示例更新，退出监听协程。");
                break;
            }
        }
    });

    // 保持主任务运行短暂时间以便观察输出
    tokio::time::sleep(Duration::from_millis(1500)).await;
    handle.abort();
    println!(">>> 单客户端示例运行完毕！");
}
