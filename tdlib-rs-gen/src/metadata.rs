// Copyright 2020 - developers of the `grammers` project.
// Copyright 2022 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::rustifier;
use std::collections::{HashMap, HashSet};
use tdlib_rs_parser::tl::{Category, Definition, Type};

/// Additional metadata required by several parts of the generation.
pub(crate) struct Metadata<'a> {
    recursing_defs: HashSet<&'a String>,
    default_impl_defs: HashSet<&'a String>,
    defs_with_type: HashMap<&'a String, Vec<&'a Definition>>,
    class_descriptions: HashMap<String, &'a str>,
}

impl<'a> Metadata<'a> {
    pub fn new(definitions: &'a [Definition]) -> Self {
        let mut metadata = Self {
            recursing_defs: HashSet::new(),
            default_impl_defs: HashSet::new(),
            defs_with_type: HashMap::new(),
            class_descriptions: HashMap::new(),
        };

        let type_definitions = definitions
            .iter()
            .filter(|d| d.category == Category::Types)
            .collect::<Vec<_>>();

        let type_definition_map = type_definitions
            .iter()
            .map(|d| (&d.name, *d))
            .collect::<HashMap<_, _>>();

        type_definitions.iter().for_each(|d| {
            metadata
                .defs_with_type
                .entry(&d.ty.name)
                .or_default()
                .push(d);

            if let Some(ref class_desc) = d.class_description {
                metadata
                    .class_descriptions
                    .insert(d.ty.name.clone(), class_desc.as_str());
            }
        });

        type_definitions.iter().for_each(|d| {
            if def_self_references(d, d, &metadata.defs_with_type, &type_definition_map, &mut HashSet::new()) {
                metadata.recursing_defs.insert(&d.name);
            }
        });

        type_definitions.iter().for_each(|d| {
            if def_contains_only_bare_types(d, &type_definition_map) {
                metadata.default_impl_defs.insert(&d.name);
            }
        });

        metadata
    }

    /// Returns `true` if any of the parameters of `Definition` eventually
    /// contains the same type as the `Definition` itself (meaning it recurses).
    #[allow(dead_code)]
    pub fn is_recursive_def(&self, def: &Definition) -> bool {
        self.recursing_defs.contains(&def.name)
    }

    /// Returns `true` if the `Definition` can implement the trait `Default`
    pub fn can_def_implement_default(&self, def: &Definition) -> bool {
        self.default_impl_defs.contains(&def.name)
    }

    pub fn defs_with_type(&self, ty: &'a Type) -> &Vec<&Definition> {
        &self.defs_with_type[&ty.name]
    }

    /// Returns the top-level class description for an enum/union type if present.
    pub fn class_description(&self, ty_name: &str) -> Option<&'a str> {
        self.class_descriptions.get(ty_name).copied()
    }
}

fn def_self_references<'a>(
    root: &Definition,
    check: &'a Definition,
    defs_with_type: &'a HashMap<&String, Vec<&Definition>>,
    type_definition_map: &'a HashMap<&String, &Definition>,
    visited: &mut HashSet<&'a String>,
) -> bool {
    visited.insert(&check.name);
    for param in check.params.iter() {
        if param.ty.name == root.ty.name {
            return true;
        }

        // 1. 如果参数为具体联合类型（大驼峰），递归检查其所有可能的子构造器
        if let Some(defs) = defs_with_type.get(&param.ty.name) {
            for def in defs {
                if visited.contains(&def.name) {
                    continue;
                }
                if def_self_references(root, def, defs_with_type, type_definition_map, visited) {
                    return true;
                }
            }
        }

        // 2. 如果参数为裸类型（小写首字母构造器），通过定义表反查其实际归属
        if let Some(def) = type_definition_map.get(&param.ty.name) {
            if def.ty.name == root.ty.name {
                return true;
            }
            if !visited.contains(&def.name)
                && def_self_references(root, def, defs_with_type, type_definition_map, visited)
            {
                return true;
            }
        }
    }

    false
}

fn def_contains_only_bare_types<'a>(
    check: &'a Definition,
    definition_map: &'a HashMap<&String, &Definition>,
) -> bool {
    for param in check.params.iter() {
        if !rustifier::parameters::is_builtin_type(param) && !param.ty.bare {
            return false;
        }

        if let Some(def) = definition_map.get(&param.ty.name)
            && !def_contains_only_bare_types(def, definition_map)
        {
            return false;
        }
    }

    true
}
