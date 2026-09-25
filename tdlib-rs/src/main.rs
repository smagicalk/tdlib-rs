use clap::{Args, Parser, Subcommand};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tdlib_rs_gen::generate_rust_code;
use tdlib_rs_parser::parse_tl_file;
use tdlib_rs_parser::tl::{Category, Definition};

/// 编译期内嵌客户端主库源码模板（包含对外核心 API 与消息接收分发循环）
static LIB_FILE: &str = include_str!("../templates/src/lib.rs");

/// 编译期内嵌基于 @extra 关联 ID 的异步响应监听器模板
static OBSERVER_FILE: &str = include_str!("../templates/src/observer.rs");

/// 编译期内嵌底层 TDLib JSON 接口 C FFI 封装模板
static TDJSON_FILE: &str = include_str!("../templates/src/tdjson.rs");

/// 编译期内嵌生成目标工程的 Cargo.toml 依赖配置模板
static CARGO_FILE: &str = include_str!("../templates/Cargo.toml.template");

/// 编译期内嵌生成目标工程的 build.rs 动态库链接配置模板
static BUILD_FILE: &str = include_str!("../templates/build.rs");

/// 编译期内嵌单客户端极简示例
static EXAMPLE_SIMPLE: &str = include_str!("../templates/examples/simple.rs");

/// 编译期内嵌原生多客户端并发隔离示例
static EXAMPLE_MULTI_CLIENT: &str = include_str!("../templates/examples/multi_client.rs");

/// 编译期内嵌交互式登录与 GetMe 示例
static EXAMPLE_GET_ME: &str = include_str!("../templates/examples/get_me.rs");

/// 编译期内嵌默认官方 TDLib TL 协议定义（零依赖 fallback）
static DEFAULT_TL: &str = include_str!("../tl/api.tl");

#[derive(Parser, Debug)]
#[command(
    name = "tdlib-rs",
    author = "Federico Bruzzone, Andrea Longoni",
    version = "1.3.0",
    about = "🚀 现代化 Telegram TDLib Rust 客户端强类型代码生成器",
    long_about = "现代化 Telegram TDLib Rust 客户端强类型代码生成器。\n\
                  支持一键解析本地或官方远程 TL 协议，自动化业务领域分类 (Domain Grouping)，\n\
                  并生成高内聚、全异步、多账号隔离的 Rust 客户端 Crate。"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// 快捷输出目录（当未显式指定子命令时作为 generate 目标目录使用）
    #[arg(value_name = "OUTPUT_DIR")]
    quick_output: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 解析 TL 协议并生成完整的异步 TDLib 客户端工程
    Generate(GenerateArgs),
    /// 从 Telegram 官方 GitHub 仓库在线下载指定版本的 td_api.tl
    Fetch(FetchArgs),
    /// 分析指定的 TL 协议定义文件并打印统计报告（不生成文件）
    Inspect(InspectArgs),
}

#[derive(Args, Debug)]
struct GenerateArgs {
    /// 生成的目标工程输出目录
    #[arg(value_name = "OUTPUT_DIR")]
    output: PathBuf,

    /// 本地 TL 协议文件路径（若未指定且未指定版本，则自动使用内嵌的官方 TL 协议）
    #[arg(short, long, value_name = "TL_PATH")]
    tl: Option<PathBuf>,

    /// 在线拉取指定 TDLib 版本的官方协议进行生成（如 "1.8.67", "master"）
    #[arg(short = 'v', long = "version-tag", value_name = "TAG")]
    version_tag: Option<String>,

    /// 自定义生成的 Crate 包名（默认为 "tdlib-rs"）
    #[arg(long, value_name = "NAME")]
    pkg_name: Option<String>,

    /// 自定义生成的 Crate 版本号（默认为 "1.3.0"）
    #[arg(long, value_name = "VERSION")]
    pkg_version: Option<String>,

    /// 仅生成 Telegram Bot 专用的 API 接口
    #[arg(long)]
    bots_only: bool,

    /// 若输出目录已存在，先清空其内容
    #[arg(long)]
    clean: bool,

    /// 仅解析协议并模拟生成，不向磁盘写入任何文件
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Debug)]
struct FetchArgs {
    /// TDLib 版本标签或 Git 分支（如 "1.8.67", "v1.8.62", "master"）
    #[arg(short = 'v', long = "version-tag", value_name = "TAG", default_value = "master")]
    version_tag: String,

    /// 本地保存路径（默认保存至 "./api_<TAG>.tl"）
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct InspectArgs {
    /// 待分析的本地 TL 协议文件路径（留空则分析内嵌默认 TL 协议）
    #[arg(value_name = "TL_PATH")]
    tl: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.command {
        Some(Commands::Generate(args)) => run_generate(args),
        Some(Commands::Fetch(args)) => run_fetch(args),
        Some(Commands::Inspect(args)) => run_inspect(args),
        None => {
            if let Some(out_dir) = cli.quick_output {
                run_generate(GenerateArgs {
                    output: out_dir,
                    tl: None,
                    version_tag: None,
                    pkg_name: None,
                    pkg_version: None,
                    bots_only: false,
                    clean: false,
                    dry_run: false,
                })
            } else {
                eprintln!("错误: 请指定输出目录或使用 --help 查看完整帮助信息。");
                std::process::exit(1);
            }
        }
    };

    if let Err(e) = res {
        eprintln!("\n❌ 执行失败: {e}");
        std::process::exit(1);
    }
}

/// 执行代码生成
fn run_generate(args: GenerateArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("      TDLib-rs 现代强类型客户端代码生成器 v1.3.0            ");
    println!("============================================================");

    // 1. 获取 TL 内容
    let (tl_content, tl_source_desc) = if let Some(tag) = &args.version_tag {
        println!("[1/4] 🌐 正在从 GitHub 官方仓库在线拉取 TDLib Tag: {tag} ...");
        let content = fetch_remote_tl(tag)?;
        (content, format!("GitHub 官方远程 Tag ({tag})"))
    } else if let Some(local_path) = &args.tl {
        println!("[1/4] 📂 正在读取本地 TL 协议定义文件: {:?} ...", local_path);
        let mut file = File::open(local_path)
            .map_err(|e| format!("无法打开本地 TL 文件 {local_path:?}: {e}"))?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        (content, format!("本地文件 ({local_path:?})"))
    } else {
        let local_fallback = Path::new("tdlib-rs/tl/api.tl");
        if local_fallback.exists() {
            println!("[1/4] 📂 自动检测到本地仓库协议: {:?} ...", local_fallback);
            let mut file = File::open(local_fallback)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            (content, format!("本地默认文件 ({local_fallback:?})"))
        } else {
            println!("[1/4] 📦 未指定输入文件，采用二进制内嵌的官方协议定义 (零依赖模式) ...");
            (DEFAULT_TL.to_string(), "内嵌官方默认 TL 协议".into())
        }
    };

    // 2. 解析 AST
    println!("[2/4] 🔍 正在解析 TL 协议 AST (源: {tl_source_desc}) ...");
    let definitions: Vec<Definition> = parse_tl_file(tl_content)
        .filter_map(|d| match d {
            Ok(d) => Some(d),
            Err(e) => {
                log::debug!("跳过未解析的 TL 条目: {e:?}");
                None
            }
        })
        .collect();

    let def_count = definitions.len();
    if def_count == 0 {
        return Err("TL 协议文件中未解析出任何有效定义，请检查文件格式！".into());
    }
    println!("    -> 成功解析得到 {def_count} 条有效定义！");

    if args.dry_run {
        println!("\n🔍 [Dry-Run 模式] 协议验证成功，跳过实际文件写入。");
        return Ok(());
    }

    // 3. 处理输出目录
    let out_dir = &args.output;
    if args.clean && out_dir.exists() {
        println!("    -> 正在清空已存在的目标目录: {:?}", out_dir);
        std::fs::remove_dir_all(out_dir)?;
    }

    let src_path = out_dir.join("src");
    std::fs::create_dir_all(&src_path)?;

    // 4. 调用代码生成引擎
    println!("[3/4] ⚙️  正在生成领域模块化 Rust 源码（types, enums, functions）...");
    generate_rust_code(&src_path, &definitions, !args.bots_only)?;

    // 5. 写入骨架与模板文件
    println!("[4/4] 📦 正在装配工程模板骨架与现代化异步示例...");

    // 自定义 Cargo.toml
    let mut cargo_toml = CARGO_FILE.to_string();
    if let Some(pkg_name) = &args.pkg_name {
        cargo_toml = cargo_toml.replace("name = \"tdlib-rs\"", &format!("name = \"{pkg_name}\""));
    }
    if let Some(pkg_ver) = &args.pkg_version {
        cargo_toml = cargo_toml.replace("version = \"1.3.0\"", &format!("version = \"{pkg_ver}\""));
    }

    save(out_dir.join("Cargo.toml"), &cargo_toml)?;
    save(out_dir.join("build.rs"), BUILD_FILE)?;
    save(src_path.join("lib.rs"), LIB_FILE)?;
    save(src_path.join("observer.rs"), OBSERVER_FILE)?;
    save(src_path.join("tdjson.rs"), TDJSON_FILE)?;

    // 写入 examples/
    let examples_dir = out_dir.join("examples");
    save(examples_dir.join("simple.rs"), EXAMPLE_SIMPLE)?;
    save(examples_dir.join("multi_client.rs"), EXAMPLE_MULTI_CLIENT)?;
    save(examples_dir.join("get_me.rs"), EXAMPLE_GET_ME)?;

    // 6. 输出完成摘要
    println!("\n============================================================");
    println!("           🎉 TDLib 客户端工程生成成功！                   ");
    println!("============================================================");
    println!("目标工程目录: {:?}", out_dir);
    println!("协议来源    : {}", tl_source_desc);
    println!("有效定义总数: {} 条", def_count);
    println!("生成架构    : 15 个业务领域模块 | 统一 Box 变体 | 全异步多账号隔离");
    println!("包含示例    : simple.rs, multi_client.rs, get_me.rs");
    println!("------------------------------------------------------------");
    println!("快速上手：");
    println!("  1. 进入目录: cd {:?}", out_dir);
    println!("  2. 链接 DLL: $env:PATH = \"<tdjson.dll所在目录>;\" + $env:PATH");
    println!("  3. 运行测试: cargo test");
    println!("  4. 运行示例: cargo run --example simple");
    println!("============================================================\n");

    Ok(())
}

/// 在线拉取指定版本的 TL 协议文件并保存
fn run_fetch(args: FetchArgs) -> Result<(), Box<dyn std::error::Error>> {
    let tag = &args.version_tag;
    println!("🌐 正在从 GitHub 官方仓库拉取 TDLib Tag: {tag} ...");
    let content = fetch_remote_tl(tag)?;

    let target_path = args.output.unwrap_or_else(|| {
        let clean_tag = tag.replace('/', "_");
        PathBuf::from(format!("api_{clean_tag}.tl"))
    });

    save(&target_path, &content)?;
    println!("✅ 协议已保存至本地: {:?}", target_path);
    println!("文件大小: {} 字符", content.len());
    Ok(())
}

/// 分析 TL 文件统计详情
fn run_inspect(args: InspectArgs) -> Result<(), Box<dyn std::error::Error>> {
    let (content, name) = if let Some(path) = &args.tl {
        println!("🔍 正在分析本地 TL 协议: {:?} ...", path);
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        (content, format!("{:?}", path))
    } else {
        println!("🔍 正在分析内嵌默认 TL 协议 ...");
        (DEFAULT_TL.to_string(), "内嵌官方 TL 协议".into())
    };

    let definitions: Vec<Definition> = parse_tl_file(content)
        .filter_map(|d| d.ok())
        .collect();

    let mut type_count = 0;
    let mut func_count = 0;
    let mut class_map: HashMap<String, usize> = HashMap::new();

    for def in &definitions {
        match def.category {
            Category::Types => {
                type_count += 1;
                let class_name = def.ty.name.clone();
                *class_map.entry(class_name).or_insert(0) += 1;
            }
            Category::Functions => {
                func_count += 1;
            }
        }
    }

    println!("\n============================================================");
    println!("              📊 TL 协议结构深度分析报告                   ");
    println!("============================================================");
    println!("协议文件  : {}", name);
    println!("有效定义  : {} 条", definitions.len());
    println!("结构体/类 : {} 个", type_count);
    println!("抽象类/枚举: {} 个", class_map.len());
    println!("API 函数  : {} 个", func_count);
    println!("------------------------------------------------------------");
    println!("主要枚举类变体数 TOP 10：");
    let mut top_classes: Vec<(&String, &usize)> = class_map.iter().collect();
    top_classes.sort_by(|a, b| b.1.cmp(a.1));
    for (cls, count) in top_classes.iter().take(10) {
        println!("  - {:<28} : {} 个变体", cls, count);
    }
    println!("============================================================\n");

    Ok(())
}

/// 从 GitHub 官方仓库下载指定版本的 td_api.tl
fn fetch_remote_tl(tag: &str) -> Result<String, Box<dyn std::error::Error>> {
    let clean_tag = if tag.starts_with('v') || tag == "master" {
        tag.to_string()
    } else {
        format!("v{tag}")
    };

    let urls = [
        format!("https://raw.githubusercontent.com/tdlib/td/{clean_tag}/td/generate/scheme/td_api.tl"),
        format!("https://raw.githubusercontent.com/tdlib/td/{tag}/td/generate/scheme/td_api.tl"),
    ];

    for url in &urls {
        println!("    -> 尝试请求: {url} ...");
        match ureq::get(url).call() {
            Ok(resp) => {
                let mut reader = resp.into_body().into_reader();
                let mut body = String::new();
                reader.read_to_string(&mut body)?;
                if !body.is_empty() {
                    return Ok(body);
                }
            }
            Err(e) => {
                log::debug!("请求 {url} 失败: {e}");
            }
        }
    }

    Err(format!("无法从 GitHub 下载版本为 '{tag}' 的 td_api.tl，请检查版本是否存在或检查网络环境。").into())
}

/// 保存文本至指定路径
fn save(file_path: impl AsRef<Path>, contents: &str) -> std::io::Result<()> {
    let file_path = file_path.as_ref();
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut file = File::create(file_path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}
