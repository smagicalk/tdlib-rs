// Copyright 2020 - developers of the `grammers` project.
// Copyright 2021 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module gathers all the code generation submodules and coordinates
//! them, feeding them the right data.
mod enums;
mod functions;
mod metadata;
mod rustifier;
mod types;

use std::io::{self, Write};
use tdlib_rs_parser::tl::{Definition, Type};

/// Don't generate types for definitions of this type,
/// since they are "core" types and treated differently.
const SPECIAL_CASED_TYPES: [&str; 6] = ["Bool", "Bytes", "Int32", "Int53", "Int64", "Ok"];

fn ignore_type(ty: &Type) -> bool {
    SPECIAL_CASED_TYPES.iter().any(|&x| x == ty.name)
}



pub fn generate_rust_code(
    rust_project_path: &mut std::path::PathBuf,
    definitions: &[Definition],
    gen_bots_only_api: bool,
) -> io::Result<()> {

    if rust_project_path.is_file()  { 
        std::fs::remove_file(rust_project_path.clone())?;
    }
    
    if !rust_project_path.exists() { 
        std::fs::create_dir_all(rust_project_path.clone())?;
    }
    
    
    let mut types_mod_dir = rust_project_path.join("types");
    std::fs::create_dir_all(&types_mod_dir)?;
    
    
    let metadata = metadata::Metadata::new(definitions);
    types::write_types_mod(&mut types_mod_dir, definitions, &metadata, gen_bots_only_api)?;


    let mut enums_mod_dir = rust_project_path.join("enums");
    std::fs::create_dir_all(&enums_mod_dir)?;
    enums::write_enums_mod(&mut enums_mod_dir, definitions, &metadata, gen_bots_only_api)?;
    
    let mut functions_mod_dir = rust_project_path.join("functions");
    std::fs::create_dir_all(&functions_mod_dir)?;
    functions::write_functions_mod(&mut functions_mod_dir, definitions, &metadata, gen_bots_only_api)?;

    Ok(())
}