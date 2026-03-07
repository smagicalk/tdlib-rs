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
use std::io::{self, Write};
use convert_case::{Case, Casing};
use tdlib_rs_parser::tl::{Category, Definition, Type};

/// Writes an enumeration listing all types such as the following rust code:
///
/// ```ignore
/// pub enum Name {
///     Variant(crate::types::Name),
/// }
/// ```
fn write_enum(
    enums_dir: &mut std::path::PathBuf,
    ty: &Type,
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {

    let enum_name = rustifier::types::type_name(&ty);
    let file_name = enum_name.to_case(Case::Snake);
    let enum_path = enums_dir.join(file_name).with_extension("rs");
    let mut enum_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&enum_path)?;

    writeln!(enum_file, "#[allow(clippy::all)]")?;
    writeln!(enum_file, "use serde::{{Deserialize, Serialize}};")?;

    writeln!(enum_file, "")?;
    writeln!(enum_file, "")?;


    writeln!(
        enum_file,
        "#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]",
    )?;
    writeln!(enum_file, "#[serde(tag = \"@type\")]")?;
    writeln!(enum_file, "pub enum {} {{", rustifier::types::type_name(ty))?;
    for d in metadata.defs_with_type(ty) {
        if rustifier::definitions::is_for_bots_only(d) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            enum_file,
            "{}",
            rustifier::definitions::description(d, "    ")
        )?;
        writeln!(
            enum_file,
            "    #[serde(rename(serialize = \"{0}\", deserialize = \"{0}\"))]",
            d.name
        )?;
        write!(enum_file, "    {}", rustifier::definitions::variant_name(d))?;

        // Variant with no struct since it has no data and it only adds noise
        if d.params.is_empty() {
            writeln!(enum_file, ",")?;
            continue;
        } else {
            write!(enum_file, "(")?;
        }

        if metadata.is_recursive_def(d) {
            write!(enum_file, "Box<")?;
        }
        write!(enum_file, "{}", rustifier::definitions::qual_name(d))?;
        if metadata.is_recursive_def(d) {
            write!(enum_file, ">")?;
        }

        writeln!(enum_file, "),")?;
    }
    writeln!(enum_file, "}}")?;
    Ok(())
}

/// Write the entire module dedicated to enums.
pub(crate) fn write_enums_mod(
    mut enums_mod_dir: &mut std::path::PathBuf,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // Begin outermost mod


    let enums_mod_path = enums_mod_dir.join("mod.rs");
    let mut enums_mod_file =  std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&enums_mod_path)?;


    let mut enums: Vec<&Type> = definitions
        .iter()
        .filter(|d| d.category == Category::Types && !ignore_type(&d.ty))
        .map(|d| &d.ty)
        .collect();
    enums.dedup();

    for ty in enums {
        let enum_name = rustifier::types::type_name(&ty);
        let file_name = enum_name.to_case(Case::Snake);

        writeln!(enums_mod_file, "mod {};", file_name)?;
        writeln!(enums_mod_file, "pub use {}::{};",file_name,enum_name)?;
        writeln!(enums_mod_file, "")?;
        writeln!(enums_mod_file, "")?;
        write_enum(&mut enums_mod_dir, ty, metadata, gen_bots_only_api)?;

    }
    Ok(())
}
