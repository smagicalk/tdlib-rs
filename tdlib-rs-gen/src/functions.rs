// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! 生成 TDLib 异步 API 请求函数（Functions）
//!
//! 具备以下核心特性：
//! 1. 按业务领域（Domain）切分文件（约 15 个文件）；
//! 2. 完备的 Rustdoc 注释，包含 `/// # Arguments` 列表与 `/// # Returns` 返回值说明；
//! 3. 严格的异步非阻塞调用，通过底层 `send_request` 与事件中心解耦；
//! 4. 在 `mod.rs` 中完整 re-export，保证 `functions::get_me(...)` 等路径 100% 兼容。

use crate::domain;
use crate::metadata::Metadata;
use crate::rustifier;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tdlib_rs_parser::tl::{Category, Definition};

/// 生成 `functions` 模块目录，写入 `functions/mod.rs` 并按领域分组写入各业务域函数文件
pub(crate) fn write_functions_mod(
    functions_mod_dir: &Path,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 筛选所有函数定义
    let functions: Vec<&Definition> = definitions
        .iter()
        .filter(|d| d.category == Category::Functions)
        .collect();

    // 按领域分组存储函数定义
    let mut domain_functions: BTreeMap<&'static str, Vec<&Definition>> = BTreeMap::new();
    for &def in &functions {
        let func_name = rustifier::definitions::function_name(def);
        let dom = domain::classify(&func_name);
        domain_functions.entry(dom).or_default().push(def);
    }

    // 构造 functions/mod.rs
    let functions_mod_path = functions_mod_dir.join("mod.rs");
    let functions_mod_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&functions_mod_path)?;
    let mut functions_mod_writer = BufWriter::new(functions_mod_file);

    writeln!(functions_mod_writer, "//!")?;
    writeln!(
        functions_mod_writer,
        "//! TDLib asynchronous API functions, categorized by business domain."
    )?;
    writeln!(functions_mod_writer, "//!")?;

    // 遍历每一个具有函数的领域，生成对应的 domain.rs 文件
    for (dom, defs_in_domain) in domain_functions {
        // 在 functions/mod.rs 中注册子模块并导出全部函数
        writeln!(functions_mod_writer, "pub mod {dom};")?;
        writeln!(functions_mod_writer, "pub use {dom}::*;")?;
        writeln!(functions_mod_writer)?;

        let domain_file_path = functions_mod_dir.join(dom).with_extension("rs");
        let domain_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&domain_file_path)?;
        let mut writer = BufWriter::new(domain_file);

        // 写入领域文件顶部说明与依赖引用
        writeln!(writer, "//!")?;
        writeln!(writer, "//! TDLib `{dom}` domain functions.")?;
        writeln!(writer, "//!")?;
        writeln!(writer, "//! {}", domain::domain_description(dom))?;
        writeln!(writer, "//!")?;
        writeln!(writer)?;
        writeln!(writer, "#[allow(clippy::all)]")?;
        writeln!(writer, "use serde_json::json;")?;
        writeln!(writer, "use crate::send_request;")?;
        writeln!(writer)?;

        for def in defs_in_domain {
            write_single_function(&mut writer, def, metadata, gen_bots_only_api)?;
        }

        writer.flush()?;
    }

    functions_mod_writer.flush()?;
    Ok(())
}

/// 将单个 API 定义输出为一个完整的 Rust 异步函数
fn write_single_function(
    writer: &mut BufWriter<File>,
    def: &Definition,
    _metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    if rustifier::definitions::is_for_bots_only(def) && !gen_bots_only_api {
        return Ok(());
    }

    let function_name = rustifier::definitions::function_name(def);
    let return_type_str = rustifier::types::qual_name(&def.ty, false);

    // 1. 函数顶层文档注释
    writeln!(writer, "{}", rustifier::definitions::description(def, ""))?;
    writeln!(writer, "///")?;
    writeln!(writer, "/// # Arguments")?;
    writeln!(writer, "///")?;

    // 遍历形参文档注释
    for param in &def.params {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            writer,
            "/// * `{}` - {}",
            rustifier::parameters::attr_name(param),
            param.description.replace('\n', "\n/// ")
        )?;
    }
    writeln!(
        writer,
        "/// * `client_id` - The numeric client identifier to send the request to."
    )?;

    // 2. 函数返回值文档注释
    writeln!(writer, "///")?;
    writeln!(writer, "/// # Returns")?;
    writeln!(writer, "///")?;
    if rustifier::types::is_ok(&def.ty) {
        writeln!(writer, "/// * `Ok(())` - On success.")?;
    } else {
        writeln!(
            writer,
            "/// * `Ok({return_type_str})` - On success."
        )?;
    }
    writeln!(
        writer,
        "/// * `Err(crate::types::Error)` - On failure, returns a TDLib error."
    )?;

    // 3. 抑制过多参数告警并生成函数签名
    writeln!(writer, "#[allow(clippy::too_many_arguments)]")?;
    write!(writer, "pub async fn {function_name}(")?;

    for param in &def.params {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        write!(writer, "{}: ", rustifier::parameters::attr_name(param))?;

        let is_optional = rustifier::parameters::is_optional(param);
        if is_optional {
            write!(writer, "Option<")?;
        }
        write!(writer, "{}", rustifier::parameters::qual_name(param))?;
        if is_optional {
            write!(writer, ">")?;
        }

        write!(writer, ", ")?;
    }

    writeln!(
        writer,
        "client_id: i32) -> Result<{return_type_str}, crate::types::Error> {{"
    )?;

    // 4. 组装 JSON 请求体
    writeln!(writer, "    let request = json!({{")?;
    writeln!(writer, "        \"@type\": \"{}\",", def.name)?;
    for param in &def.params {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            writer,
            "        \"{0}\": {1},",
            param.name,
            rustifier::parameters::attr_name(param),
        )?;
    }
    writeln!(writer, "    }});")?;

    // 5. 调用底层 send_request 并处理返回值
    writeln!(
        writer,
        "    let response = send_request(client_id, request).await;"
    )?;
    writeln!(writer, "    if response[\"@type\"] == \"error\" {{")?;
    writeln!(
        writer,
        "        return Err(serde_json::from_value(response).unwrap());"
    )?;
    writeln!(writer, "    }}")?;

    if rustifier::types::is_ok(&def.ty) {
        writeln!(writer, "    Ok(())")?;
    } else {
        writeln!(
            writer,
            "    Ok(serde_json::from_value(response).unwrap())"
        )?;
    }

    writeln!(writer, "}}")?;
    writeln!(writer)?;
    Ok(())
}
