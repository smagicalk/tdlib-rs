use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tdlib_rs_gen::generate_rust_code;
use tdlib_rs_parser::parse_tl_file;
use tdlib_rs_parser::tl::Definition;

/// 编译期内嵌客户端主库源码模板（包含对外核心 API 与消息接收分发循环）
static LIB_FILE: &str = include_str!("../temp/lib.rs");

/// 编译期内嵌基于 @extra 关联 ID 的异步响应监听器模板
static OBSERVER_FILE: &str = include_str!("../temp/observer.rs");

/// 编译期内嵌底层 TDLib JSON 接口 C FFI 封装模板
static TDJSON_FILE: &str = include_str!("../temp/tdjson.rs");

/// 编译期内嵌生成目标工程的 Cargo.toml 依赖配置模板
static CARGO_FILE: &str = include_str!("../temp/Cargo.toml.txt");

/// 编译期内嵌生成目标工程的 build.rs 动态库链接配置模板
static BUILD_FILE: &str = include_str!("../temp/build.rs");

/// 读取并解析指定路径的 TDLib TL (Type Language) 协议文件
///
/// # 参数
/// * `file_path` - TL 文件所在路径
///
/// # 返回
/// 成功则返回过滤掉非法条目后的有效 Definition 集合
fn load_tl(file_path: impl AsRef<Path>) -> std::io::Result<Vec<Definition>> {
    let file_path = file_path.as_ref();
    // 打印加载日志
    println!("正在加载 TL 协议定义文件: {:?}", file_path);

    // 打开 TL 定义文件
    let mut file = File::open(file_path)?;
    // 分配字符串缓冲区并读取文件全部内容
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    println!("读取完成，总字符长度: {}，开始解析 AST...", contents.len());

    // 调用 tdlib-rs-parser 的流式解析器，过滤解析错误条目并保留有效定义
    let definitions: Vec<Definition> = parse_tl_file(contents)
        .filter_map(|d| match d {
            Ok(d) => Some(d),
            Err(e) => {
                // 打印解析失败的具体错误信息（如遇到未支持的语法规则）
                eprintln!("TL: parse error: {e:?}");
                None
            }
        })
        .collect();

    println!("TL 协议解析完成，共获取 {} 条有效定义", definitions.len());
    Ok(definitions)
}

/// 主入口：解析 TL 协议，将生成的 Rust 数据类型/枚举/接口及骨架模板写入目标工程
fn main() -> std::io::Result<()> {
    // 获取可选的命令行参数（参数 1 为 TL 路径，参数 2 为输出工程目录）
    let args: Vec<String> = std::env::args().collect();

    // 确定 TL 定义文件的输入路径：优先使用命令行参数，若未提供则尝试当前工作目录或硬编码默认路径
    let tl_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        let local_tl = Path::new("tdlib-rs/tl/api.tl");
        if local_tl.exists() {
            local_tl.to_path_buf()
        } else {
            PathBuf::from(r"F:\code\rust\tdlib-rs\tdlib-rs\tl\api.tl")
        }
    };

    // 确定生成工程的目标输出目录
    let out_dir = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from(r"F:\code\rust\tdlib_test2")
    };

    println!(">>> 代码生成开始");
    println!("输入 TL 路径 : {:?}", tl_path);
    println!("输出目标目录 : {:?}", out_dir);

    // 1. 加载并解析 TL 协议，提取所有结构体、枚举与函数 AST 定义
    let definitions = load_tl(&tl_path)?;

    // 2. 构造目标工程的源码目录路径 (out_dir/src)
    let src_path = out_dir.join("src");

    // 3. 调用代码生成引擎，在 src/ 目录下生成 types/, enums/, functions/ 模块
    // 参数 true 表示同时生成 Bot 专用的 API 接口
    println!("正在生成模块化 Rust 源码（types, enums, functions）...");
    generate_rust_code(&src_path, &definitions, true)?;

    // 4. 将预设的核心模板文件拷贝写入生成工程中
    println!("正在写入项目工程骨架文件...");
    // 写入 src/lib.rs（客户端主逻辑与模块入口）
    save(src_path.join("lib.rs"), LIB_FILE)?;
    // 写入 src/observer.rs（异步响应分发器）
    save(src_path.join("observer.rs"), OBSERVER_FILE)?;
    // 写入 src/tdjson.rs（C FFI 桥接接口）
    save(src_path.join("tdjson.rs"), TDJSON_FILE)?;
    // 写入根目录 Cargo.toml（包依赖配置）
    save(out_dir.join("Cargo.toml"), CARGO_FILE)?;
    // 写入根目录 build.rs（链接 tdjson 动态库配置）
    save(out_dir.join("build.rs"), BUILD_FILE)?;

    println!(">>> 客户端工程生成完成！目标目录: {:?}", out_dir);
    Ok(())
}

/// 将文本内容完整保存至指定文件（覆盖已有文件并截断）
fn save(file_path: impl AsRef<Path>, contents: &str) -> std::io::Result<()> {
    let file_path = file_path.as_ref();
    // 若父目录不存在则自动创建父目录
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // 原子写入全部字节流并刷新到磁盘
    std::fs::write(file_path, contents.as_bytes())?;
    Ok(())
}


