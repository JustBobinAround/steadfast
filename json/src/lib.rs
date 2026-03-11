use parsing::{StrParser, prelude::*};
use serializer::{DataHolder, Deserialize, PrimType, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
};
// See rfc4627, rfc8259
#[derive(Debug, PartialEq)]
pub struct JsonVal {
    data: DataHolder,
}

fn print_prim_type(ty: &PrimType, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match ty {
        PrimType::Bool(bool) => write!(f, "{}", bool),
        PrimType::Char(char) => write!(f, "{}", char),
        PrimType::F32(f32) => write!(f, "{}", f32),
        PrimType::F64(f64) => write!(f, "{}", f64),
        PrimType::I8(i8) => write!(f, "{}", i8),
        PrimType::I16(i16) => write!(f, "{}", i16),
        PrimType::I32(i32) => write!(f, "{}", i32),
        PrimType::I64(i64) => write!(f, "{}", i64),
        PrimType::I128(i128) => write!(f, "{}", i128),
        PrimType::Isize(isize) => write!(f, "{}", isize),
        PrimType::U8(u8) => write!(f, "{}", u8),
        PrimType::U16(u16) => write!(f, "{}", u16),
        PrimType::U32(u32) => write!(f, "{}", u32),
        PrimType::U64(u64) => write!(f, "{}", u64),
        PrimType::U128(u128) => write!(f, "{}", u128),
        PrimType::Usize(usize) => write!(f, "{}", usize),
        PrimType::String(string) => write!(f, "{}", string),
        PrimType::None => write!(f, "null"),
    }
}

fn print_dataholder(data_holder: &DataHolder, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match data_holder {
        DataHolder::Primitive(ty) => print_prim_type(ty, f),
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
        let f = s.parse::<f64>().map_err(|_| ParseErr::FailedToParseNum {
            found: s,
            radix: 10,
        })?;
        Ok(JsonVal {
            data: DataHolder::Primitive(PrimType::F64(f)),
        })
    } else {
        let i = s.parse::<i64>().map_err(|_| ParseErr::FailedToParseNum {
            found: s,
            radix: 10,
        })?;
        Ok(JsonVal {
            data: DataHolder::Primitive(PrimType::I64(i)),
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
                data: DataHolder::Primitive(PrimType::String(s)),
            })
        } else if parser.matches(|b| b == b'{') {
            parser.skip_whitespace_and_lines();
            parser.consume_or_err(|b| b == b'{')?;
            parser.skip_whitespace_and_lines();
            let mut map = BTreeMap::new();
            while parser.is_dquote() {
                parser.consume_or_err(|b| b == b'"')?;
                let key = parser.consume_str_lit();
                parser.consume_or_err(|b| b == b'"')?;
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
                    data: DataHolder::Primitive(PrimType::Bool(true)),
                }),
                "false" => Ok(JsonVal {
                    data: DataHolder::Primitive(PrimType::Bool(false)),
                }),
                "null" => Ok(JsonVal {
                    data: DataHolder::Primitive(PrimType::None),
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

    #[test]
    fn test_parsing() {
        let id = JsonVal {
            data: DataHolder::Primitive(PrimType::I64(1)),
        };
        let name = JsonVal {
            data: DataHolder::Primitive(PrimType::String(String::from("Alice"))),
        };
        let active = JsonVal {
            data: DataHolder::Primitive(PrimType::Bool(true)),
        };
        let roles = JsonVal {
            data: DataHolder::Array(vec![
                DataHolder::Primitive(PrimType::String(String::from("admin"))),
                DataHolder::Primitive(PrimType::String(String::from("editor"))),
            ]),
        };
        let age = JsonVal {
            data: DataHolder::Primitive(PrimType::F64(29.5)),
        };
        let email = JsonVal {
            data: DataHolder::Primitive(PrimType::String(String::from("alice@example.com"))),
        };
        let theme = JsonVal {
            data: DataHolder::Primitive(PrimType::String(String::from("dark"))),
        };
        let mut profile = BTreeMap::new();
        profile.insert(String::from("age"), age.data);
        profile.insert(String::from("email"), email.data);
        let mut preferences = BTreeMap::new();
        preferences.insert(String::from("theme"), theme.data);
        profile.insert(String::from("preferences"), DataHolder::Struct(preferences));

        let mut obj_map = BTreeMap::new();
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
        assert_eq!(JsonVal::parse(&mut test_json), Ok(obj));
    }
}

impl From<DataHolder> for JsonVal {
    fn from(value: DataHolder) -> Self {
        JsonVal { data: value }
    }
}

impl From<JsonVal> for DataHolder {
    fn from(value: JsonVal) -> Self {
        value.data
    }
}

pub trait FromJson: Deserialize {
    fn from_json_str(json_str: &str) -> Result<Self, ()>;
    fn from_json_val(json_val: JsonVal) -> Result<Self, ()>;
}
impl<T: Deserialize> FromJson for T {
    fn from_json_str(json_str: &str) -> Result<Self, ()> {
        let mut str_parser = StrParser::from_str(json_str);
        let json_val = JsonVal::parse(&mut str_parser).map_err(|_| ())?;
        Self::deserialize(json_val.into())
    }
    fn from_json_val(json_val: JsonVal) -> Result<Self, ()> {
        Self::deserialize(json_val.into())
    }
}
pub trait IntoJson: Sized {
    fn to_json_val(self) -> JsonVal;
    fn to_json(self) -> String;
}
impl<T: Serialize> IntoJson for T {
    fn to_json_val(self) -> JsonVal {
        let dh = self.serialize();
        JsonVal { data: dh }
    }
    fn to_json(self) -> String {
        self.to_json_val().to_string()
    }
}
