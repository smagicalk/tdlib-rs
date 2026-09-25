// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! 生成 TDLib 数据结构体（Types）
//!
//! 具备以下核心特性：
//! 1. 按业务领域（Domain）切分文件（约 15 个文件）；
//! 2. 字段类型安全、正确标记 Option 与 serde_as；
//! 3. 严格匹配 TDLib 的 JSON wire 格式，保持字段重命名（如 `r#type` -> `"type"`, `is_self` -> `"self"`）；
//! 4. 自动生成完备的 Rustdoc 结构体与字段级文档注释；
//! 5. 在 `mod.rs` 中完整 re-export，保证 100% 导入路径兼容。

use crate::domain;
use crate::ignore_type;
use crate::metadata::Metadata;
use crate::rustifier;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tdlib_rs_parser::tl::{Category, Definition};

/// 生成 `types` 模块目录，写入 `types/mod.rs` 并按领域分组写入各业务域结构体文件
pub(crate) fn write_types_mod(
    types_mod_dir: &Path,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 筛选所有类型定义
    let types: Vec<&Definition> = definitions
        .iter()
        .filter(|d| d.category == Category::Types && !ignore_type(&d.ty))
        .collect();

    // 按领域分组存储类型定义
    let mut domain_types: BTreeMap<&'static str, Vec<&Definition>> = BTreeMap::new();
    for &def in &types {
        let type_name = rustifier::definitions::type_name(def);
        let dom = domain::classify(&type_name);
        domain_types.entry(dom).or_default().push(def);
    }

    // 构造 types/mod.rs
    let types_mod_path = types_mod_dir.join("mod.rs");
    let types_mod_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&types_mod_path)?;
    let mut types_mod_writer = BufWriter::new(types_mod_file);

    writeln!(types_mod_writer, "//!")?;
    writeln!(
        types_mod_writer,
        "//! TDLib data structs, categorized by business domain."
    )?;
    writeln!(types_mod_writer, "//!")?;

    // 遍历每一个具有类型的领域，生成对应的 domain.rs 文件
    for (dom, defs_in_domain) in domain_types {
        // 在 types/mod.rs 中注册子模块并导出全部项
        writeln!(types_mod_writer, "pub mod {dom};")?;
        writeln!(types_mod_writer, "pub use {dom}::*;")?;
        writeln!(types_mod_writer)?;

        let domain_file_path = types_mod_dir.join(dom).with_extension("rs");
        let domain_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&domain_file_path)?;
        let mut writer = BufWriter::new(domain_file);

        // 写入领域文件顶部说明与依赖引用
        writeln!(writer, "//!")?;
        writeln!(writer, "//! TDLib `{dom}` domain types.")?;
        writeln!(writer, "//!")?;
        writeln!(writer, "//! {}", domain::domain_description(dom))?;
        writeln!(writer, "//!")?;
        writeln!(writer)?;
        writeln!(writer, "#[allow(clippy::all)]")?;
        writeln!(writer, "use serde::{{Deserialize, Serialize}};")?;
        writeln!(writer, "use serde_with::{{serde_as, DisplayFromStr}};")?;
        writeln!(writer)?;

        for def in defs_in_domain {
            write_single_struct(&mut writer, def, metadata, gen_bots_only_api)?;
        }

        writer.flush()?;
    }

    types_mod_writer.flush()?;
    Ok(())
}

/// 将单个数据类型输出为一个完整的 Rust struct
fn write_single_struct(
    writer: &mut BufWriter<File>,
    def: &Definition,
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    if rustifier::definitions::is_for_bots_only(def) && !gen_bots_only_api {
        return Ok(());
    }

    let type_name = rustifier::definitions::type_name(def);

    // 1. 输出该类型的文档注释（源自 TL 注释）
    writeln!(writer, "{}", rustifier::definitions::description(def, ""))?;

    // 2. 检查是否有任何字段需要使用 serde_as 属性转换
    let has_serde_as = def
        .params
        .iter()
        .any(|p| rustifier::parameters::serde_as(p).is_some());

    if has_serde_as {
        writeln!(writer, "#[serde_as]")?;
    }

    // 3. 派生 Traits
    write!(writer, "#[derive(Clone, Debug, ")?;
    if metadata.can_def_implement_default(def) {
        write!(writer, "Default, ")?;
    }
    writeln!(writer, "PartialEq, Deserialize, Serialize)]")?;

    // 4. 声明结构体头
    writeln!(writer, "pub struct {type_name} {{")?;

    // 5. 遍历结构体字段
    for param in &def.params {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        // 写入字段文档注释
        writeln!(writer, "{}", rustifier::parameters::description(param, "    "))?;

        // serde_as 注解
        if let Some(serde_as) = rustifier::parameters::serde_as(param) {
            writeln!(writer, "    #[serde_as(as = \"{serde_as}\")]")?;
        }

        let attr_name = rustifier::parameters::attr_name(param);
        // 如果字段被重命名（如 r#type 或 is_self），添加明确的 serde rename 保证 wire 兼容
        if attr_name.starts_with("r#") || attr_name != param.name {
            writeln!(writer, "    #[serde(rename = \"{}\")]", param.name)?;
        }

        write!(writer, "    pub {attr_name}: ")?;

        let is_optional = rustifier::parameters::is_optional(param);
        if is_optional {
            write!(writer, "Option<")?;
        }

        // 检查直接自引用字段
        let is_direct_self = param.ty.name == def.name;
        if is_direct_self {
            write!(writer, "Box<")?;
        }

        write!(writer, "{}", rustifier::parameters::qual_name(param))?;

        if is_direct_self {
            write!(writer, ">")?;
        }
        if is_optional {
            write!(writer, ">")?;
        }

        writeln!(writer, ",")?;
    }

    writeln!(writer, "}}")?;
    writeln!(writer)?;
    Ok(())
}
