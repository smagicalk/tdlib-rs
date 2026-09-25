use std::io::{self, Write};
use tdlib_rs::enums::{AuthorizationState, OptionValue, Update};
use tdlib_rs::{functions, Client};

fn ask_input(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer);
    buffer.trim().to_string()
}

#[tokio::main]
async fn main() {
    println!("========================================");
    println!("      TDLib 交互式登录与 GetMe 示例     ");
    println!("========================================");

    // 1. 设置 TDLib 日志级别
    tdlib_rs::execute(r#"{"@type":"setLogVerbosityLevel","new_verbosity_level":1}"#);

    // 2. 同步获取已加载 TDLib DLL 动态库版本号
    if let Some(res) = tdlib_rs::execute(r#"{"@type":"getOption","name":"version"}"#) {
        println!("TDLib DLL 版本 (通过 td_execute 同步获取): {res}");
    }

    // 3. 从环境变量或控制台获取 API_ID 和 API_HASH (https://my.telegram.org)
    let api_id: i32 = std::env::var("API_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            let input = ask_input("请输入 Telegram API_ID: ");
            input.parse().unwrap_or(0)
        });

    let api_hash: String = std::env::var("API_HASH").unwrap_or_else(|_| {
        ask_input("请输入 Telegram API_HASH: ")
    });

    if api_id == 0 || api_hash.is_empty() {
        eprintln!("错误: 必须提供有效的 API_ID 与 API_HASH 才能连接 Telegram！");
        return;
    }

    // 4. 实例化现代全异步 Client
    let mut client = Client::new();
    let client_id = client.id();
    println!("客户端实例创建成功，client_id = {client_id}");

    // 5. 异步请求获取 TDLib DLL 核心版本号与 commit_hash
    match functions::get_option("version".into(), client_id).await {
        Ok(OptionValue::String(val)) => {
            println!("TDLib DLL 核心版本号 (通过 get_option 异步请求获取): {}", val.value);
        }
        Ok(other) => println!("TDLib DLL 版本信息: {:?}", other),
        Err(e) => eprintln!("异步获取 TDLib 版本失败: {}: {}", e.code, e.message),
    }

    if let Ok(OptionValue::String(commit)) = functions::get_option("commit_hash".into(), client_id).await {
        println!("TDLib DLL Commit Hash: {}", commit.value);
    }

    // 6. 事件驱动认证循环
    while let Some(update) = client.receive().await {
        if let Update::AuthorizationState(auth_update) = update {
            match &auth_update.authorization_state {
                AuthorizationState::WaitTdlibParameters => {
                    println!("[1/4] 配置 TDLib 核心数据库参数...");
                    let res = functions::set_tdlib_parameters(
                        false,
                        "tdlib_session_db".into(),
                        String::new(),
                        String::new(),
                        true,
                        true,
                        true,
                        false,
                        api_id,
                        api_hash.clone(),
                        "zh-cn".into(),
                        "Desktop".into(),
                        "Windows".into(),
                        env!("CARGO_PKG_VERSION").into(),
                        client_id,
                    )
                    .await;

                    if let Err(e) = res {
                        eprintln!("设置 TDLib 参数失败: {}: {}", e.code, e.message);
                    }
                }
                AuthorizationState::WaitPhoneNumber => {
                    println!("[2/4] 需要手机号进行登录验证。");
                    let phone = ask_input("请输入国际区号手机号 (例如 +8613800000000): ");
                    let res = functions::set_authentication_phone_number(
                        phone,
                        None,
                        client_id,
                    )
                    .await;

                    if let Err(e) = res {
                        eprintln!("发送手机号失败: {}: {}", e.code, e.message);
                    }
                }
                AuthorizationState::WaitCode(_) => {
                    println!("[3/4] 验证码已发送至你的 Telegram 客户端或短信。");
                    let code = ask_input("请输入收到的验证码: ");
                    let res = functions::check_authentication_code(code, client_id).await;

                    if let Err(e) = res {
                        eprintln!("验证码校验失败: {}: {}", e.code, e.message);
                    }
                }
                AuthorizationState::WaitPassword(_) => {
                    println!("[3.5/4] 该账号启用了两步验证密码。");
                    let password = ask_input("请输入两步验证密码: ");
                    let res = functions::check_authentication_password(password, client_id).await;

                    if let Err(e) = res {
                        eprintln!("密码校验失败: {}: {}", e.code, e.message);
                    }
                }
                AuthorizationState::Ready => {
                    println!("[4/4] 登录成功！正在查询当前登录用户信息 (get_me)...");
                    match functions::get_me(client_id).await {
                        Ok(user_enum) => {
                            println!("================ 个人资料 ================");
                            println!("{:#?}", user_enum);
                            println!("==========================================");
                        }
                        Err(e) => {
                            eprintln!("获取个人信息失败: {}: {}", e.code, e.message);
                        }
                    }
                    println!("GetMe 演示结束，退出程序。");
                    break;
                }
                AuthorizationState::Closed => {
                    println!("TDLib 客户端已关闭。");
                    break;
                }
                other => {
                    println!("当前认证状态: {:?}", other);
                }
            }
        }
    }
}
