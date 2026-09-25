// Copyright 2020 - developers of the `grammers` project.
// Copyright 2022 - developers of the `tdlib-rs` project.
// Copyright 2024 - developers of the `tgt` and `tdlib-rs` projects.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::errors::{ParamParseError, ParseError};
use crate::tl::{Category, Parameter, Type};

/// A [Type Language] definition.
///
/// [Type Language]: https://core.telegram.org/mtproto/TL
#[derive(Debug, PartialEq)]
pub struct Definition {
    /// The name of this definition. Also known as "predicate" or "method".
    pub name: String,

    /// The description of this definition.
    pub description: String,

    /// A possibly-empty list of parameters this definition has.
    pub params: Vec<Parameter>,

    /// The type to which this definition belongs to.
    pub ty: Type,

    /// The category to which this definition belongs to.
    pub category: Category,

    /// Optional top-level class description (from `//@class ... @description ...`).
    pub class_description: Option<String>,
}

impl fmt::Display for Definition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;

        for param in self.params.iter() {
            write!(f, " {param}")?;
        }
        write!(f, " = {}", self.ty)?;
        Ok(())
    }
}

impl FromStr for Definition {
    type Err = ParseError;

    /// Parses a [Type Language] definition.
    ///
    /// # Examples
    ///
    /// ```
    /// use tdlib_rs_parser::tl::Definition;
    ///
    /// assert!("sendMessage chat_id:int message:string = Message".parse::<Definition>().is_ok());
    /// ```
    ///
    /// [Type Language]: https://core.telegram.org/mtproto/TL
    fn from_str(definition: &str) -> Result<Self, Self::Err> {
        if definition.trim().is_empty() {
            return Err(ParseError::Empty);
        }

        let (definition, mut docs, class_description) = {
            let mut docs = HashMap::new();
            let mut comments_end = 0;
            let mut class_description = None;
            let mut has_class = false;

            if let Some(start) = definition.rfind("//")
                && let Some(end) = definition[start..].find('\n')
            {
                comments_end = start + end;
            }

            let mut offset = 0;
            while let Some(start) = definition[offset..].find('@') {
                let start = start + offset;
                let end = if let Some(end) = definition[start + 1..].find('@') {
                    start + 1 + end
                } else {
                    comments_end
                };

                let comment = definition[start + 1..end].replace("//-", "");
                let comment = comment.replace("//", "").trim().to_owned();
                if let Some((name, content)) = comment.split_once(' ') {
                    if name == "class" {
                        has_class = true;
                    } else if name == "description" && has_class && class_description.is_none() {
                        class_description = Some(content.trim().to_string());
                    } else {
                        docs.insert(name.into(), content.into());
                    }
                } else {
                    docs.insert(comment, String::new());
                }

                offset = end;
            }

            (&definition[comments_end..], docs, class_description)
        };

        // Parse `(left = ty)`
        let (left, ty) = {
            let mut it = definition.split('=');
            let ls = it.next().unwrap(); // split() always return at least one
            if let Some(t) = it.next() {
                (ls.trim(), t.trim())
            } else {
                return Err(ParseError::MissingType);
            }
        };

        let ty = Type::from_str(ty).map_err(|_| ParseError::MissingType)?;

        // Parse `name middle`
        let (name, middle) = {
            if let Some(pos) = left.find(' ') {
                (left[..pos].trim(), left[pos..].trim())
            } else {
                (left.trim(), "")
            }
        };
        if name.is_empty() {
            return Err(ParseError::MissingName);
        }

        // Parse `description`
        let description = docs.remove("description").unwrap_or_default();

        // Parse `middle`
        let params = middle
            .split_whitespace()
            .map(Parameter::from_str)
            .filter_map(|p| {
                let mut result = match p {
                    // Any parameter that's okay should just be passed as-is.
                    Ok(p) => Some(Ok(p)),

                    // Unimplenented parameters are unimplemented definitions.
                    Err(ParamParseError::NotImplemented) => Some(Err(ParseError::NotImplemented)),

                    // Any error should just become a `ParseError`
                    Err(x) => Some(Err(ParseError::InvalidParam(x))),
                };

                if let Some(Ok(ref mut param)) = result {
                    let name = if param.name == "description" {
                        "param_description"
                    } else {
                        &param.name
                    };

                    if let Some(description) = docs.remove(name) {
                        param.description = description;
                    }
                }

                result
            })
            .collect::<Result<_, ParseError>>()?;

        Ok(Definition {
            name: name.into(),
            description,
            params,
            ty,
            category: Category::Types,
            class_description,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_def() {
        assert_eq!(Definition::from_str(""), Err(ParseError::Empty));
    }

    #[test]
    fn parse_no_name() {
        assert_eq!(Definition::from_str(" = foo"), Err(ParseError::MissingName));
    }

    #[test]
    fn parse_no_type() {
        assert_eq!(Definition::from_str("foo"), Err(ParseError::MissingType));
        assert_eq!(Definition::from_str("foo = "), Err(ParseError::MissingType));
    }

    #[test]
    fn parse_unimplemented() {
        assert_eq!(
            Definition::from_str("int ? = Int"),
            Err(ParseError::NotImplemented)
        );
    }

    #[test]
    fn parse_valid_definition() {
        let def = Definition::from_str("a=d").unwrap();
        assert_eq!(def.name, "a");
        assert_eq!(def.params.len(), 0);
        assert_eq!(
            def.ty,
            Type {
                name: "d".into(),
                bare: true,
                generic_arg: None,
            }
        );

        let def = Definition::from_str("a=d<e>").unwrap();
        assert_eq!(def.name, "a");
        assert_eq!(def.params.len(), 0);
        assert_eq!(
            def.ty,
            Type {
                name: "d".into(),
                bare: true,
                generic_arg: Some(Box::new("e".parse().unwrap())),
            }
        );

        let def = Definition::from_str("a b:c = d").unwrap();
        assert_eq!(def.name, "a");
        assert_eq!(def.params.len(), 1);
        assert_eq!(
            def.ty,
            Type {
                name: "d".into(),
                bare: true,
                generic_arg: None,
            }
        );
    }

    #[test]
    fn parse_multiline_definition() {
        let def = "
            first lol:param
              = t;
            ";

        assert_eq!(Definition::from_str(def).unwrap().name, "first");

        let def = "
            second
              lol:String
            = t;
            ";

        assert_eq!(Definition::from_str(def).unwrap().name, "second");

        let def = "
            third

              lol:String

            =
                     t;
            ";

        assert_eq!(Definition::from_str(def).unwrap().name, "third");
    }

    #[test]
    fn parse_complete() {
        let def = "
            //@description This is a test description
            name pname:Vector<X> = Type";
        assert_eq!(
            Definition::from_str(def),
            Ok(Definition {
                name: "name".into(),
                description: "This is a test description".into(),
                params: vec![Parameter {
                    name: "pname".into(),
                    ty: Type {
                        name: "Vector".into(),
                        bare: false,
                        generic_arg: Some(Box::new(Type {
                            name: "X".into(),
                            bare: false,
                            generic_arg: None,
                        })),
                    },
                    description: String::new(),
                },],
                ty: Type {
                    name: "Type".into(),
                    bare: false,
                    generic_arg: None,
                },
                category: Category::Types,
                class_description: None,
            })
        );
    }

    #[test]
    fn parse_with_class_description() {
        let def = "
            //@class TestClass @description This is a class description
            //@description This is a constructor description
            testConstructor = TestClass";
        let parsed = Definition::from_str(def).unwrap();
        assert_eq!(parsed.name, "testConstructor");
        assert_eq!(parsed.description, "This is a constructor description");
        assert_eq!(
            parsed.class_description,
            Some("This is a class description".into())
        );
    }

    #[test]
    fn test_to_string() {
        let def = "name pname:Vector<X> = Type";
        assert_eq!(Definition::from_str(def).unwrap().to_string(), def);
    }
}
