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

use std::fs::File;
use crate::ignore_type;
use crate::metadata::Metadata;
use crate::rustifier;
use std::io::{self, Write};
use std::path::PathBuf;
use tdlib_rs_parser::tl::{Category, Definition};
use convert_case::{ Case,Casing};

/// Defines the `struct` corresponding to the definition:
///
/// ```ignore
/// pub struct Name {
///     pub field: Type,
/// }
/// ```
fn write_struct(
    types_mod: &mut File,
    types_dir: &mut &mut PathBuf,
    def: &Definition,
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    if rustifier::definitions::is_for_bots_only(def) && !gen_bots_only_api {
        return Ok(());
    }



    let type_name = rustifier::definitions::type_name(&def);
    let file_name = type_name.to_case(Case::Snake);

    writeln!(types_mod, "mod {};", file_name)?;
    writeln!(types_mod, "pub use {}::{};",file_name,type_name)?;
    writeln!(types_mod, "")?;
    writeln!(types_mod, "")?;

    let type_path = types_dir.join(file_name).with_extension("rs");
    let mut type_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&type_path)?;

    writeln!(type_file, "#[allow(clippy::all)]")?;
    writeln!(type_file, "use serde::{{Deserialize, Serialize}};")?;
    writeln!(type_file, "use serde_with::{{serde_as, DisplayFromStr}};")?;
    writeln!(type_file, "")?;
    writeln!(type_file, "")?;


    writeln!(type_file, "{}", rustifier::definitions::description(def, ""))?;

    let serde_as = def
        .params
        .iter()
        .any(|p| rustifier::parameters::serde_as(p).is_some());

    if serde_as {
        writeln!(type_file, "#[serde_as]",)?;
    }

    write!(type_file, "#[derive(Clone, Debug, ",)?;
    if metadata.can_def_implement_default(def) {
        write!(type_file, "Default, ",)?;
    }
    writeln!(type_file, "PartialEq, Deserialize, Serialize)]",)?;

    writeln!(
        type_file,
        "pub struct {} {{",
        rustifier::definitions::type_name(def),
    )?;

    for param in def.params.iter() {
        if rustifier::parameters::is_for_bots_only(param) && !gen_bots_only_api {
            continue;
        }

        writeln!(
            type_file,
            "{}",
            rustifier::parameters::description(param, "    ")
        )?;

        if let Some(serde_as) = rustifier::parameters::serde_as(param) {
            writeln!(type_file, "    #[serde_as(as = \"{serde_as}\")]")?;
        }
        write!(
            type_file,
            "    pub {}: ",
            rustifier::parameters::attr_name(param),
        )?;

        let is_optional = rustifier::parameters::is_optional(param);
        if is_optional {
            write!(type_file, "Option<")?;
        }
        write!(type_file, "{}", rustifier::parameters::qual_name(param))?;
        if is_optional {
            write!(type_file, ">")?;
        }

        writeln!(type_file, ",")?;
    }

    writeln!(type_file, "}}")?;
    Ok(())
}


/// Writes an entire definition as Rust code (`struct`).


/// Write the entire module dedicated to types.
pub(crate) fn write_types_mod(
    mut types_mod_dir: &mut std::path::PathBuf,
    definitions: &[Definition],
    metadata: &Metadata,
    gen_bots_only_api: bool,
) -> io::Result<()> {
    // Begin outermost mod
    // writeln!(file, "#[allow(clippy::all)]")?;
    // writeln!(file, "pub mod types {{")?;
    // writeln!(file, "    use serde::{{Deserialize, Serialize}};")?;
    // writeln!(file, "    use serde_with::{{serde_as, DisplayFromStr}};")?;

    let types_mod_path = types_mod_dir.join("mod.rs");
    let mut types_mod_file =  std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&types_mod_path)?;


    let types = definitions
        .iter()
        .filter(|d| d.category == Category::Types && !ignore_type(&d.ty) && !d.params.is_empty());

    for definition in types {
        // pub use  error::Error;
        write_struct(&mut  types_mod_file,&mut types_mod_dir, definition, metadata, gen_bots_only_api)?;
    }

    // End outermost mod
   Ok(())
}
