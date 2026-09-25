// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! 生成 TDLib 联合类型枚举（Enums）
//!
//! 具备以下核心特性：
//! 1. 按业务领域（Domain）切分文件（约 15 个文件）；
//! 2. 所有携带结构体载荷的变体统一使用 `Box<T>` 封装，消除认知负担；
//! 3. 自动生成便捷构造函数（如 `ChatMemberStatus::creator(...)`）与 `impl From<T> for Enum`；
//! 4. 自动注入顶层 `//@class` 文档注释与规范 Rustdoc；
//! 5. 在 `mod.rs` 中完整 re-export，保证 100% 导入路径兼容。

use crate::domain;
use crate::ignore_type;
use crate::metadata::Metadata;
use crate::rustifier;
use convert_case::{Case, Casing};
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tdlib_rs_parser::tl::{Category, Definition, Type};

/// 生成 `enums` 模块目录，写入 `enums/mod.rs` 并按领域分组写入各业务域枚举文件
pub(crate) fn write_enums_mod(
    enums_mod_dir: &Path,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 收集所有去重后的联合类型
    let mut seen = HashSet::new();
    let enums: Vec<&Type> = definitions
        .iter()
        .filter(|d| d.category == Category::Types && !ignore_type(&d.ty))
        .map(|d| &d.ty)
        .filter(|ty| seen.insert(&ty.name))
        .collect();

    // 按领域分组存储枚举定义
    let mut domain_enums: BTreeMap<&'static str, Vec<&Type>> = BTreeMap::new();
    for &ty in &enums {
        let enum_name = rustifier::types::type_name(ty);
        let dom = domain::classify(&enum_name);
        domain_enums.entry(dom).or_default().push(ty);
    }

    // 构造 enums/mod.rs
    let enums_mod_path = enums_mod_dir.join("mod.rs");
    let enums_mod_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&enums_mod_path)?;
    let mut enums_mod_writer = BufWriter::new(enums_mod_file);

    writeln!(enums_mod_writer, "//!")?;
    writeln!(
        enums_mod_writer,
        "//! TDLib union enums, categorized by business domain."
    )?;
    writeln!(enums_mod_writer, "//!")?;

    // 遍历每一个具有枚举的领域，生成对应的 domain.rs 文件
    for (dom, types_in_domain) in domain_enums {
        // 在 enums/mod.rs 中注册子模块并导出全部项
        writeln!(enums_mod_writer, "pub mod {dom};")?;
        writeln!(enums_mod_writer, "pub use {dom}::*;")?;
        writeln!(enums_mod_writer)?;

        let domain_file_path = enums_mod_dir.join(dom).with_extension("rs");
        let domain_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&domain_file_path)?;
        let mut writer = BufWriter::new(domain_file);

        // 写入领域文件顶部说明与依赖引用
        writeln!(writer, "//!")?;
        writeln!(writer, "//! TDLib `{dom}` domain enums.")?;
        writeln!(writer, "//!")?;
        writeln!(writer, "//! {}", domain::domain_description(dom))?;
        writeln!(writer, "//!")?;
        writeln!(writer)?;
        writeln!(writer, "#[allow(clippy::all)]")?;
        writeln!(writer, "use serde::{{Deserialize, Serialize}};")?;
        writeln!(writer)?;

        for ty in types_in_domain {
            write_single_enum(&mut writer, ty, metadata, gen_bots_only_api)?;
        }

        writer.flush()?;
    }

    enums_mod_writer.flush()?;
    Ok(())
}

/// 将单个联合类型输出为一个完整的 Rust enum（含统一 Box、构造函数与 From 实现）
fn write_single_enum(
    writer: &mut BufWriter<File>,
    ty: &Type,
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    let enum_name = rustifier::types::type_name(ty);
    let defs = metadata.defs_with_type(ty);

    // 过滤出有效的变体定义
    let valid_defs: Vec<&&Definition> = defs
        .iter()
        .filter(|d| !rustifier::definitions::is_for_bots_only(d) || gen_bots_only_api)
        .collect();

    if valid_defs.is_empty() {
        return Ok(());
    }

    // 1. 写入顶层类注释（优先采用提取的 class 文档）
    if let Some(class_desc) = metadata.class_description(&ty.name) {
        let formatted_desc = class_desc.replace('\n', "\n/// ");
        writeln!(writer, "/// {formatted_desc}")?;
    } else {
        writeln!(writer, "/// TDLib `{}` union enum.", ty.name)?;
    }

    // 2. 写入 derive 宏与多态标签
    writeln!(
        writer,
        "#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]"
    )?;
    writeln!(writer, "#[serde(tag = \"@type\")]")?;
    writeln!(writer, "pub enum {enum_name} {{")?;

    // 3. 写入各个变体
    for d in &valid_defs {
        // 变体文档注释
        writeln!(writer, "{}", rustifier::definitions::description(d, "    "))?;
        // Serde rename 注解
        writeln!(
            writer,
            "    #[serde(rename(serialize = \"{0}\", deserialize = \"{0}\"))]",
            d.name
        )?;
        let variant_name = rustifier::definitions::variant_name(d);
        write!(writer, "    {variant_name}")?;

        // 统一化规范：带数据的变体一律统一采用 Box<T> 封装
        if d.params.is_empty() {
            writeln!(writer, ",")?;
        } else {
            writeln!(
                writer,
                "(Box<{}>),",
                rustifier::definitions::qual_name(d)
            )?;
        }
    }
    writeln!(writer, "}}")?;
    writeln!(writer)?;

    // 4. 自动生成便捷构造函数（如 FormattedTextOrString::formatted_text(v)）
    let payload_defs: Vec<&&Definition> = valid_defs.iter().filter(|d| !d.params.is_empty()).copied().collect();

    if !payload_defs.is_empty() {
        writeln!(writer, "impl {enum_name} {{")?;
        for d in &payload_defs {
            let variant_name = rustifier::definitions::variant_name(d);
            let snake_name = rustifier::escape_keyword(&variant_name.to_case(Case::Snake));
            let qual_type = rustifier::definitions::qual_name(d);

            writeln!(
                writer,
                "    /// Convenience constructor to create a [`{enum_name}::{variant_name}`] variant."
            )?;
            writeln!(writer, "    ///")?;
            writeln!(writer, "    /// Automatically wraps the payload in `Box`.")?;
            writeln!(
                writer,
                "    pub fn {snake_name}(val: {qual_type}) -> Self {{"
            )?;
            writeln!(writer, "        Self::{variant_name}(Box::new(val))")?;
            writeln!(writer, "    }}")?;
            writeln!(writer)?;
        }
        writeln!(writer, "}}")?;
        writeln!(writer)?;

        // 5. 自动实现 From<T> for Enum
        for d in &payload_defs {
            let variant_name = rustifier::definitions::variant_name(d);
            let qual_type = rustifier::definitions::qual_name(d);

            writeln!(
                writer,
                "/// Converts a [`{qual_type}`] into [`{enum_name}`]."
            )?;
            writeln!(writer, "impl From<{qual_type}> for {enum_name} {{")?;
            writeln!(writer, "    fn from(val: {qual_type}) -> Self {{")?;
            writeln!(writer, "        Self::{variant_name}(Box::new(val))")?;
            writeln!(writer, "    }}")?;
            writeln!(writer, "}}")?;
            writeln!(writer)?;
        }
    }

    Ok(())
}
