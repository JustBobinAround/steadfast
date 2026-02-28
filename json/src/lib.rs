use parsing::prelude::*;
use std::collections::HashMap;
use std::io::Read;
// See rfc4627, rfc8259
#[derive(Debug, PartialEq)]
pub enum JsonVal {
    String(String),
    Float(f64),
    Int(i64),
    Array(Vec<JsonVal>),
    Object(JsonObj),
    Bool(bool),
    None,
}

impl std::fmt::Display for JsonVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Float(s) => write!(f, "{}", s),
            Self::Int(s) => write!(f, "{}", s),
            Self::Array(s) => {
                write!(f, "[")?;
                let len = s.len();
                for (i, item) in s.iter().enumerate() {
                    if i == len - 1 {
                        write!(f, "{}", item)?;
                    } else {
                        write!(f, "{},", item)?;
                    }
                }
                write!(f, "]")
            }
            Self::Object(o) => write!(f, "{}", o),
            Self::Bool(b) => {
                if *b {
                    write!(f, "true")
                } else {
                    write!(f, "false")
                }
            }
            Self::None => {
                write!(f, "null")
            }
        }
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
        Ok(JsonVal::Float(f))
    } else {
        let i = s.parse::<i64>().map_err(|_| ParseErr::FailedToParseNum {
            found: s,
            radix: 10,
        })?;
        Ok(JsonVal::Int(i))
    }
}
impl<R: Read> Parsable<R> for JsonVal {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        parser.skip_whitespace_and_lines();
        if parser.is_dquote() {
            parser.consume_or_err(|b| b == b'"')?;
            let s = parser.consume_str_lit();
            parser.consume_or_err(|b| b == b'"')?;
            Ok(JsonVal::String(s))
        } else if parser.matches(|b| b == b'{') {
            let obj = JsonObj::parse(parser)?;
            Ok(JsonVal::Object(obj))
        } else if parser.matches(|b| b == b'[') {
            let mut vals = Vec::new();
            parser.consume_or_err(|b| b == b'[')?;
            loop {
                parser.skip_whitespace_and_lines();
                let val = JsonVal::parse(parser)?;
                vals.push(val);
                parser.skip_whitespace_and_lines();
                if parser.matches(|b| b == b']') {
                    break;
                }
                parser.consume_or_err(|b| b == b',')?;
            }
            parser.consume_or_err(|b| b == b']')?;
            Ok(JsonVal::Array(vals))
        } else if parser.is_alpha() {
            let keyword = parser.consume_while(|this| this.is_alpha());
            match keyword.as_str() {
                "true" => Ok(JsonVal::Bool(true)),
                "false" => Ok(JsonVal::Bool(false)),
                "null" => Ok(JsonVal::None),
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

#[derive(Debug, PartialEq)]
pub struct JsonObj {
    map: HashMap<String, JsonVal>,
}
impl std::fmt::Display for JsonObj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let len = self.map.len();
        for (i, (key, val)) in self.map.iter().enumerate() {
            if i == len - 1 {
                write!(f, "\"{}\": {}", key, val)?;
            } else {
                write!(f, "\"{}\": {},", key, val)?;
            }
        }
        write!(f, "}}")
    }
}

impl JsonObj {
    pub fn new(map: HashMap<String, JsonVal>) -> JsonObj {
        JsonObj { map }
    }
}

impl<R: Read> Parsable<R> for JsonObj {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
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
            map.insert(key, val);
            if parser.matches(|b| b == b'}') {
                break;
            } else {
                parser.consume_or_err(|b| b == b',')?;
                parser.skip_whitespace_and_lines();
            }
        }
        parser.skip_whitespace_and_lines();
        parser.consume_or_err(|b| b == b'}')?;

        Ok(JsonObj { map })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parsing::StrParser;

    #[test]
    fn test_parsing() {
        let id = JsonVal::Int(1);
        let name = JsonVal::String("Alice".to_string());
        let active = JsonVal::Bool(true);
        let roles = JsonVal::Array(vec![
            JsonVal::String("admin".to_string()),
            JsonVal::String("editor".to_string()),
        ]);
        let age = JsonVal::Int(29);
        let email = JsonVal::String("alice@example.com".to_string());
        let theme = JsonVal::String("dark".to_string());
        let mut profile = HashMap::new();
        profile.insert(String::from("age"), age);
        profile.insert(String::from("email"), email);
        let mut preferences = HashMap::new();
        preferences.insert(String::from("theme"), theme);
        profile.insert(
            String::from("preferences"),
            JsonVal::Object(JsonObj::new(preferences)),
        );

        let mut obj_map = HashMap::new();
        obj_map.insert(String::from("id"), id);
        obj_map.insert(String::from("name"), name);
        obj_map.insert(String::from("active"), active);
        obj_map.insert(String::from("roles"), roles);
        obj_map.insert(
            String::from("profile"),
            JsonVal::Object(JsonObj::new(profile)),
        );
        let mut test_json = StrParser::from_str(
            r#"{
  "id": 1,
  "name": "Alice",
  "active": true,
  "roles": ["admin", "editor"],
  "profile": {
    "age": 29,
    "email": "alice@example.com",
    "preferences": {
      "theme": "dark",
    }
  }
}"#,
        );

        let obj = JsonObj::new(obj_map);
        assert_eq!(JsonObj::parse(&mut test_json), Ok(obj))
    }
}

macro_rules! impl_into_json_int {
    ($t: ty) => {
        impl From<$t> for JsonVal {
            fn from(value: $t) -> Self {
                JsonVal::Int(value as i64)
            }
        }
    };
}

impl_into_json_int!(i8);
impl_into_json_int!(i16);
impl_into_json_int!(i32);

impl_into_json_int!(u8);
impl_into_json_int!(u16);
impl_into_json_int!(u32);
impl_into_json_int!(u64);

impl From<f32> for JsonVal {
    fn from(value: f32) -> Self {
        JsonVal::Float(value as f64)
    }
}

impl From<String> for JsonVal {
    fn from(value: String) -> Self {
        JsonVal::String(value)
    }
}

impl From<&str> for JsonVal {
    fn from(value: &str) -> Self {
        JsonVal::String(value.to_string())
    }
}

impl<T: Into<JsonVal>> From<Vec<T>> for JsonVal {
    fn from(value: Vec<T>) -> Self {
        JsonVal::Array(value.into_iter().map(|entry| entry.into()).collect())
    }
}
