// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! 该模块作为代码生成的总协调调度中心，负责管理各代码生成子模块并为其分发数据。
mod domain;
mod enums;
mod functions;
mod metadata;
mod rustifier;
mod types;

use std::io;
use std::path::Path;
use tdlib_rs_parser::tl::{Definition, Type};

/// 不需要为其单独生成结构体/枚举的特殊核心基础类型
/// 这些类型在 Rust 中直接映射为原生类型或由框架内置处理（例如 Bool -> bool, Int32 -> i32, Ok -> () 等）
const SPECIAL_CASED_TYPES: [&str; 6] = ["Bool", "Bytes", "Int32", "Int53", "Int64", "Ok"];

/// 判断给定类型是否属于需要跳过的特殊核心基础类型
fn ignore_type(ty: &Type) -> bool {
    // 检查类型名称是否匹配特殊类型列表中的任意一项
    SPECIAL_CASED_TYPES.iter().any(|&x| x == ty.name)
}

/// 根据解析后的 TL 定义列表生成完整的模块化 Rust 代码
///
/// # 参数
/// * `rust_project_path` - 目标工程源码根目录（通常为生成目标项目的 `src/` 目录）
/// * `definitions` - TL 协议解析后获得的所有类型与方法定义
/// * `gen_bots_only_api` - 是否生成仅供 Bot 使用的专用 API 接口
pub fn generate_rust_code(
    rust_project_path: impl AsRef<Path>,
    definitions: &[Definition],
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 获取路径引用，支持传入任意实现了 AsRef<Path> 的类型（如 PathBuf, &Path, &str 等）
    let rust_project_path = rust_project_path.as_ref();

    // 如果目标路径已存在且是单个普通文件，则先将其删除以避免目录创建冲突
    if rust_project_path.is_file() {
        std::fs::remove_file(rust_project_path)?;
    }

    // 确保目标根目录及其所有上级父目录存在
    if !rust_project_path.exists() {
        std::fs::create_dir_all(rust_project_path)?;
    }

    // 从所有定义构建元数据（用于处理循环引用、派生 Default、收集联合类型成员等）
    let metadata = metadata::Metadata::new(definitions);

    // 1. 创建 types 模块子目录并生成所有数据结构体代码
    let types_mod_dir = rust_project_path.join("types");
    // 确保 types/ 目录存在
    std::fs::create_dir_all(&types_mod_dir)?;
    // 生成所有结构体并导出到 types/mod.rs
    types::write_types_mod(&types_mod_dir, definitions, &metadata, gen_bots_only_api)?;

    // 2. 创建 enums 模块子目录并生成所有联合枚举代码
    let enums_mod_dir = rust_project_path.join("enums");
    // 确保 enums/ 目录存在
    std::fs::create_dir_all(&enums_mod_dir)?;
    // 生成所有联合类型枚举并导出到 enums/mod.rs
    enums::write_enums_mod(&enums_mod_dir, definitions, &metadata, gen_bots_only_api)?;

    // 3. 创建 functions 模块子目录并生成所有客户端异步请求方法代码
    let functions_mod_dir = rust_project_path.join("functions");
    // 确保 functions/ 目录存在
    std::fs::create_dir_all(&functions_mod_dir)?;
    // 生成所有 TDLib API 请求方法并导出到 functions/mod.rs
    functions::write_functions_mod(&functions_mod_dir, definitions, &metadata, gen_bots_only_api)?;

    // 全部模块生成完成，返回成功
    Ok(())
}