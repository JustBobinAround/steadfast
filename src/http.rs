pub mod request;
pub mod response;
pub mod routing;
pub mod server;
pub mod uri;

use crate::steadfast_parsing::StrParser;
use crate::steadfast_parsing::prelude::*;
use crate::stream_writer::prelude::*;
pub use request::Method;
use request::RequestBody;
pub use routing::Query;
use std::collections::HashMap;
use std::io::{Read, Write};

pub struct Body<T: ToBody>(pub T);

// impl std::fmt::Display for Body<String> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }

pub trait ToBody: Sized {
    fn into_body(body: RequestBody) -> Result<Body<Self>, ()>;
}

// impl<T: Deserialize> ToBody for T {
//     fn into_body(body: RequestBody) -> Result<Body<Self>, ()> {
//         let mut parser = StrParser::from_str(&body);
//         let query = RequestQuery::parse(&mut parser).map_err(|_| ())?;
//         match T::into_query(query) {
//             Ok(routing::Query(t)) => Ok(Body(t)),
//             _ => Err(()),
//         }
//     }
// }

// TODO
// impl<T: Deserialize> ToQuery for T {
//     fn into_query(body: String) -> Result<Query<Self>, ()> {
//         let s = query.parameters;
//         match T::deserialize(s) {
//             Ok(t) => Ok(Query(t)),
//             Err(_) => Err(()),
//         }
//     }
// }

/// Based on rfc2616 Section 4.2
///
/// # Augmented Backus-Naur Form
/// ```text
///       message-header = field-name ":" [ field-value ]
///       field-name     = token
///       field-value    = *( field-content | LWS )
///       field-content  = <the OCTETs making up the field-value
///                        and consisting of either *TEXT or combinations
///                        of token, separators, and quoted-string>
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageHeader {
    name: String,
    value: String,
}

impl MessageHeader {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn extract_name_val(self) -> (String, String) {
        (self.name, self.value)
    }
}

impl MessageHeader {
    pub fn into_header<T: FromMessageHeader>(self) -> ParseResult<(String, T)> {
        T::from_extension_header(self)
    }
}

impl<R: Read> Parsable<R> for MessageHeader {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let name = parser.consume_while(|p| p.is_token_char());
        let name = name.to_ascii_lowercase();
        if name.is_empty() {
            return Err(ParseErr::BlankHeaderFieldName);
        }
        parser.skip_whitespace();
        parser.consume_or_err(|c| c == b':')?;
        parser.skip_whitespace();
        let mut parts = String::new();
        parts.push_str(
            parser
                .consume_while(|p| !p.matches(|c| c == b'\r'))
                .as_str(),
        );
        parser.consume();

        while let Some(c) = parser.peek()
            && c == b'\n'
        {
            parser.consume();
            if parser.is_linear_whitespace() {
                parser.skip_whitespace();
                parts.push_str(
                    parser
                        .consume_while(|p| !p.matches(|c| c == b'\r'))
                        .as_str(),
                );
                parser.consume();
            } else {
                break;
            }
        }

        Ok(MessageHeader { name, value: parts })
    }
}
impl<W: std::io::Write> StreamWritable<W> for MessageHeader {
    fn write_to_stream(self, stream: &mut W) -> StreamResult {
        // TODO: pct encoding of illegal value
        write!(stream, "{}: {}", self.name, self.value)?;
        Ok(())
    }
}
/// Based on rfc2616 Section 3.1
///
/// # Augmented Backus-Naur Form
/// ```text
/// HTTP-Version   = "HTTP" "/" 1*DIGIT "." 1*DIGIT
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HTTPVersion {
    major: u8,
    minor: u8,
}

impl Default for HTTPVersion {
    fn default() -> Self {
        HTTPVersion { major: 1, minor: 1 }
    }
}

impl<R: Read> Parsable<R> for HTTPVersion {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        parser.skip_whitespace();
        parser.expect_str("HTTP/")?;
        let major_str = parser.consume_while(|p| p.is_digit());
        let major =
            u8::from_str_radix(major_str.as_str(), 10).map_err(|_| ParseErr::FailedToParseNum {
                found: major_str,
                radix: 10,
            })?;
        parser.consume_or_err(|c| c == b'.')?;
        let minor_str = parser.consume_while(|p| p.is_digit());
        let minor = if minor_str.is_empty() {
            0
        } else {
            u8::from_str_radix(minor_str.as_str(), 10).map_err(|_| ParseErr::FailedToParseNum {
                found: minor_str,
                radix: 10,
            })?
        };

        Ok(HTTPVersion { major, minor })
    }
}

impl<W: Write> StreamWritable<W> for HTTPVersion {
    fn write_to_stream(self, stream: &mut W) -> StreamResult {
        if self.minor == 0 {
            write!(stream, "HTTP/{}", self.major)?;
        } else {
            write!(stream, "HTTP/{}.{}", self.major, self.minor)?;
        }

        Ok(())
    }
}

/// Based on rfc2616 Section 14
#[derive(Debug, PartialEq, Eq)]
pub enum EntityHeader {
    Allow(String),                          // Section 14.7
    ContentEncoding(String),                // Section 14.11
    ContentLanguage(String),                // Section 14.12
    ContentLength(usize),                   // Section 14.13
    ContentLocation(String),                // Section 14.14
    ContentMD5(String),                     // Section 14.15
    ContentRange(String),                   // Section 14.16
    ContentType(String),                    // Section 14.17
    Expires(String),                        // Section 14.21
    LastModified(String),                   // Section 14.29
    ContentDisposition(ContentDisposition), // RFC 6266
}

impl EntityHeader {
    pub const fn name(&self) -> &'static str {
        match self {
            EntityHeader::Allow(_) => "allow", // Section 14.7
            EntityHeader::ContentEncoding(_) => "content-encoding", // Section 14.11
            EntityHeader::ContentLanguage(_) => "content-language", // Section 14.12
            EntityHeader::ContentLength(_) => "content-length", // Section 14.13
            EntityHeader::ContentLocation(_) => "content-location", // Section 14.14
            EntityHeader::ContentMD5(_) => "content-md5", // Section 14.15
            EntityHeader::ContentRange(_) => "content-range", // Section 14.16
            EntityHeader::ContentType(_) => "content-type", // Section 14.17
            EntityHeader::Expires(_) => "expires", // Section 14.21
            EntityHeader::LastModified(_) => "last-modified", // Section 14.29
            EntityHeader::ContentDisposition(_) => "content-disposition", // RFC 6266
        }
    }
}

impl FromMessageHeader for EntityHeader {
    fn can_convert(eh: &MessageHeader) -> bool {
        let name = eh.name.as_str();
        name == "allow"
            || name == "content-encoding"
            || name == "content-language"
            || name == "content-length"
            || name == "content-location"
            || name == "content-md5"
            || name == "content-range"
            || name == "content-type"
            || name == "expires"
            || name == "last-modified"
            || name == "content-disposition"
    }
    fn from_extension_header(eh: MessageHeader) -> ParseResult<(String, Self)> {
        let val = eh.value;
        let name = eh.name.as_str();
        let header = match name {
            "allow" => Self::Allow(val),
            "content-encoding" => Self::ContentEncoding(val),
            "content-language" => Self::ContentLanguage(val),
            "content-length" => {
                let length = usize::from_str_radix(val.as_str(), 10).map_err(|_| {
                    ParseErr::FailedToParseNum {
                        found: val,
                        radix: 10,
                    }
                })?;
                Self::ContentLength(length)
            }
            "content-location" => Self::ContentLocation(val),
            "content-md5" => Self::ContentMD5(val),
            "content-range" => Self::ContentRange(val),
            "content-type" => Self::ContentType(val),
            "expires" => Self::Expires(val),
            "last-modified" => Self::LastModified(val),
            "content-disposition" => {
                let mut s_parser = StrParser::from_str(&val);
                Self::ContentDisposition(ContentDisposition::parse(&mut s_parser)?)
            }
            _ => unreachable!(
                "Failed to convert extension header. Perhaps can_convert was not checked"
            ),
        };

        Ok((eh.name, header))
    }
}

impl ToMessageHeader for EntityHeader {
    fn consume_value_as_string(self) -> String {
        match self {
            EntityHeader::Allow(s) => s,                          // Section 14.7
            EntityHeader::ContentEncoding(s) => s,                // Section 14.11
            EntityHeader::ContentLanguage(s) => s,                // Section 14.12
            EntityHeader::ContentLength(n) => n.to_string(),      // Section 14.13
            EntityHeader::ContentLocation(s) => s,                // Section 14.14
            EntityHeader::ContentMD5(s) => s,                     // Section 14.15
            EntityHeader::ContentRange(s) => s,                   // Section 14.16
            EntityHeader::ContentType(s) => s,                    // Section 14.17
            EntityHeader::Expires(s) => s,                        // Section 14.21
            EntityHeader::LastModified(s) => s,                   // Section 14.29
            EntityHeader::ContentDisposition(s) => s.to_string(), // RFC 6266
        }
    }
    fn to_msg_header(self) -> MessageHeader {
        let name = self.name().to_string();
        let value = self.consume_value_as_string();

        MessageHeader { name, value }
    }
}

pub trait FromMessageHeader: Sized {
    fn can_convert(eh: &MessageHeader) -> bool;
    fn from_extension_header(eh: MessageHeader) -> ParseResult<(String, Self)>;
}
pub trait ToMessageHeader: Sized {
    fn consume_value_as_string(self) -> String;
    fn to_msg_header(self) -> MessageHeader;
}
/// Based on rfc2616 Section 14
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeneralHeader {
    CacheControl(String),     // Section 14.9
    Connection(String),       // Section 14.10
    Date(String),             // Section 14.18
    Pragma(String),           // Section 14.32
    Trailer(String),          // Section 14.40
    TransferEncoding(String), // Section 14.41
    Upgrade(String),          // Section 14.42
    Via(String),              // Section 14.45
    Warning(String),          // Section 14.46
}

impl GeneralHeader {
    pub const fn name(&self) -> &'static str {
        match self {
            GeneralHeader::CacheControl(_) => "cache-control", // Section 14.9
            GeneralHeader::Connection(_) => "connection",      // Section 14.10
            GeneralHeader::Date(_) => "date",                  // Section 14.18
            GeneralHeader::Pragma(_) => "pragma",              // Section 14.32
            GeneralHeader::Trailer(_) => "trailer",            // Section 14.40
            GeneralHeader::TransferEncoding(_) => "transfer-encoding", // Section 14.41
            GeneralHeader::Upgrade(_) => "upgrade",            // Section 14.42
            GeneralHeader::Via(_) => "via",                    // Section 14.45
            GeneralHeader::Warning(_) => "warning",            // Section 14.46
        }
    }
}

impl FromMessageHeader for GeneralHeader {
    fn can_convert(eh: &MessageHeader) -> bool {
        let name = eh.name.as_str();
        name == "cache-control"
            || name == "connection"
            || name == "date"
            || name == "pragma"
            || name == "trailer"
            || name == "transfer-encoding"
            || name == "upgrade"
            || name == "via"
            || name == "warning"
    }

    fn from_extension_header(eh: MessageHeader) -> ParseResult<(String, Self)> {
        let val = eh.value;
        let name = eh.name.as_str();
        let header = match name {
            "cache-control" => Self::CacheControl(val),
            "connection" => Self::Connection(val),
            "date" => Self::Date(val),
            "pragma" => Self::Pragma(val),
            "trailer" => Self::Trailer(val),
            "transfer-encoding" => Self::TransferEncoding(val),
            "upgrade" => Self::Upgrade(val),
            "via" => Self::Via(val),
            "warning" => Self::Warning(val),
            _ => unreachable!(
                "Failed to convert extension header. Perhaps can_convert was not checked"
            ),
        };

        Ok((eh.name, header))
    }
}

impl ToMessageHeader for GeneralHeader {
    fn consume_value_as_string(self) -> String {
        match self {
            GeneralHeader::CacheControl(s) => s,     // Section 14.9
            GeneralHeader::Connection(s) => s,       // Section 14.10
            GeneralHeader::Date(s) => s,             // Section 14.18
            GeneralHeader::Pragma(s) => s,           // Section 14.32
            GeneralHeader::Trailer(s) => s,          // Section 14.40
            GeneralHeader::TransferEncoding(s) => s, // Section 14.41
            GeneralHeader::Upgrade(s) => s,          // Section 14.42
            GeneralHeader::Via(s) => s,              // Section 14.45
            GeneralHeader::Warning(s) => s,          // Section 14.46
        }
    }
    fn to_msg_header(self) -> MessageHeader {
        let name = self.name().to_string();
        let value = self.consume_value_as_string();

        MessageHeader { name, value }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DispositionType {
    Inline,
    Attachment,
    DispExtType(String),
}
impl std::fmt::Display for DispositionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inline => write!(f, "inline"),
            Self::Attachment => write!(f, "attachment"),
            Self::DispExtType(s) => write!(f, "{}", s),
        }
    }
}

impl<R: Read> Parsable<R> for DispositionType {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let s = parser.consume_while_lower(|p| p.is_token_char());
        if s.len() == 0 {
            return Err(ParseErr::ZeroLenDispositionTy);
        }

        Ok(if s == "inline" {
            Self::Inline
        } else if s == "attachment" {
            Self::Attachment
        } else {
            Self::DispExtType(s)
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ContentDisposition {
    ty: DispositionType,
    params: HashMap<String, String>,
}

impl std::fmt::Display for ContentDisposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ty.to_string())?;
        for (k, v) in self.params.iter() {
            v.escape_default().to_string(); // TODO: verify if OOB escapes are fully http compliant
            write!(f, ";{}=\"{}\"", k, v)?;
        }
        Ok(())
    }
}

impl<R: Read> Parsable<R> for ContentDisposition {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let ty = DispositionType::parse(parser)?;
        let mut params = HashMap::new();

        while parser.matches(|b| b == b';') {
            parser.consume();
            let key = parser.consume_while(|p| p.is_token_char());
            parser.expect_str("=")?;
            let val = if parser.matches(|b| b == b'"') {
                parser.consume_str_lit()
            } else {
                parser.consume_while(|p| p.is_token_char())
            };

            params.insert(key, val);
        }

        Ok(Self { ty, params })
    }
}
