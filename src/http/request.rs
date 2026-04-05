use super::{
    EntityHeader, FromMessageHeader, GeneralHeader, HTTPVersion, MessageHeader,
    uri::{RequestQuery, URIPath},
};
use crate::steadfast_parsing::prelude::*;
use std::{collections::BTreeMap, io::Read};

pub trait FromRequest: Sized {
    fn from_request(a: Request) -> Self;
}

/// Based on rfc2616 Section 5.1.1
///
/// # Augmented Backus-Naur Form
/// ```text
/// Method         = "OPTIONS"                ; Section 9.2
///   | "GET"                    ; Section 9.3
///   | "HEAD"                   ; Section 9.4
///   | "POST"                   ; Section 9.5
///   | "PUT"                    ; Section 9.6
///   | "DELETE"                 ; Section 9.7
///   | "TRACE"                  ; Section 9.8
///   | "CONNECT"                ; Section 9.9
///   | extension-method
///
/// extension-method = token
/// ```
///
/// Not supporting extension methods for now
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Method {
    Options,
    Get,
    Head,
    Post,
    Put,
    Delete,
    Trace,
    Connect,
}

impl<R: Read> Parsable<R> for Method {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        parser.skip_whitespace();
        let token = parser.consume_while(|p| p.is_alpha());
        match token.as_str() {
            "OPTIONS" => Ok(Method::Options),
            "GET" => Ok(Method::Get),
            "HEAD" => Ok(Method::Head),
            "POST" => Ok(Method::Post),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            "TRACE" => Ok(Method::Trace),
            "CONNECT" => Ok(Method::Connect),
            _ => Err(ParseErr::InvalidRequestOption { found: token }),
        }
    }
}

/// Based on rfc2616 Section 14
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestHeader {
    Accept(String),             // Section 14.1
    AcceptCharset(String),      // Section 14.2
    AcceptEncoding(String),     // Section 14.3
    AcceptLanguage(String),     // Section 14.4
    Authorization(String),      // Section 14.8
    Expect(String),             // Section 14.20
    From(String),               // Section 14.22
    Host(String),               // Section 14.23
    IfMatch(String),            // Section 14.24
    IfModifiedSince(String),    // Section 14.25
    IfNoneMatch(String),        // Section 14.26
    IfRange(String),            // Section 14.27
    IfUnmodifiedSince(String),  // Section 14.28
    MaxForwards(String),        // Section 14.31
    ProxyAuthorization(String), // Section 14.34
    Range(String),              // Section 14.35
    Referer(String),            // Section 14.36
    TE(String),                 // Section 14.39
    UserAgent(String),          // Section 14.43
}

impl RequestHeader {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Accept(_) => "accept",
            Self::AcceptCharset(_) => "accept-charset",
            Self::AcceptEncoding(_) => "accept-encoding",
            Self::AcceptLanguage(_) => "accept-language",
            Self::Authorization(_) => "authorization",
            Self::Expect(_) => "expect",
            Self::From(_) => "from",
            Self::Host(_) => "host",
            Self::IfMatch(_) => "if-match",
            Self::IfModifiedSince(_) => "if-modified-since",
            Self::IfNoneMatch(_) => "if-none-match",
            Self::IfRange(_) => "if-range",
            Self::IfUnmodifiedSince(_) => "if-unmodified-since",
            Self::MaxForwards(_) => "max-forwards",
            Self::ProxyAuthorization(_) => "proxy-authorization",
            Self::Range(_) => "range",
            Self::Referer(_) => "referer",
            Self::TE(_) => "te",
            Self::UserAgent(_) => "user-agent",
        }
    }
}

impl FromMessageHeader for RequestHeader {
    fn can_convert(eh: &MessageHeader) -> bool {
        let name = eh.name.as_str();
        name == "accept"
            || name == "accept-charset"
            || name == "accept-encoding"
            || name == "accept-language"
            || name == "authorization"
            || name == "expect"
            || name == "from"
            || name == "host"
            || name == "if-match"
            || name == "if-modified-since"
            || name == "if-none-match"
            || name == "if-range"
            || name == "if-unmodified-since"
            || name == "max-forwards"
            || name == "proxy-authorization"
            || name == "range"
            || name == "referer"
            || name == "te"
            || name == "user-agent"
    }
    fn from_extension_header(eh: MessageHeader) -> ParseResult<(String, Self)> {
        let val = eh.value;
        let name = eh.name.as_str();
        let header = match name {
            "accept" => Self::Accept(val),
            "accept-charset" => Self::AcceptCharset(val),
            "accept-encoding" => Self::AcceptEncoding(val),
            "accept-language" => Self::AcceptLanguage(val),
            "authorization" => Self::Authorization(val),
            "expect" => Self::Expect(val),
            "from" => Self::From(val),
            "host" => Self::Host(val),
            "if-match" => Self::IfMatch(val),
            "if-modified-since" => Self::IfModifiedSince(val),
            "if-none-match" => Self::IfNoneMatch(val),
            "if-range" => Self::IfRange(val),
            "if-unmodified-since" => Self::IfUnmodifiedSince(val),
            "max-forwards" => Self::MaxForwards(val),
            "proxy-authorization" => Self::ProxyAuthorization(val),
            "range" => Self::Range(val),
            "referer" => Self::Referer(val),
            "te" => Self::TE(val),
            "user-agent" => Self::UserAgent(val),
            _ => unreachable!(
                "Failed to convert extension header. Perhaps can_convert was not checked"
            ),
        };

        Ok((eh.name, header))
    }
}

/// Based on RFC 2616 section 5
///
/// # Augmented Backus-Naur Form
/// ```text
///        Request       = Request-Line              ; Section 5.1
///                        *(( general-header        ; Section 4.5
///                         | request-header         ; Section 5.3
///                         | entity-header ) CRLF)  ; Section 7.1
///                        CRLF
///                        [ message-body ]          ; Section 4.3
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum RequestHeaderType {
    EntityHeader(EntityHeader),
    ExtensionHeader(String),
    GeneralHeader(GeneralHeader),
    RequestHeader(RequestHeader),
}

/// Abstraction used to take ownership of name to be held in header hashmap
#[derive(Debug, PartialEq, Eq)]
pub struct RequestHeaderMap {
    name: String,
    ty: RequestHeaderType,
}

impl RequestHeaderMap {
    pub fn extract_name_type(self) -> (String, RequestHeaderType) {
        (self.name, self.ty)
    }
}

impl<R: Read> Parsable<R> for RequestHeaderMap {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let header = MessageHeader::parse(parser);
        let header = header?;
        if GeneralHeader::can_convert(&header) {
            let (name, header) = header.into_header()?;
            Ok(Self {
                name,
                ty: RequestHeaderType::GeneralHeader(header),
            })
        } else if RequestHeader::can_convert(&header) {
            let (name, header) = header.into_header()?;
            Ok(Self {
                name,
                ty: RequestHeaderType::RequestHeader(header),
            })
        } else if EntityHeader::can_convert(&header) {
            let (name, header) = header.into_header()?;
            Ok(Self {
                name,
                ty: RequestHeaderType::EntityHeader(header),
            })
        } else {
            let (name, value) = header.extract_name_val();
            Ok(Self {
                name,
                ty: RequestHeaderType::ExtensionHeader(value),
            })
        }
    }
}

pub type RequestHeaders = BTreeMap<String, RequestHeaderType>;
#[derive(Debug, PartialEq, Eq)]
pub enum RequestBody {
    FormData(BTreeMap<String, String>),
    Plain(String),
    Empty,
}

/// Based on RFC 2616 section 5
///
/// # Augmented Backus-Naur Form
/// ```text
///        Request       = Request-Line              ; Section 5.1
///                        *(( general-header        ; Section 4.5
///                         | request-header         ; Section 5.3
///                         | entity-header ) CRLF)  ; Section 7.1
///                        CRLF
///                        [ message-body ]          ; Section 4.3
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub path: URIPath,
    pub query: RequestQuery,
    pub http_version: HTTPVersion,
    pub headers: RequestHeaders,
    pub body: RequestBody,
}

pub type RequestTuple = (
    Method,
    URIPath,
    RequestQuery,
    HTTPVersion,
    BTreeMap<String, RequestHeaderType>,
    RequestBody,
);

impl Request {
    pub fn to_request_tuple(self) -> RequestTuple {
        (
            self.method,
            self.path,
            self.query,
            self.http_version,
            self.headers,
            self.body,
        )
    }

    pub fn from_request_tuple(r: RequestTuple) -> Self {
        Request {
            method: r.0,
            path: r.1,
            query: r.2,
            http_version: r.3,
            headers: r.4,
            body: r.5,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &URIPath {
        &self.path
    }

    pub fn method_path(&self) -> (&Method, &str) {
        (&self.method, self.path.entire_path().as_str())
    }
}

impl<R: Read> Parsable<R> for Request {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let method = Method::parse(parser)?;
        parser.skip_whitespace();
        let path = URIPath::parse(parser)?;
        let query = if parser.matches(|c| c == b'?') {
            parser.consume();
            RequestQuery::parse(parser)?
        } else {
            RequestQuery::default()
        };
        parser.skip_whitespace();
        let http_version = HTTPVersion::parse(parser)?;
        parser.skip_whitespace();
        parser.expect_crlf()?;

        let mut headers = BTreeMap::new();
        let mut body_len = None;

        while let Ok(header) = RequestHeaderMap::parse(parser) {
            let (name, ty) = header.extract_name_type();
            if let RequestHeaderType::EntityHeader(EntityHeader::ContentLength(len)) = ty {
                body_len = Some(len)
            }
            headers.insert(name, ty);
        }

        let body = match body_len {
            Some(body_len) => {
                parser.expect_crlf()?;
                // eprintln!("{}", parser.peek().unwrap() as char);
                // parser.consume_or_err(|c| c == b'\n')?;
                // eprintln!("hit");
                RequestBody::Plain(parser.consume_n(body_len))
            }
            None => RequestBody::Empty,
        };

        Ok(Request {
            method,
            path,
            query,
            http_version,
            headers,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::steadfast_parsing::StrParser;

    use super::*;

    #[test]
    fn test_methods() {
        let mut parser = StrParser::from_str("GET");
        assert_eq!(Method::parse(&mut parser), Ok(Method::Get));
        let mut parser = StrParser::from_str("POST");
        assert_eq!(Method::parse(&mut parser), Ok(Method::Post));
    }

    #[test]
    fn test_http_version() {
        let mut parser = StrParser::from_str("HTTP/1.1");
        assert_eq!(
            HTTPVersion::parse(&mut parser),
            Ok(HTTPVersion { major: 1, minor: 1 })
        );
    }

    #[test]
    fn test_request() {
        let mut parser = StrParser::from_str("/somepath");
        let path = URIPath::parse(&mut parser).unwrap();
        let mut parser = StrParser::from_str("some=query"); // first '?' is consumed in actual request parsing
        let query = RequestQuery::parse(&mut parser).unwrap();

        let mut parser = StrParser::from_str(
            "GET /somepath?some=query HTTP/1.1\r\nHost: 127.0.0.1:8000\r\nUser-Agent: curl/8.14.1\r\nAccept: */*",
        );
        let mut headers = BTreeMap::new();
        headers.insert(
            String::from("host"),
            RequestHeaderType::RequestHeader(RequestHeader::Host(String::from("127.0.0.1:8000"))),
        );
        headers.insert(
            String::from("user-agent"),
            RequestHeaderType::RequestHeader(RequestHeader::UserAgent(String::from("curl/8.14.1"))),
        );
        headers.insert(
            String::from("accept"),
            RequestHeaderType::RequestHeader(RequestHeader::Accept(String::from("*/*"))),
        );
        assert_eq!(
            Request::parse(&mut parser),
            Ok(Request {
                method: Method::Get,
                path,
                query: query,
                http_version: HTTPVersion { major: 1, minor: 1 },
                headers,
                body: RequestBody::Empty
            })
        );
    }
    #[test]
    fn test_request_body() {
        let mut parser = StrParser::from_str("/somepath");
        let path = URIPath::parse(&mut parser).unwrap();
        let mut parser = StrParser::from_str("some=query"); // first '?' is consumed in actual request parsing
        let query = RequestQuery::parse(&mut parser).unwrap();

        let mut parser = StrParser::from_str(
            "GET /somepath?some=query HTTP/1.1\r\nHost: 127.0.0.1:8000\r\nUser-Agent: curl/8.14.1\r\nContent-Length: 14\r\nAccept: */*\r\n\r\nthis is a test    ",
        );
        let mut headers = BTreeMap::new();
        headers.insert(
            String::from("host"),
            RequestHeaderType::RequestHeader(RequestHeader::Host(String::from("127.0.0.1:8000"))),
        );
        headers.insert(
            String::from("user-agent"),
            RequestHeaderType::RequestHeader(RequestHeader::UserAgent(String::from("curl/8.14.1"))),
        );
        headers.insert(
            String::from("content-length"),
            RequestHeaderType::EntityHeader(EntityHeader::ContentLength(14)),
        );
        headers.insert(
            String::from("accept"),
            RequestHeaderType::RequestHeader(RequestHeader::Accept(String::from("*/*"))),
        );
        assert_eq!(
            Request::parse(&mut parser),
            Ok(Request {
                method: Method::Get,
                path,
                query: query,
                http_version: HTTPVersion { major: 1, minor: 1 },
                headers,
                body: RequestBody::Plain(String::from("this is a test"))
            })
        );
    }
}
