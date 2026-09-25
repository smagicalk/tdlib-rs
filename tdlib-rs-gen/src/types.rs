// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to generate Rust's `struct`'s from TL definitions.

use crate::ignore_type;
use crate::metadata::Metadata;
use crate::rustifier;
use convert_case::{Case, Casing};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tdlib_rs_parser::tl::{Category, Definition};

/// 将单个 TL 类型定义写入独立的 Rust 结构体文件，并在 types/mod.rs 中注册模块和重导出
///
/// # 参数
/// * `types_mod` - 指向 types/mod.rs 的缓冲写入器
/// * `types_dir` - types 模块所在的目录路径
/// * `def` - 当前要处理的 TL 结构体定义
/// * `metadata` - 元数据分析器（用于判断是否可派生 Default 等）
/// * `gen_bots_only_api` - 是否生成仅供 Bot 使用的专用 API
fn write_struct(
    types_mod: &mut BufWriter<File>,
    types_dir: &Path,
    def: &Definition,
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 若当前定义属于 Bot 专用且未开启生成 Bot API 开关，则直接跳过
    if rustifier::definitions::is_for_bots_only(def) && !gen_bots_only_api {
        return Ok(());
    }

    // 获取 Rust 格式的类型大驼峰名称（例如 "FormattedText"）
    let type_name = rustifier::definitions::type_name(def);
    // 将大驼峰名称转换为 snake_case 文件名（例如 "formatted_text"）
    let file_name = type_name.to_case(Case::Snake);

    // 在 types/mod.rs 中声明子模块并公开重导出该结构体
    writeln!(types_mod, "mod {};", file_name)?;
    writeln!(types_mod, "pub use {}::{};", file_name, type_name)?;
    writeln!(types_mod)?;

    // 构造独立的类型文件路径（如 "types/formatted_text.rs"）
    let type_path = types_dir.join(&file_name).with_extension("rs");
    // 打开/创建目标文件（截断已有内容），并通过 BufWriter 缓冲写入以提高 I/O 效率
    let type_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&type_path)?;
    let mut type_writer = BufWriter::new(type_file);

    // 生成 Clippy 警告抑制宏（TDLib 生成的字段较多且命名特殊）
    writeln!(type_writer, "#[allow(clippy::all)]")?;
    // 引入序列化与反序列化 trait
    writeln!(type_writer, "use serde::{{Deserialize, Serialize}};")?;
    // 引入 serde_with 扩展（用于支持字符串与整数等特殊转换）
    writeln!(type_writer, "use serde_with::{{serde_as, DisplayFromStr}};")?;
    writeln!(type_writer)?;

    // 输出该类型的文档注释（源自 TL 注释）
    writeln!(type_writer, "{}", rustifier::definitions::description(def, ""))?;

    // 检查是否有任何字段需要使用 serde_as 属性转换（如 int64/int53 跨平台数字安全处理）
    let has_serde_as = def
        .params
        .iter()
        .any(|p| rustifier::parameters::serde_as(p).is_some());

    // 如果存在需要特殊转换的字段，添加 #[serde_as] 宏
    if has_serde_as {
        writeln!(type_writer, "#[serde_as]")?;
    }

    // 开始生成 derive 派生宏
    write!(type_writer, "#[derive(Clone, Debug, ")?;
    // 若所有字段均为简单类型且可构建默认值，则自动为其派生 Default
    if metadata.can_def_implement_default(def) {
        write!(type_writer, "Default, ")?;
    }
    // 派生常规比较与序列化 Trait
    writeln!(type_writer, "PartialEq, Deserialize, Serialize)]")?;

    // 声明结构体定义头（如 "pub struct FormattedText {"）
    writeln!(
        type_writer,
        "pub struct {} {{",
        rustifier::definitions::type_name(def),
    )?;

    // 遍历结构体的各个字段参数
    for param in def.params.iter() {
        // 如果字段属于 Bot 专用且未开启 Bot API 生成，则跳过该字段
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        // 写入字段的文档注释，并保持缩进
        writeln!(
            type_writer,
            "{}",
            rustifier::parameters::description(param, "    ")
        )?;

        // 若字段有特殊序列化处理器（例如 i64 序列化为 string），添加对应 serde_as 注解
        if let Some(serde_as) = rustifier::parameters::serde_as(param) {
            writeln!(type_writer, "    #[serde_as(as = \"{serde_as}\")]")?;
        }

        // 写入字段修饰符及字段名（例如 "    pub text: "）
        write!(
            type_writer,
            "    pub {}: ",
            rustifier::parameters::attr_name(param),
        )?;

        // 判断该字段在 TL 协议中是否为可选（Nullable）
        let is_optional = rustifier::parameters::is_optional(param);
        // 如果是可选类型，外层包裹 Option<...>
        if is_optional {
            write!(type_writer, "Option<")?;
        }
        // 写入 Rust 类型完整限定名（如 String, i32, Vec<...> 等）
        write!(type_writer, "{}", rustifier::parameters::qual_name(param))?;
        // 补全 Option 的闭合尖括号
        if is_optional {
            write!(type_writer, ">")?;
        }

        // 字段声明以逗号换行结尾
        writeln!(type_writer, ",")?;
    }

    // 闭合结构体大括号
    writeln!(type_writer, "}}")?;
    // 刷新缓冲区确保完全写入磁盘
    type_writer.flush()?;
    Ok(())
}

/// 遍历所有定义并生成整个 `types` 模块及各个独立的结构体文件
pub(crate) fn write_types_mod(
    types_mod_dir: &Path,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 构造 types/mod.rs 文件路径
    let types_mod_path = types_mod_dir.join("mod.rs");
    // 创建或截断 types/mod.rs 文件
    let types_mod_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&types_mod_path)?;
    // 使用 BufWriter 包装以提高批量写入效率
    let mut types_mod_writer = BufWriter::new(types_mod_file);

    // 过滤出所有属于结构体类型（Category::Types）、非内置忽略类型且含有参数字段的 TL 定义
    let types = definitions
        .iter()
        .filter(|d| d.category == Category::Types && !ignore_type(&d.ty) && !d.params.is_empty());

    // 逐个生成结构体文件并在 types/mod.rs 中注册
    for definition in types {
        write_struct(
            &mut types_mod_writer,
            types_mod_dir,
            definition,
            metadata,
            gen_bots_only_api,
        )?;
    }

    // 刷新 mod.rs 写入缓冲
    types_mod_writer.flush()?;
    Ok(())
}
