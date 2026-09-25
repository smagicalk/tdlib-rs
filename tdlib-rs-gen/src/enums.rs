// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to generate Rust's `enum`'s from TL definitions.

use crate::ignore_type;
use crate::metadata::Metadata;
use crate::rustifier;
use convert_case::{Case, Casing};
use std::collections::HashSet;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tdlib_rs_parser::tl::{Category, Definition, Type};

/// 将单个 TL 联合类型（抽象基类）转换为 Rust `enum` 并写入对应的独立文件
///
/// 生成形如：
/// ```ignore
/// #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
/// #[serde(tag = "@type")]
/// pub enum ChatMemberStatus {
///     #[serde(rename(serialize = "chatMemberStatusCreator", deserialize = "chatMemberStatusCreator"))]
///     Creator(crate::types::ChatMemberStatusCreator),
///     ...
/// }
/// ```
fn write_enum(
    enums_dir: &Path,
    ty: &Type,
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 获取该枚举在 Rust 中的大驼峰类型名称（例如 "ChatMemberStatus"）
    let enum_name = rustifier::types::type_name(ty);
    // 将大驼峰转换为 snake_case 文件名（例如 "chat_member_status"）
    let file_name = enum_name.to_case(Case::Snake);
    // 构造生成的单独枚举文件完整路径（如 "enums/chat_member_status.rs"）
    let enum_path = enums_dir.join(&file_name).with_extension("rs");

    // 打开或创建目标文件，截断已有内容
    let enum_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&enum_path)?;
    // 使用 BufWriter 进行缓冲写，减少磁盘 I/O 开销
    let mut enum_writer = BufWriter::new(enum_file);

    // 添加抑制 Clippy 警告属性宏
    writeln!(enum_writer, "#[allow(clippy::all)]")?;
    // 导入 Serde 序列化与反序列化 Trait
    writeln!(enum_writer, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(enum_writer)?;

    // 派生常规的 Clone, Debug, PartialEq 以及 Serde 宏
    writeln!(
        enum_writer,
        "#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]",
    )?;
    // Telegram TDLib 通过 JSON 中的 "@type" 字段区分具体的枚举变体（多态分发）
    writeln!(enum_writer, "#[serde(tag = \"@type\")]")?;
    // 输出枚举声明头（如 "pub enum ChatMemberStatus {"）
    writeln!(enum_writer, "pub enum {} {{", enum_name)?;

    // 遍历该联合类型下的所有子定义（即此 enum 的各个变体）
    for d in metadata.defs_with_type(ty) {
        // 如果该变体仅用于 Bot 且未开启 Bot API 生成，则跳过
        if rustifier::definitions::is_for_bots_only(d) && !gen_bots_only_api {
            continue;
        }

        // 写入当前变体的文档注释
        writeln!(
            enum_writer,
            "{}",
            rustifier::definitions::description(d, "    ")
        )?;
        // 使用 serde rename 映射 TDLib 的 TL 原始类型名（例如 "chatMemberStatusCreator"）
        writeln!(
            enum_writer,
            "    #[serde(rename(serialize = \"{0}\", deserialize = \"{0}\"))]",
            d.name
        )?;
        // 写入变体名称（大驼峰）
        write!(enum_writer, "    {}", rustifier::definitions::variant_name(d))?;

        // 若该定义没有任何参数，则作为无数据的纯标识变体（例如 Variant,）
        if d.params.is_empty() {
            writeln!(enum_writer, ",")?;
            continue;
        } else {
            write!(enum_writer, "(")?;
        }

        // 判断该定义是否存在递归嵌套，若有循环依赖则使用 Box 包装以打破无限大小
        let is_recursive = metadata.is_recursive_def(d);
        if is_recursive {
            write!(enum_writer, "Box<")?;
        }
        // 写入该变体包裹的结构体完整限定路径（如 "crate::types::ChatMemberStatusCreator"）
        write!(enum_writer, "{}", rustifier::definitions::qual_name(d))?;
        if is_recursive {
            write!(enum_writer, ">")?;
        }

        // 闭合元组变体括号并换行
        writeln!(enum_writer, "),")?;
    }

    // 闭合枚举定义大括号
    writeln!(enum_writer, "}}")?;
    // 刷新缓冲区
    enum_writer.flush()?;
    Ok(())
}

/// 生成 `enums` 模块目录，写入 `enums/mod.rs` 并为每个联合类型创建对应的独立枚举文件
pub(crate) fn write_enums_mod(
    enums_mod_dir: &Path,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 构造 enums/mod.rs 文件路径
    let enums_mod_path = enums_mod_dir.join("mod.rs");
    // 创建或截断 enums/mod.rs
    let enums_mod_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&enums_mod_path)?;
    // 使用 BufWriter 包装写入流
    let mut enums_mod_writer = BufWriter::new(enums_mod_file);

    // 收集所有联合类型，并利用 HashSet 进行精确去重（保持出现顺序且避免非连续重复）
    let mut seen = HashSet::new();
    let enums: Vec<&Type> = definitions
        .iter()
        .filter(|d| d.category == Category::Types && !ignore_type(&d.ty))
        .map(|d| &d.ty)
        .filter(|ty| seen.insert(&ty.name))
        .collect();

    // 遍历每一个去重后的枚举类型
    for ty in enums {
        // 获取枚举的 Rust 类型名
        let enum_name = rustifier::types::type_name(ty);
        // 获取枚举的 snake_case 文件名
        let file_name = enum_name.to_case(Case::Snake);

        // 在 enums/mod.rs 中声明该子模块并公开导出
        writeln!(enums_mod_writer, "mod {};", file_name)?;
        writeln!(enums_mod_writer, "pub use {}::{};", file_name, enum_name)?;
        writeln!(enums_mod_writer)?;

        // 生成具体的枚举定义文件
        write_enum(enums_mod_dir, ty, metadata, gen_bots_only_api)?;
    }

    // 刷新 mod.rs 写入缓冲
    enums_mod_writer.flush()?;
    Ok(())
}
