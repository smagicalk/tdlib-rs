use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // 优先从环境变量 LOCAL_TDLIB_PATH 获取本地编译好的 TDLib 动态库路径
    // 若未设置该环境变量，则尝试默认路径
    let default_path = if cfg!(windows) {
        r"F:\tdlib\td\tdlib".to_string()
    } else {
        "/usr/local".to_string()
    };
    let dir_str = env::var("LOCAL_TDLIB_PATH").unwrap_or(default_path);
    let dir = PathBuf::from(&dir_str);

    let include_dir = dir.join("include");
    let bin_dir = dir.join("bin");
    let lib_dir = dir.join("lib");

    // 1. 指定 TDLib C 头文件目录
    if include_dir.exists() {
        println!("cargo:include={}", include_dir.display());
    }

    // 2. 指定动态链接库搜索路径（bin 目录存放 dll/dylib/so）
    if bin_dir.exists() {
        println!("cargo:rustc-link-search=native={}", bin_dir.display());
    }

    // 3. 指定 lib 目录搜索路径（存放 .lib 导入库文件或 .so 文件）
    if lib_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
    }

    // 4. 指示 Rust 链接器动态链接 tdjson
    println!("cargo:rustc-link-lib=dylib=tdjson");

    // 5. 设置运行时动态链接查找路径 rpath（Unix 类平台生效）
    if !cfg!(windows) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", bin_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/bin");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    // 6. 仅当 build.rs 本身或环境变量变化时重新执行构建脚本
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LOCAL_TDLIB_PATH");
}
