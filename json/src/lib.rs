use parsing::{StrParser, prelude::*};
use serializer::{DataHolder, Deserialize, PrimType, Serialize};
use std::{collections::HashMap, io::Read};
// See rfc4627, rfc8259
#[derive(Debug, PartialEq)]
pub struct JsonVal {
    data: DataHolder,
}

fn print_prim_type(
    ty: &PrimType,
    val: &String,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match ty {
        PrimType::Bool => {
            if val == "true" {
                write!(f, "true")
            } else {
                write!(f, "false")
            }
        }
        PrimType::String | PrimType::Char => write!(f, "\"{}\"", val),
        PrimType::None => write!(f, "null"),
        _ => write!(f, "{}", val),
    }
}

fn print_dataholder(data_holder: &DataHolder, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match data_holder {
        DataHolder::Primitive { ty, val } => print_prim_type(ty, val, f),
        DataHolder::Array(elements) => {
            write!(f, "[")?;
            let len = elements.len();
            for (i, item) in elements.iter().enumerate() {
                print_dataholder(item, f)?;
                if i != len - 1 {
                    write!(f, ",")?;
                }
            }
            write!(f, "]")
        }
        DataHolder::Struct(obj) => {
            write!(f, "{{")?;
            let len = obj.len();
            for (i, (key, val)) in obj.iter().enumerate() {
                write!(f, "\"{}\":", key)?;
                print_dataholder(val, f)?;
                if i != len - 1 {
                    write!(f, ",")?;
                }
            }
            write!(f, "}}")
        }
    }
}

impl std::fmt::Display for JsonVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        print_dataholder(&self.data, f)
    }
}

fn parse_number<R: Read>(parser: &mut Parser<R>, has_minus: bool) -> ParseResult<JsonVal> {
    let mut s = if has_minus {
        String::from("-")
    } else {
        String::new()
    };
    if parser.matches(|b| b == b'0') {
        let c = parser
            .consume()
            .expect("json number parse found none after leading zero check");
        s.push(c as char);
        if parser.is_digit() {
            return Err(ParseErr::FoundLeadingZero);
        }
    }

    let mut is_float = false;

    loop {
        if !is_float && parser.matches(|b| b == b'.' || b == b'e' || b == b'E') {
            is_float = true;
        } else if !parser.is_digit() {
            break;
        }

        let c = parser
            .consume()
            .expect("json parser returned none after float check") as char;

        s.push(c);
    }

    if is_float {
        // let f = s.parse::<f64>().map_err(|_| ParseErr::FailedToParseNum {
        //     found: s,
        //     radix: 10,
        // })?;
        Ok(JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::F64,
                val: s,
            },
        })
    } else {
        Ok(JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::I64,
                val: s,
            },
        })
    }
}
impl<R: Read> Parsable<R> for JsonVal {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        parser.skip_whitespace_and_lines();
        if parser.is_dquote() {
            parser.consume_or_err(|b| b == b'"')?;
            let s = parser.consume_str_lit();
            parser.consume_or_err(|b| b == b'"')?;
            Ok(JsonVal {
                data: DataHolder::Primitive {
                    ty: PrimType::String,
                    val: s,
                },
            })
        } else if parser.matches(|b| b == b'{') {
            parser.skip_whitespace_and_lines();
            parser.consume_or_err(|b| b == b'{')?;
            parser.skip_whitespace_and_lines();
            let mut map = HashMap::new();
            while parser.is_dquote() {
                parser.consume_or_err(|b| b == b'"')?;
                eprintln!("{:#?}", parser.peek());
                let key = parser.consume_str_lit();
                parser.consume_or_err(|b| b == b'"')?;
                eprintln!("key: {}", key);
                parser.skip_whitespace_and_lines();
                parser.consume_or_err(|b| b == b':')?;
                parser.skip_whitespace_and_lines();
                let val = JsonVal::parse(parser)?;
                parser.skip_whitespace_and_lines();
                map.insert(key, val.data);
                if parser.matches(|b| b == b'}') {
                    break;
                } else {
                    parser.consume_or_err(|b| b == b',')?;
                    parser.skip_whitespace_and_lines();
                }
            }
            parser.skip_whitespace_and_lines();
            parser.consume_or_err(|b| b == b'}')?;
            Ok(JsonVal {
                data: DataHolder::Struct(map),
            })
        } else if parser.matches(|b| b == b'[') {
            let mut vals = Vec::new();
            parser.consume_or_err(|b| b == b'[')?;
            loop {
                parser.skip_whitespace_and_lines();
                let val = JsonVal::parse(parser)?;
                vals.push(val.data);
                parser.skip_whitespace_and_lines();
                if parser.matches(|b| b == b']') {
                    break;
                }
                parser.consume_or_err(|b| b == b',')?;
            }
            parser.consume_or_err(|b| b == b']')?;
            Ok(JsonVal {
                data: DataHolder::Array(vals),
            })
        } else if parser.is_alpha() {
            let keyword = parser.consume_while(|this| this.is_alpha());
            match keyword.as_str() {
                "true" => Ok(JsonVal {
                    data: DataHolder::Primitive {
                        ty: PrimType::Bool,
                        val: String::from("true"),
                    },
                }),
                "false" => Ok(JsonVal {
                    data: DataHolder::Primitive {
                        ty: PrimType::Bool,
                        val: String::from("false"),
                    },
                }),
                "null" => Ok(JsonVal {
                    data: DataHolder::Primitive {
                        ty: PrimType::None,
                        val: String::from(""),
                    },
                }),
                _ => Err(ParseErr::ExpectedValidKeyword {
                    found: keyword,
                    at: parser.idx(),
                }),
            }
        } else if parser.is_digit() {
            parse_number(parser, false)
        } else if parser.matches(|b| b == b'-') {
            parser.consume_or_err(|b| b == b'-')?;
            parse_number(parser, true)
        } else {
            Err(ParseErr::InvalidUTF8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parsing::StrParser;

    #[test]
    fn test_parsing() {
        let id = JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::I64,
                val: String::from("1"),
            },
        };
        let name = JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("Alice"),
            },
        };
        let active = JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::Bool,
                val: String::from("true"),
            },
        };
        let roles = JsonVal {
            data: DataHolder::Array(vec![
                DataHolder::Primitive {
                    ty: PrimType::String,
                    val: String::from("admin"),
                },
                DataHolder::Primitive {
                    ty: PrimType::String,
                    val: String::from("editor"),
                },
            ]),
        };
        let age = JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::F64,
                val: String::from("29.5"),
            },
        };
        let email = JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("alice@example.com"),
            },
        };
        let theme = JsonVal {
            data: DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("dark"),
            },
        };
        let mut profile = HashMap::new();
        profile.insert(String::from("age"), age.data);
        profile.insert(String::from("email"), email.data);
        let mut preferences = HashMap::new();
        preferences.insert(String::from("theme"), theme.data);
        profile.insert(String::from("preferences"), DataHolder::Struct(preferences));

        let mut obj_map = HashMap::new();
        obj_map.insert(String::from("id"), id.data);
        obj_map.insert(String::from("name"), name.data);
        obj_map.insert(String::from("active"), active.data);
        obj_map.insert(String::from("roles"), roles.data);
        obj_map.insert(String::from("profile"), DataHolder::Struct(profile));
        let mut test_json = StrParser::from_str(
            r#"{
  "id": 1,
  "name": "Alice",
  "active": true,
  "roles": ["admin", "editor"],
  "profile": {
    "age": 29.5,
    "email": "alice@example.com",
    "preferences": {
      "theme": "dark",
    }
  }
}"#,
        );

        let obj = JsonVal {
            data: DataHolder::Struct(obj_map),
        };
        assert_eq!(JsonVal::parse(&mut test_json), Ok(obj))
    }
}

// macro_rules! impl_into_json_int {
//     ($t: ty) => {
//         impl From<$t> for JsonVal {
//             fn from(value: $t) -> Self {
//                 JsonVal::Int(value as i64)
//             }
//         }
//     };
// }

// impl_into_json_int!(i8);
// impl_into_json_int!(i16);
// impl_into_json_int!(i32);

// impl_into_json_int!(u8);
// impl_into_json_int!(u16);
// impl_into_json_int!(u32);
// impl_into_json_int!(u64);

// impl From<f32> for JsonVal {
//     fn from(value: f32) -> Self {
//         JsonVal::Float(value as f64)
//     }
// }

// impl From<String> for JsonVal {
//     fn from(value: String) -> Self {
//         JsonVal::String(value)
//     }
// }

// impl From<&str> for JsonVal {
//     fn from(value: &str) -> Self {
//         JsonVal::String(value.to_string())
//     }
// }

// impl<T: Into<JsonVal>> From<Vec<T>> for JsonVal {
//     fn from(value: Vec<T>) -> Self {
//         JsonVal::Array(value.into_iter().map(|entry| entry.into()).collect())
//     }
// }
// pub trait FromJson: Sized + Deserialize {
//     fn from_json(json: JsonVal) -> ParseResult<Self> {
//         let data_holder = match json {
//             JsonVal::String(s) => Dataholder::Primative {
//                 ty: PrimType::String,
//                 val: s,
//             },
//             _ => todo!(),
//         };

//         Err(ParseErr::InvalidUTF8)
//     }
// }
