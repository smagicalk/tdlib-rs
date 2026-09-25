# TDLib-rs 🦀

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-2021%20%2F%202024-orange.svg)](https://www.rust-lang.org/)
[![TDLib](https://img.shields.io/badge/TDLib-1.8.x%2B-blue.svg)](https://core.telegram.org/tdlib)

现代化、强类型、全异步的 **Telegram TDLib (Telegram Database Library)** Rust 客户端与代码生成器全套方案。

本项目不仅提供底层 TDLib JSON C-FFI 的安全封装，还搭载了基于 `clap` 的**专业级代码生成器 CLI**，支持从本地 TL 文件或直接自 Telegram 官方 GitHub 仓库在线拉取任意版本的 `td_api.tl`，一键生成模块化、高性能、多账号隔离的 Rust 客户端 Crate。

---

## 🌟 核心特性与架构升级

* **🚀 现代专业代码生成器 CLI**：
  * 基于 `clap v4` 构建，支持参数补全、协议语法预检（`inspect`）、远程协议下载（`fetch`）与一键生成（`generate`）。
  * 内置默认官方协议 fallback，即使在空目录下也能 **0 外部依赖直接生成**。
  * 支持输入 TDLib 版本号（如 `-v 1.8.67`），CLI 自动在线拉取 GitHub 官方协议并完成代码生成。
* **📁 智能领域切分（Domain Grouping）**：
  * 告别旧版生成 3,200 多个琐碎文件导致 IDE 假死与编译器慢吞吞的问题。
  * 自动将 3,200+ 条 TL 协议按 15 个高内聚业务领域切分（`chat`, `message`, `user`, `call`, `auth`, `bot`, `media` 等），全工程仅 50 余个语义文件，代码可读性与编译速度倍增。
* **🛡️ 彻底防御循环递归死锁（E0072 免疫）**：
  * 枚举变体负载统一采用 `Box<T>` 封装，并自动生成便捷构造函数（如 `OptionValue::string(...)`）与 `From` trait。
  * 从结构上对任意深度、任意循环嵌套的 TL 结构天然免疫，后续升级任意 TDLib 版本绝不产生 `error[E0072: recursive type has infinite size]`。
* **⚡ 全异步非阻塞与原生多账号并发隔离**：
  * **后台 OS 线程驱动**：事件监听循环运行在专用 OS 守护线程中，不占用 Tokio Worker 线程，彻底消除异步死锁。
  * **多账号无锁解复用**：`Dispatcher` 依照 `client_id` 精准分发推送更新（Update），每个客户端实例拥有独立私有通道，多账号并发运行绝不串号、绝不抢消息。
  * **RAII 内存安全**：异步请求携带自动取消守卫，Future 超时或中断时自动注销关联路由，杜绝内存泄漏。
* **📑 100% 完备的 Rustdoc 文档注释**：
  * 自动提取 TL 协议中的 `//@class` 顶层文档、`//@description` 及参数说明。
  * 自动标注 `# Arguments` 与 `# Returns`，代码补全与跳转体验极佳。

---

## 📦 工作区结构 (Workspace Layout)

```text
tdlib-rs/
├── tdlib-rs/               <-- 现代代码生成器 CLI 二进制工程 (可 cargo install)
│   ├── src/main.rs         <-- CLI 命令解析与执行流控制
│   ├── templates/          <-- 1:1 镜像目标工程的微型骨架系统 (Cargo.toml, build.rs, lib.rs...)
│   └── tl/                 <-- 本地附带的官方 TL 协议定义 (api.tl)
├── tdlib-rs-gen/           <-- 核心代码生成引擎库 (领域分类、Rustifier、AST 转换)
└── tdlib-rs-parser/        <-- 通用 Type Language (TL) 词法/语法解析器
```

---

## 🛠️ CLI 工具使用指南

你可以直接在当前工作区使用 `cargo run -p tdlib-rs`，也可以通过 `cargo install --path tdlib-rs` 安装到全局使用。

### 1. 查看帮助信息
```powershell
cargo run -p tdlib-rs -- --help
cargo run -p tdlib-rs -- generate --help
```

### 2. 常用操作场景

#### 场景 A：使用内置协议生成客户端工程
```powershell
# 简写模式：直接传入输出路径
cargo run -p tdlib-rs -- F:\code\rust\my_tdlib_client

# 完整命令（带 --clean 清空已有目录）：
cargo run -p tdlib-rs -- generate F:\code\rust\my_tdlib_client --clean
```

#### 场景 B：从 GitHub 在线拉取指定 TDLib 版本一键生成
```powershell
# 自动从官方 GitHub 下载 1.8.67 版 td_api.tl 并生成工程
cargo run -p tdlib-rs -- generate F:\code\rust\my_tdlib_client -v 1.8.67 --clean
```

#### 场景 C：使用指定的本地 TL 文件生成
```powershell
cargo run -p tdlib-rs -- generate F:\code\rust\my_tdlib_client -t ./path/to/custom_api.tl
```

#### 场景 D：协议结构深度分析 (`inspect`)
无需写入任何文件，快速洞察 TL 文件的结构组成：
```powershell
cargo run -p tdlib-rs -- inspect tdlib-rs/tl/api.tl
```
*输出示例：*
```text
============================================================
              📊 TL 协议结构深度分析报告                   
============================================================
协议文件  : "tdlib-rs/tl/api.tl"
有效定义  : 3211 条
结构体/类 : 2189 个
抽象类/枚举: 748 个
API 函数  : 1022 个
------------------------------------------------------------
主要枚举类变体数 TOP 10：
  - Update                       : 189 个变体
  - MessageContent               : 105 个变体
  - InternalLinkType             : 57 个变体
  - ChatEventAction              : 53 个变体
  - PushMessageContent           : 46 个变体
  - StarTransactionType          : 45 个变体
  - LinkPreviewType              : 40 个变体
  - PageBlock                    : 36 个变体
  - RichText                     : 30 个变体
  - PremiumFeature               : 29 个变体
============================================================
```

#### 场景 E：仅在线下载协议文件 (`fetch`)
```powershell
cargo run -p tdlib-rs -- fetch -v 1.8.67 -o ./api_1.8.67.tl
```

---

## 💻 生成客户端使用指南 (Generated Crate Quickstart)

生成的工程是一个独立的 Rust Crate，开箱即可投入生产。

### 1. 环境准备 (动态库配置)
运行前请确保 Telegram TDLib 动态链接库（Windows 上为 `tdjson.dll`，Linux 上为 `libtdjson.so`，macOS 上为 `libtdjson.dylib`）位于系统动态库搜索路径中。

**Windows PowerShell 设置示例：**
```powershell
$env:PATH = "F:\tdlib\td\tdlib\bin;" + $env:PATH
```

### 2. 单客户端极简示例 (`examples/simple.rs`)

```rust
use std::time::Duration;
use tdlib_rs::enums::OptionValue;
use tdlib_rs::{functions, Client};

#[tokio::main]
async fn main() {
    println!(">>> 启动 TDLib 单客户端快速起步示例");

    // 1. 同步执行底层本地命令：设置日志冗余等级
    tdlib_rs::execute(r#"{"@type":"setLogVerbosityLevel","new_verbosity_level":1}"#);

    // 2. 同步查询 TDLib DLL 动态库版本 (免建客户端即可秒级响应)
    if let Some(res) = tdlib_rs::execute(r#"{"@type":"getOption","name":"version"}"#) {
        println!("TDLib DLL 版本 (通过 td_execute 获取): {res}");
    }

    // 3. 创建现代面向对象 Client 实例
    let mut client = Client::new();
    println!("成功创建客户端实例，分配 client_id: {}", client.id());

    // 4. 异步请求获取核心版本号与 Commit Hash
    if let Ok(OptionValue::String(val)) = functions::get_option("version".into(), client.id()).await {
        println!("TDLib DLL 核心版本号 (通过 get_option 异步请求获取): {}", val.value);
    }

    // 5. 启动异步协程监听并处理服务端推送的更新事件
    let handle = tokio::spawn(async move {
        while let Some(update) = client.receive().await {
            println!("[收到更新]: {:?}", update);
        }
    });

    tokio::time::sleep(Duration::from_millis(2000)).await;
    handle.abort();
}
```

### 3. 多账号原生并发隔离示例 (`examples/multi_client.rs`)

```rust
use tdlib_rs::Client;

#[tokio::main]
async fn main() {
    // 实例化两个完全不同的账号客户端
    let mut client_a = Client::new();
    let mut client_b = Client::new();

    let id_a = client_a.id();
    let id_b = client_b.id();

    // 协程 1: 专职监听账号 A 的事件流
    tokio::spawn(async move {
        while let Some(update) = client_a.receive().await {
            println!("[账号 A - ID {id_a} 收到专属消息]: {:?}", update);
        }
    });

    // 协程 2: 专职监听账号 B 的事件流 (两者完全并行，互不干扰)
    tokio::spawn(async move {
        while let Some(update) = client_b.receive().await {
            println!("[账号 B - ID {id_b} 收到专属消息]: {:?}", update);
        }
    });
}
```

---

## 🔄 TDLib 版本升级指南

如果未来 TDLib 发布了新版本（例如 `1.8.68`、`1.9.0`），你**不需要修改生成器的任何源码**：

1. **直接联网升级**：
   ```powershell
   cargo run -p tdlib-rs -- generate ./tdlib_client -v 1.9.0 --clean
   ```
2. **或下载自定义 TL 本地升级**：
   ```powershell
   cargo run -p tdlib-rs -- generate ./tdlib_client -t ./new_td_api.tl --clean
   ```
通用生成架构会自动完成 AST 解析、领域划分（未知前缀自动兜底进入 `misc` 模块）、枚举防死锁统一封装，生成即可编译通过！

---

## 🧪 测试套件验证

```powershell
# 1. 运行生成器核心与解析器全套单元测试 (全部 42 项测试)
cargo test --workspace

# 2. 对生成的工程进行全套测试
$env:PATH = "<tdjson_dir>;" + $env:PATH
cargo test --manifest-path ./my_tdlib_client/Cargo.toml
```

---

## 📄 开源许可证 (License)

本项目遵循以下双重许可协议，你可以自由选择任一协议使用：
* [MIT License](LICENSE-MIT)
* [Apache License, Version 2.0](LICENSE-APACHE)
