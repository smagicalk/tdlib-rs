
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 优先从环境变量 LOCAL_TDLIB_PATH 获取本地编译好的 TDLib 动态库路径
    // 若未设置该环境变量，则回退使用默认的本地路径
    let dir = env::var("LOCAL_TDLIB_PATH").unwrap_or_else(|_| r"F:\tdlib\td\tdlib".to_string());

    // 1. 指定 TDLib C 头文件目录（供可能的底层 C/C++ 依赖使用）
    println!("cargo:include={dir}\\include");

    // 2. 指定 Windows/Linux/macOS 下动态链接库的搜索路径（bin 目录存放 dll/dylib/so）
    println!("cargo:rustc-link-search=native={dir}\\bin");
    // 3. 指定 lib 目录搜索路径（存放 .lib 导入库文件）
    println!("cargo:rustc-link-search=native={dir}\\lib");

    // 4. 指示 Rust 链接器动态链接 tdjson 动态库（在 Windows 下链接 tdjson.lib/tdjson.dll，在 Linux 下链接 libtdjson.so）
    println!("cargo:rustc-link-lib=dylib=tdjson");

    // 5. 设置运行时动态链接查找路径 rpath（Unix 类平台生效）
    println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}\\bin");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/bin");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");

    // 6. 告诉 Cargo 仅当 build.rs 本身或相关环境变量变化时重新执行构建脚本
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOCAL_TDLIB_PATH");
}

/// 辅助检测当前运行的 Linux 发行版是否为 Debian
#[allow(dead_code)]
fn is_debian() -> bool {
    // 读取系统 release 信息文件
    match fs::read_to_string("/etc/os-release") {
        Ok(content) => content.contains("ID=debian") || content.contains("Debian"),
        Err(_) => false,
    }
}

/// 辅助检测当前运行的 Linux 发行版是否为 Ubuntu
#[allow(dead_code)]
fn is_ubuntu() -> bool {
    // 读取系统 release 信息文件
    match fs::read_to_string("/etc/os-release") {
        Ok(content) => content.contains("ID=ubuntu") || content.contains("Ubuntu"),
        Err(_) => false,
    }
}

