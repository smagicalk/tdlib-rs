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

use std::fs::File;
use crate::metadata::Metadata;
use crate::rustifier;
use std::io::{self, Write};
use convert_case::{Case, Casing};
use tdlib_rs_parser::tl::{Category, Definition};
use crate::enums::write_enums_mod;

/// Defines the `function` corresponding to the definition:
///
/// ```ignore
/// pub async fn name(client_id: i32, field: Type) -> Result {
///
/// }
/// ```
fn write_function(
    function_mod: &mut File,
    function_dir: &mut std::path::PathBuf,
    def: &Definition,
    _metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    if rustifier::definitions::is_for_bots_only(def) && !gen_bots_only_api {
        return Ok(());
    }

    let function_name = rustifier::definitions::function_name(&def);
    let file_name = function_name.to_case(Case::Snake);

    writeln!(function_mod, "mod {};", file_name)?;
    writeln!(function_mod, "pub use {}::{};",file_name,function_name)?;
    writeln!(function_mod, "")?;
    writeln!(function_mod, "")?;


    let function_path = function_dir.join(file_name).with_extension("rs");
    let mut function_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&function_path)?;

    // Begin outermost mod
    writeln!(function_file, "#[allow(clippy::all)]")?;
    writeln!(function_file, "    use serde_json::json;")?;
    writeln!(function_file, "    use crate::send_request;")?;

    // Documentation
    writeln!(function_file, "{}", rustifier::definitions::description(def, ""))?;
    writeln!(function_file, "/// # Arguments")?;
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            function_file,
            "/// * `{}` - {}",
            rustifier::parameters::attr_name(param),
            param.description.replace('\n', "\n    /// ")
        )?;
    }
    writeln!(
        function_file,
        "/// * `client_id` - The client id to send the request to"
    )?;

    // Function
    writeln!(function_file, "#[allow(clippy::too_many_arguments)]")?;
    write!(
        function_file,
        "pub async fn {}(",
        rustifier::definitions::function_name(def)
    )?;
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        write!(function_file, "{}: ", rustifier::parameters::attr_name(param))?;

        let is_optional = rustifier::parameters::is_optional(param);
        if is_optional {
            write!(function_file, "Option<")?;
        }
        write!(function_file, "{}", rustifier::parameters::qual_name(param))?;
        if is_optional {
            write!(function_file, ">")?;
        }

        write!(function_file, ", ")?;
    }

    writeln!(
        function_file,
        "client_id: i32) -> Result<{}, crate::types::Error> {{",
        rustifier::types::qual_name(&def.ty, false)
    )?;

    // Compose request
    writeln!(function_file, "    let request = json!({{")?;
    writeln!(function_file, "        \"@type\": \"{}\",", def.name)?;
    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            function_file,
            "        \"{0}\": {1},",
            param.name,
            rustifier::parameters::attr_name(param),
        )?;
    }
    writeln!(function_file, "        }});")?;

    // Send request
    writeln!(
        function_file,
        "    let response = send_request(client_id, request).await;"
    )?;
    writeln!(function_file, "    if response[\"@type\"] == \"error\" {{")?;
    writeln!(
        function_file,
        "        return Err(serde_json::from_value(response).unwrap())"
    )?;
    writeln!(function_file, "    }}")?;

    if rustifier::types::is_ok(&def.ty) {
        writeln!(function_file, "    Ok(())")?;
    } else {
        writeln!(
            function_file,
            "    Ok(serde_json::from_value(response).unwrap())"
        )?;
    }

    writeln!(function_file, "}}")?;
    Ok(())
}


/// Write the entire module dedicated to functions.
pub(crate) fn write_functions_mod(
    mut function_dir: &mut std::path::PathBuf,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {


    let function_mod_path = function_dir.join("mod.rs");
    let mut function_mod_file =  std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&function_mod_path)?;


    let functions = definitions
        .iter()
        .filter(|d| d.category == Category::Functions);

    for definition in functions {
        write_function(&mut function_mod_file,&mut function_dir, definition, metadata, gen_bots_only_api)?;
    }

    Ok(())
}
