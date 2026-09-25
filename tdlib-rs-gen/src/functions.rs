// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code to generate Rust's `fn`'s from TL definitions.

use crate::metadata::Metadata;
use crate::rustifier;
use convert_case::{Case, Casing};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tdlib_rs_parser::tl::{Category, Definition};

/// 将单个 TL 函数定义转换为 Rust 异步函数并写入独立的源码文件，并在 functions/mod.rs 中注册与重导出
///
/// 生成形如：
/// ```ignore
/// pub async fn get_me(client_id: i32) -> Result<crate::types::User, crate::types::Error> {
///     let request = json!({ "@type": "getMe" });
///     let response = send_request(client_id, request).await;
///     ...
/// }
/// ```
fn write_function(
    function_mod: &mut BufWriter<File>,
    function_dir: &Path,
    def: &Definition,
    _metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 若当前函数属于 Bot 专用且未开启 Bot API 生成，则直接跳过
    if rustifier::definitions::is_for_bots_only(def) && !gen_bots_only_api {
        return Ok(());
    }

    // 获取函数的 Rust 大驼峰/规范名称
    let function_name = rustifier::definitions::function_name(def);
    // 转换为 snake_case 风格作为文件名
    let file_name = function_name.to_case(Case::Snake);

    // 在 functions/mod.rs 中注册子模块并公开导出
    writeln!(function_mod, "mod {};", file_name)?;
    writeln!(function_mod, "pub use {}::{};", file_name, function_name)?;
    writeln!(function_mod)?;

    // 构造独立的函数源码文件路径（如 "functions/get_me.rs"）
    let function_path = function_dir.join(&file_name).with_extension("rs");
    // 创建或截断目标文件
    let function_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&function_path)?;
    // 使用 BufWriter 包装写入流
    let mut function_writer = BufWriter::new(function_file);

    // 写入 Clippy 警告抑制宏
    writeln!(function_writer, "#[allow(clippy::all)]")?;
    // 引入 serde_json 用于组装 JSON 动态请求对象
    writeln!(function_writer, "use serde_json::json;")?;
    // 引入底层发送请求异步函数 send_request
    writeln!(function_writer, "use crate::send_request;")?;
    writeln!(function_writer)?;

    // 输出该函数的完整文档注释
    writeln!(function_writer, "{}", rustifier::definitions::description(def, ""))?;
    writeln!(function_writer, "/// # Arguments")?;
    // 遍历生成每个入参的文档注释
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            function_writer,
            "/// * `{}` - {}",
            rustifier::parameters::attr_name(param),
            param.description.replace('\n', "\n/// ")
        )?;
    }
    // 添加用于通信的目标 client_id 参数说明
    writeln!(
        function_writer,
        "/// * `client_id` - The client id to send the request to"
    )?;

    // 标记抑制过多参数告警（部分 TDLib 接口包含十几个参数）
    writeln!(function_writer, "#[allow(clippy::too_many_arguments)]")?;
    // 输出函数签名起始部分（例如 "pub async fn get_me("）
    write!(
        function_writer,
        "pub async fn {}(",
        rustifier::definitions::function_name(def)
    )?;

    // 遍历输出各个形参及其类型
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        // 输出参数名及冒号
        write!(function_writer, "{}: ", rustifier::parameters::attr_name(param))?;

        // 判断该参数是否可选
        let is_optional = rustifier::parameters::is_optional(param);
        if is_optional {
            write!(function_writer, "Option<")?;
        }
        // 输出参数类型的完整限定名
        write!(function_writer, "{}", rustifier::parameters::qual_name(param))?;
        if is_optional {
            write!(function_writer, ">")?;
        }

        // 参数间用逗号和空格分隔
        write!(function_writer, ", ")?;
    }

    // 每个请求函数必须携带 client_id 参数，并返回 Result<返回类型, Error>
    writeln!(
        function_writer,
        "client_id: i32) -> Result<{}, crate::types::Error> {{",
        rustifier::types::qual_name(&def.ty, false)
    )?;

    // 组装 JSON 请求体：以 @type 标明调用的接口名称
    writeln!(function_writer, "    let request = json!({{")?;
    writeln!(function_writer, "        \"@type\": \"{}\",", def.name)?;
    // 将 Rust 入参映射为 JSON 字段
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            function_writer,
            "        \"{0}\": {1},",
            param.name,
            rustifier::parameters::attr_name(param),
        )?;
    }
    writeln!(function_writer, "    }});")?;

    // 发起异步请求并等待底层响应返回
    writeln!(
        function_writer,
        "    let response = send_request(client_id, request).await;"
    )?;
    // 检查响应中的 @type 是否为 "error"，若是则反序列化为 Error 类型返回 Err
    writeln!(function_writer, "    if response[\"@type\"] == \"error\" {{")?;
    writeln!(
        function_writer,
        "        return Err(serde_json::from_value(response).unwrap());"
    )?;
    writeln!(function_writer, "    }}")?;

    // 如果返回类型是 Ok（无实际数据 payload），则返回 Ok(())
    if rustifier::types::is_ok(&def.ty) {
        writeln!(function_writer, "    Ok(())")?;
    } else {
        // 否则将 JSON 响应体反序列化为对应的具体数据类型并返回
        writeln!(
            function_writer,
            "    Ok(serde_json::from_value(response).unwrap())"
        )?;
    }

    // 闭合函数体大括号
    writeln!(function_writer, "}}")?;
    // 刷新缓冲区
    function_writer.flush()?;
    Ok(())
}

/// 生成 `functions` 模块目录，写入 `functions/mod.rs` 并为每个 API 生成对应的请求函数
pub(crate) fn write_functions_mod(
    function_dir: &Path,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // 构造 functions/mod.rs 文件路径
    let function_mod_path = function_dir.join("mod.rs");
    // 创建或截断 functions/mod.rs
    let function_mod_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&function_mod_path)?;
    // 使用 BufWriter 包装写入流
    let mut function_mod_writer = BufWriter::new(function_mod_file);

    // 筛选出属于 Category::Functions 的所有方法定义
    let functions = definitions
        .iter()
        .filter(|d| d.category == Category::Functions);

    // 逐个生成函数文件并在 functions/mod.rs 中导出
    for definition in functions {
        write_function(
            &mut function_mod_writer,
            function_dir,
            definition,
            metadata,
            gen_bots_only_api,
        )?;
    }

    // 刷新 mod.rs 写入缓冲
    function_mod_writer.flush()?;
    Ok(())
}
