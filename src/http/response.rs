use super::{
    EntityHeader, FromMessageHeader, GeneralHeader, HTTPVersion, MessageHeader, ToMessageHeader,
};
use crate::steadfast_parsing::prelude::*;
use crate::stream_writer::{StreamResult, StreamWritable};
use std::{collections::HashMap, io::Read};

/// Based on RFC 2616 section 6.1.1
///
/// - **1xx**: Informational - Request received, continuing process
/// - **2xx**: Success - The action was successfully received, understood, and accepted
/// - **3xx**: Redirection - Further action must be taken in order to complete the request
/// - **4xx**: Client Error - The request contains bad syntax or cannot be fulfilled
/// - **5xx**: Server Error - The server failed to fulfill an apparently valid request
///
/// # Augmented Backus-Naur Form
/// ```text
///
/// Status-Code    =
///             "100"  ; Section 10.1.1: Continue
///           | "101"  ; Section 10.1.2: Switching Protocols
///           | "200"  ; Section 10.2.1: OK
///           | "201"  ; Section 10.2.2: Created
///           | "202"  ; Section 10.2.3: Accepted
///           | "203"  ; Section 10.2.4: Non-Authoritative Information
///           | "204"  ; Section 10.2.5: No Content
///           | "205"  ; Section 10.2.6: Reset Content
///           | "206"  ; Section 10.2.7: Partial Content
///           | "300"  ; Section 10.3.1: Multiple Choices
///           | "301"  ; Section 10.3.2: Moved Permanently
///           | "302"  ; Section 10.3.3: Found
///           | "303"  ; Section 10.3.4: See Other
///           | "304"  ; Section 10.3.5: Not Modified
///           | "305"  ; Section 10.3.6: Use Proxy
///           | "307"  ; Section 10.3.8: Temporary Redirect
///           | "400"  ; Section 10.4.1: Bad Request
///           | "401"  ; Section 10.4.2: Unauthorized
///           | "402"  ; Section 10.4.3: Payment Required
///           | "403"  ; Section 10.4.4: Forbidden
///           | "404"  ; Section 10.4.5: Not Found
///           | "405"  ; Section 10.4.6: Method Not Allowed
///           | "406"  ; Section 10.4.7: Not Acceptable
///           | "407"  ; Section 10.4.8: Proxy Authentication Required
///           | "408"  ; Section 10.4.9: Request Time-out
///           | "409"  ; Section 10.4.10: Conflict
///           | "410"  ; Section 10.4.11: Gone
///           | "411"  ; Section 10.4.12: Length Required
///           | "412"  ; Section 10.4.13: Precondition Failed
///           | "413"  ; Section 10.4.14: Request Entity Too Large
///           | "414"  ; Section 10.4.15: Request-URI Too Large
///           | "415"  ; Section 10.4.16: Unsupported Media Type
///           | "416"  ; Section 10.4.17: Requested range not satisfiable
///           | "417"  ; Section 10.4.18: Expectation Failed
///           | "500"  ; Section 10.5.1: Internal Server Error
///           | "501"  ; Section 10.5.2: Not Implemented
///           | "502"  ; Section 10.5.3: Bad Gateway
///           | "503"  ; Section 10.5.4: Service Unavailable
///           | "504"  ; Section 10.5.5: Gateway Time-out
///           | "505"  ; Section 10.5.6: HTTP Version not supported
///           | extension-code
///
///       extension-code = 3DIGIT
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusCode {
    Continue,                     // "100"  ; Section 10.1.1:
    SwitchingProtocols,           // "101"  ; Section 10.1.2:
    OK,                           // "200"  ; Section 10.2.1:
    Created,                      // "201"  ; Section 10.2.2:
    Accepted,                     // "202"  ; Section 10.2.3:
    NonAuthoritativeInformation,  // "203"  ; Section 10.2.4:
    NoContent,                    // "204"  ; Section 10.2.5:
    ResetContent,                 // "205"  ; Section 10.2.6:
    PartialContent,               // "206"  ; Section 10.2.7:
    MultipleChoices,              // "300"  ; Section 10.3.1:
    MovedPermanently,             // "301"  ; Section 10.3.2:
    Found,                        // "302"  ; Section 10.3.3:
    SeeOther,                     // "303"  ; Section 10.3.4:
    NotModified,                  // "304"  ; Section 10.3.5:
    UseProxy,                     // "305"  ; Section 10.3.6:
    TemporaryRedirect,            // "307"  ; Section 10.3.8:
    BadRequest,                   // "400"  ; Section 10.4.1:
    Unauthorized,                 // "401"  ; Section 10.4.2:
    PaymentRequired,              // "402"  ; Section 10.4.3:
    Forbidden,                    // "403"  ; Section 10.4.4:
    NotFound,                     // "404"  ; Section 10.4.5:
    MethodNotAllowed,             // "405"  ; Section 10.4.6:
    NotAcceptable,                // "406"  ; Section 10.4.7:
    ProxyAuthenticationRequired,  // "407"  ; Section 10.4.8:
    RequestTimeout,               // "408"  ; Section 10.4.9:
    Conflict,                     // "409"  ; Section 10.4.10:
    Gone,                         // "410"  ; Section 10.4.11:
    LengthRequired,               // "411"  ; Section 10.4.12:
    PreconditionFailed,           // "412"  ; Section 10.4.13:
    RequestEntityTooLarge,        // "413"  ; Section 10.4.14:
    RequestUriTooLarge,           // "414"  ; Section 10.4.15:
    UnsupportedMediaType,         // "415"  ; Section 10.4.16:
    RequestedRangeNotSatisfiable, // "416"  ; Section 10.4.17:
    ExpectationFailed,            // "417"  ; Section 10.4.18:
    InternalServerError,          // "500"  ; Section 10.5.1:
    NotImplemented,               // "501"  ; Section 10.5.2:
    BadGateway,                   // "502"  ; Section 10.5.3:
    ServiceUnavailable,           // "503"  ; Section 10.5.4:
    GatewayTimeout,               // "504"  ; Section 10.5.5:
    HTTPVersionNotSupported,      // "505"  ; Section 10.5.6:
    ExtensionCode(u16),
}

impl StatusCode {
    // TODO: add better result for extensions
    pub const fn as_str(&self) -> Option<&str> {
        match self {
            Self::Continue => Some("100"),           // "100"  ; Section 10.1.1:
            Self::SwitchingProtocols => Some("101"), // "101"  ; Section 10.1.2:
            Self::OK => Some("200"),                 // "200"  ; Section 10.2.1:
            Self::Created => Some("201"),            // "201"  ; Section 10.2.2:
            Self::Accepted => Some("202"),           // "202"  ; Section 10.2.3:
            Self::NonAuthoritativeInformation => Some("203"), // "203"  ; Section 10.2.4:
            Self::NoContent => Some("204"),          // "204"  ; Section 10.2.5:
            Self::ResetContent => Some("205"),       // "205"  ; Section 10.2.6:
            Self::PartialContent => Some("206"),     // "206"  ; Section 10.2.7:
            Self::MultipleChoices => Some("300"),    // "300"  ; Section 10.3.1:
            Self::MovedPermanently => Some("301"),   // "301"  ; Section 10.3.2:
            Self::Found => Some("302"),              // "302"  ; Section 10.3.3:
            Self::SeeOther => Some("303"),           // "303"  ; Section 10.3.4:
            Self::NotModified => Some("304"),        // "304"  ; Section 10.3.5:
            Self::UseProxy => Some("305"),           // "305"  ; Section 10.3.6:
            Self::TemporaryRedirect => Some("307"),  // "307"  ; Section 10.3.8:
            Self::BadRequest => Some("400"),         // "400"  ; Section 10.4.1:
            Self::Unauthorized => Some("401"),       // "401"  ; Section 10.4.2:
            Self::PaymentRequired => Some("402"),    // "402"  ; Section 10.4.3:
            Self::Forbidden => Some("403"),          // "403"  ; Section 10.4.4:
            Self::NotFound => Some("404"),           // "404"  ; Section 10.4.5:
            Self::MethodNotAllowed => Some("405"),   // "405"  ; Section 10.4.6:
            Self::NotAcceptable => Some("406"),      // "406"  ; Section 10.4.7:
            Self::ProxyAuthenticationRequired => Some("407"), // "407"  ; Section 10.4.8:
            Self::RequestTimeout => Some("408"),     // "408"  ; Section 10.4.9:
            Self::Conflict => Some("409"),           // "409"  ; Section 10.4.10:
            Self::Gone => Some("410"),               // "410"  ; Section 10.4.11:
            Self::LengthRequired => Some("411"),     // "411"  ; Section 10.4.12:
            Self::PreconditionFailed => Some("412"), // "412"  ; Section 10.4.13:
            Self::RequestEntityTooLarge => Some("413"), // "413"  ; Section 10.4.14:
            Self::RequestUriTooLarge => Some("414"), // "414"  ; Section 10.4.15:
            Self::UnsupportedMediaType => Some("415"), // "415"  ; Section 10.4.16:
            Self::RequestedRangeNotSatisfiable => Some("416"), // "416"  ; Section 10.4.17:
            Self::ExpectationFailed => Some("417"),  // "417"  ; Section 10.4.18:
            Self::InternalServerError => Some("500"), // "500"  ; Section 10.5.1:
            Self::NotImplemented => Some("501"),     // "501"  ; Section 10.5.2:
            Self::BadGateway => Some("502"),         // "502"  ; Section 10.5.3:
            Self::ServiceUnavailable => Some("503"), // "503"  ; Section 10.5.4:
            Self::GatewayTimeout => Some("504"),     // "504"  ; Section 10.5.5:
            Self::HTTPVersionNotSupported => Some("505"), // "505"  ; Section 10.5.6:
            Self::ExtensionCode(_) => None,
        }
    }
    pub const fn from_code(n: u16) -> Result<Self, ParseErr> {
        match n {
            100 => Ok(Self::Continue),                     // "100"  ; Section 10.1.1:
            101 => Ok(Self::SwitchingProtocols),           // "101"  ; Section 10.1.2:
            200 => Ok(Self::OK),                           // "200"  ; Section 10.2.1:
            201 => Ok(Self::Created),                      // "201"  ; Section 10.2.2:
            202 => Ok(Self::Accepted),                     // "202"  ; Section 10.2.3:
            203 => Ok(Self::NonAuthoritativeInformation),  // "203"  ; Section 10.2.4:
            204 => Ok(Self::NoContent),                    // "204"  ; Section 10.2.5:
            205 => Ok(Self::ResetContent),                 // "205"  ; Section 10.2.6:
            206 => Ok(Self::PartialContent),               // "206"  ; Section 10.2.7:
            300 => Ok(Self::MultipleChoices),              // "300"  ; Section 10.3.1:
            301 => Ok(Self::MovedPermanently),             // "301"  ; Section 10.3.2:
            302 => Ok(Self::Found),                        // "302"  ; Section 10.3.3:
            303 => Ok(Self::SeeOther),                     // "303"  ; Section 10.3.4:
            304 => Ok(Self::NotModified),                  // "304"  ; Section 10.3.5:
            305 => Ok(Self::UseProxy),                     // "305"  ; Section 10.3.6:
            307 => Ok(Self::TemporaryRedirect),            // "307"  ; Section 10.3.8:
            400 => Ok(Self::BadRequest),                   // "400"  ; Section 10.4.1:
            401 => Ok(Self::Unauthorized),                 // "401"  ; Section 10.4.2:
            402 => Ok(Self::PaymentRequired),              // "402"  ; Section 10.4.3:
            403 => Ok(Self::Forbidden),                    // "403"  ; Section 10.4.4:
            404 => Ok(Self::NotFound),                     // "404"  ; Section 10.4.5:
            405 => Ok(Self::MethodNotAllowed),             // "405"  ; Section 10.4.6:
            406 => Ok(Self::NotAcceptable),                // "406"  ; Section 10.4.7:
            407 => Ok(Self::ProxyAuthenticationRequired),  // "407"  ; Section 10.4.8:
            408 => Ok(Self::RequestTimeout),               // "408"  ; Section 10.4.9:
            409 => Ok(Self::Conflict),                     // "409"  ; Section 10.4.10:
            410 => Ok(Self::Gone),                         // "410"  ; Section 10.4.11:
            411 => Ok(Self::LengthRequired),               // "411"  ; Section 10.4.12:
            412 => Ok(Self::PreconditionFailed),           // "412"  ; Section 10.4.13:
            413 => Ok(Self::RequestEntityTooLarge),        // "413"  ; Section 10.4.14:
            414 => Ok(Self::RequestUriTooLarge),           // "414"  ; Section 10.4.15:
            415 => Ok(Self::UnsupportedMediaType),         // "415"  ; Section 10.4.16:
            416 => Ok(Self::RequestedRangeNotSatisfiable), // "416"  ; Section 10.4.17:
            417 => Ok(Self::ExpectationFailed),            // "417"  ; Section 10.4.18:
            500 => Ok(Self::InternalServerError),          // "500"  ; Section 10.5.1:
            501 => Ok(Self::NotImplemented),               // "501"  ; Section 10.5.2:
            502 => Ok(Self::BadGateway),                   // "502"  ; Section 10.5.3:
            503 => Ok(Self::ServiceUnavailable),           // "503"  ; Section 10.5.4:
            504 => Ok(Self::GatewayTimeout),               // "504"  ; Section 10.5.5:
            505 => Ok(Self::HTTPVersionNotSupported),      // "505"  ; Section 10.5.6:
            n if n < 1000 => {
                Ok(Self::ExtensionCode(n)) // this should technically be under 999
            }
            n => {
                Err(ParseErr::InvalidStatusCode { found: n }) // this should technically be under 999
            }
        }
    }
}
impl<R: Read> Parsable<R> for StatusCode {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let mut num = [0; 3];
        for i in 0..3 {
            match parser.peek() {
                Some(c) => {
                    if c.is_ascii_digit() {
                        parser.consume();
                        num[i] = c;
                    } else {
                        return Err(ParseErr::InvalidStatusCodeStr { found: c });
                    }
                }
                None => return Err(ParseErr::ExpectedStatusCode),
            }
        }

        match &num {
            b"100" => Ok(Self::Continue),           // "100"  ; Section 10.1.1:
            b"101" => Ok(Self::SwitchingProtocols), // "101"  ; Section 10.1.2:
            b"200" => Ok(Self::OK),                 // "200"  ; Section 10.2.1:
            b"201" => Ok(Self::Created),            // "201"  ; Section 10.2.2:
            b"202" => Ok(Self::Accepted),           // "202"  ; Section 10.2.3:
            b"203" => Ok(Self::NonAuthoritativeInformation), // "203"  ; Section 10.2.4:
            b"204" => Ok(Self::NoContent),          // "204"  ; Section 10.2.5:
            b"205" => Ok(Self::ResetContent),       // "205"  ; Section 10.2.6:
            b"206" => Ok(Self::PartialContent),     // "206"  ; Section 10.2.7:
            b"300" => Ok(Self::MultipleChoices),    // "300"  ; Section 10.3.1:
            b"301" => Ok(Self::MovedPermanently),   // "301"  ; Section 10.3.2:
            b"302" => Ok(Self::Found),              // "302"  ; Section 10.3.3:
            b"303" => Ok(Self::SeeOther),           // "303"  ; Section 10.3.4:
            b"304" => Ok(Self::NotModified),        // "304"  ; Section 10.3.5:
            b"305" => Ok(Self::UseProxy),           // "305"  ; Section 10.3.6:
            b"307" => Ok(Self::TemporaryRedirect),  // "307"  ; Section 10.3.8:
            b"400" => Ok(Self::BadRequest),         // "400"  ; Section 10.4.1:
            b"401" => Ok(Self::Unauthorized),       // "401"  ; Section 10.4.2:
            b"402" => Ok(Self::PaymentRequired),    // "402"  ; Section 10.4.3:
            b"403" => Ok(Self::Forbidden),          // "403"  ; Section 10.4.4:
            b"404" => Ok(Self::NotFound),           // "404"  ; Section 10.4.5:
            b"405" => Ok(Self::MethodNotAllowed),   // "405"  ; Section 10.4.6:
            b"406" => Ok(Self::NotAcceptable),      // "406"  ; Section 10.4.7:
            b"407" => Ok(Self::ProxyAuthenticationRequired), // "407"  ; Section 10.4.8:
            b"408" => Ok(Self::RequestTimeout),     // "408"  ; Section 10.4.9:
            b"409" => Ok(Self::Conflict),           // "409"  ; Section 10.4.10:
            b"410" => Ok(Self::Gone),               // "410"  ; Section 10.4.11:
            b"411" => Ok(Self::LengthRequired),     // "411"  ; Section 10.4.12:
            b"412" => Ok(Self::PreconditionFailed), // "412"  ; Section 10.4.13:
            b"413" => Ok(Self::RequestEntityTooLarge), // "413"  ; Section 10.4.14:
            b"414" => Ok(Self::RequestUriTooLarge), // "414"  ; Section 10.4.15:
            b"415" => Ok(Self::UnsupportedMediaType), // "415"  ; Section 10.4.16:
            b"416" => Ok(Self::RequestedRangeNotSatisfiable), // "416"  ; Section 10.4.17:
            b"417" => Ok(Self::ExpectationFailed),  // "417"  ; Section 10.4.18:
            b"500" => Ok(Self::InternalServerError), // "500"  ; Section 10.5.1:
            b"501" => Ok(Self::NotImplemented),     // "501"  ; Section 10.5.2:
            b"502" => Ok(Self::BadGateway),         // "502"  ; Section 10.5.3:
            b"503" => Ok(Self::ServiceUnavailable), // "503"  ; Section 10.5.4:
            b"504" => Ok(Self::GatewayTimeout),     // "504"  ; Section 10.5.5:
            b"505" => Ok(Self::HTTPVersionNotSupported), // "505"  ; Section 10.5.6:
            n => {
                match u16::from_str_radix(str::from_utf8(n).map_err(|_| ParseErr::InvalidUTF8)?, 10)
                {
                    Ok(n) => Ok(Self::ExtensionCode(n)),
                    Err(_) => Err(ParseErr::ExpectedStatusCode),
                }
            }
        }
    }
}

impl<W: std::io::Write> StreamWritable<W> for StatusCode {
    fn write_to_stream(self, stream: &mut W) -> StreamResult {
        if let Some(s) = self.as_str() {
            write!(stream, "{}", s)?;
        }
        Ok(())
    }
}
/// Based on rfc2616 Section 6.2
///
/// # Augmented Backus-Naur Form
/// ```text
///       response-header = Accept-Ranges           ; Section 14.5
///                       | Age                     ; Section 14.6
///                       | ETag                    ; Section 14.19
///                       | Location                ; Section 14.30
///                       | Proxy-Authenticate      ; Section 14.33
///                       | Retry-After             ; Section 14.37
///                       | Server                  ; Section 14.38
///                       | Vary                    ; Section 14.44
///                       | WWW-Authenticate        ; Section 14.47
/// ``
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResponseHeader {
    AcceptRanges(String),      // Section 14.5
    Age(String),               // Section 14.6
    ETag(String),              // Section 14.19
    Location(String),          // Section 14.30
    ProxyAuthenticate(String), // Section 14.33
    RetryAfter(String),        // Section 14.37
    Server(String),            // Section 14.38
    Vary(String),              // Section 14.44
    WWWAuthenticate(String),   // Section 14.47
}

impl ResponseHeader {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::AcceptRanges(_) => "accept-ranges", // Section 14.5
            Self::Age(_) => "age",                    // Section 14.6
            Self::ETag(_) => "etag",                  // Section 14.19
            Self::Location(_) => "location",          // Section 14.30
            Self::ProxyAuthenticate(_) => "proxy-authenticate", // Section 14.33
            Self::RetryAfter(_) => "retry-after",     // Section 14.37
            Self::Server(_) => "server",              // Section 14.38
            Self::Vary(_) => "vary",                  // Section 14.44
            Self::WWWAuthenticate(_) => "www-authenticate", // Section 14.47
        }
    }
}

impl FromMessageHeader for ResponseHeader {
    fn can_convert(eh: &MessageHeader) -> bool {
        let name = eh.name.as_str();
        name== "accept-ranges" // Section 14.5
            ||name== "age"                    // Section 14.6
            ||name== "etag"                  // Section 14.19
            ||name== "location"          // Section 14.30
            ||name== "proxy-authenticate" // Section 14.33
            ||name== "retry-after"     // Section 14.37
            ||name== "server"              // Section 14.38
            ||name== "vary"                  // Section 14.44
            ||name== "www-authenticate" // Section 14.47
    }

    fn from_extension_header(eh: MessageHeader) -> ParseResult<(String, Self)> {
        let val = eh.value;
        let name = eh.name.as_str();
        let header = match name {
            "accept-ranges" => Self::AcceptRanges(val), // Section 14.5
            "age" => Self::Age(val),                    // Section 14.6
            "etag" => Self::ETag(val),                  // Section 14.19
            "location" => Self::Location(val),          // Section 14.30
            "proxy-authenticate" => Self::ProxyAuthenticate(val), // Section 14.33
            "retry-after" => Self::RetryAfter(val),     // Section 14.37
            "server" => Self::Server(val),              // Section 14.38
            "vary" => Self::Vary(val),                  // Section 14.44
            "www-authenticate" => Self::WWWAuthenticate(val), // Section 14.47
            _ => unreachable!(
                "Failed to convert extension header. Perhaps can_convert was not checked"
            ),
        };

        Ok((eh.name, header))
    }
}

impl ToMessageHeader for ResponseHeader {
    fn consume_value_as_string(self) -> String {
        match self {
            Self::AcceptRanges(s) => s,      // Section 14.5
            Self::Age(s) => s,               // Section 14.6
            Self::ETag(s) => s,              // Section 14.19
            Self::Location(s) => s,          // Section 14.30
            Self::ProxyAuthenticate(s) => s, // Section 14.33
            Self::RetryAfter(s) => s,        // Section 14.37
            Self::Server(s) => s,            // Section 14.38
            Self::Vary(s) => s,              // Section 14.44
            Self::WWWAuthenticate(s) => s,   // Section 14.47
        }
    }
    fn to_msg_header(self) -> MessageHeader {
        let name = self.name().to_string();
        let value = self.consume_value_as_string();

        MessageHeader { name, value }
    }
}
/// Based on RFC 2616 section 6.1
///
/// # Augmented Backus-Naur Form
/// ```text
/// Reason-Phrase  = *<TEXT, excluding CR, LF>
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReasonPhrase(String);

impl<R: Read> Parsable<R> for ReasonPhrase {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let reason = parser
            .consume_while(|p| p.is_text() && p.peek().is_some_and(|c| c != b'\r' && c != b'\n'));

        Ok(ReasonPhrase(reason))
    }
}
impl<W: std::io::Write> StreamWritable<W> for ReasonPhrase {
    fn write_to_stream(self, stream: &mut W) -> StreamResult {
        write!(stream, "{}", self.0)?;
        Ok(())
    }
}

/// Based on RFC 2616 section 6.1
///
/// # Augmented Backus-Naur Form
/// ```text
/// Status-Line = HTTP-Version SP Status-Code SP Reason-Phrase CRLF
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusLine {
    http_version: HTTPVersion,
    status_code: StatusCode,
    reason_phrase: ReasonPhrase,
}

impl StatusLine {
    pub fn new(
        http_version: HTTPVersion,
        status_code: StatusCode,
        reason_phrase: ReasonPhrase,
    ) -> Self {
        StatusLine {
            http_version,
            status_code,
            reason_phrase,
        }
    }
    pub fn new_simple(status_code: StatusCode) -> Self {
        StatusLine {
            http_version: HTTPVersion::default(),
            status_code,
            reason_phrase: ReasonPhrase(String::new()),
        }
    }
}

impl<R: Read> Parsable<R> for StatusLine {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let http_version = HTTPVersion::parse(parser)?;
        parser.skip_whitespace();
        let status_code = StatusCode::parse(parser)?;
        parser.skip_whitespace();
        let reason_phrase = ReasonPhrase::parse(parser)?;

        Ok(StatusLine {
            http_version,
            status_code,
            reason_phrase,
        })
    }
}
impl<W: std::io::Write> StreamWritable<W> for StatusLine {
    fn write_to_stream(self, stream: &mut W) -> StreamResult {
        self.http_version.write_to_stream(stream)?;
        write!(stream, " ")?;
        self.status_code.write_to_stream(stream)?;
        if !self.reason_phrase.0.is_empty() {
            write!(stream, " ")?;
            self.reason_phrase.write_to_stream(stream)?;
        }
        write!(stream, "\r\n")?;

        Ok(())
    }
}

/// Based on RFC 2616 section 6
///
/// # Augmented Backus-Naur Form
/// ```text
///      Response       = Status-Line               ; Section 6.1
///                       *(( general-header        ; Section 4.5
///                        | response-header        ; Section 6.2
///                        | entity-header ) CRLF)  ; Section 7.1
///                       CRLF
///                       [ message-body ]          ; Section 7.2
/// ```
#[derive(Debug, PartialEq, Eq)]
pub enum ResponseHeaderType {
    GeneralHeader(GeneralHeader),
    ResponseHeader(ResponseHeader),
    EntityHeader(EntityHeader),
    ExtensionHeader { name: String, value: String },
}
impl ToMessageHeader for ResponseHeaderType {
    fn consume_value_as_string(self) -> String {
        match self {
            Self::GeneralHeader(header) => header.consume_value_as_string(),
            Self::ResponseHeader(header) => header.consume_value_as_string(),
            Self::EntityHeader(header) => header.consume_value_as_string(),
            Self::ExtensionHeader { name: _, value } => value,
        }
    }
    fn to_msg_header(self) -> MessageHeader {
        match self {
            Self::GeneralHeader(header) => header.to_msg_header(),
            Self::ResponseHeader(header) => header.to_msg_header(),
            Self::EntityHeader(header) => header.to_msg_header(),
            Self::ExtensionHeader { name, value } => MessageHeader { name, value },
        }
    }
}

/// Abstraction used to take ownership of name to be held in header hashmap
#[derive(Debug, PartialEq, Eq)]
pub struct ResponseHeaderMap {
    name: String,
    ty: ResponseHeaderType,
}

impl ResponseHeaderMap {
    pub fn from_key_val(kv_pair: (String, ResponseHeaderType)) -> Self {
        Self {
            name: kv_pair.0,
            ty: kv_pair.1,
        }
    }
    pub fn extract_name_type(self) -> (String, ResponseHeaderType) {
        (self.name, self.ty)
    }
}

impl<R: Read> Parsable<R> for ResponseHeaderMap {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let header = MessageHeader::parse(parser);
        let header = header?;
        if GeneralHeader::can_convert(&header) {
            let (name, header) = header.into_header()?;
            Ok(Self {
                name,
                ty: ResponseHeaderType::GeneralHeader(header),
            })
        } else if ResponseHeader::can_convert(&header) {
            let (name, header) = header.into_header()?;
            Ok(Self {
                name,
                ty: ResponseHeaderType::ResponseHeader(header),
            })
        } else if EntityHeader::can_convert(&header) {
            let (name, header) = header.into_header()?;
            Ok(Self {
                name,
                ty: ResponseHeaderType::EntityHeader(header),
            })
        } else {
            let (name, value) = header.extract_name_val();
            Ok(Self {
                name: name.clone(),
                ty: ResponseHeaderType::ExtensionHeader { name, value },
            })
        }
    }
}
/// Based on RFC 2616 section 6
///
/// # Augmented Backus-Naur Form
/// ```text
///      Response       = Status-Line               ; Section 6.1
///                       *(( general-header        ; Section 4.5
///                        | response-header        ; Section 6.2
///                        | entity-header ) CRLF)  ; Section 7.1
///                       CRLF
///                       [ message-body ]          ; Section 7.2
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Response {
    status_line: StatusLine,
    headers: HashMap<String, String>,
    body: Option<String>,
}

impl Response {
    pub fn new(status: StatusCode, headers: HashMap<String, String>, body: Option<String>) -> Self {
        Self {
            status_line: StatusLine::new_simple(status),
            headers,
            body,
        }
    }
    pub fn new_simple(status: StatusCode, body: Option<String>) -> Self {
        Self {
            status_line: StatusLine::new_simple(status),
            headers: HashMap::new(),
            body: body,
        }
    }
    pub fn test_response() -> Response {
        let mut headers = HashMap::new();

        let msg = String::from(
            r#"<!DOCTYPE html>
<html>
  <head>
  </head>
  <body>
    <h1>This is a test</h1>
  </body>
</html>"#,
        );
        let msg_len = msg.len();
        let (k, v) = ResponseHeaderType::EntityHeader(EntityHeader::ContentLength(msg_len))
            .to_msg_header()
            .extract_name_val();
        headers.insert(k, v);
        let (k, v) =
            ResponseHeaderType::EntityHeader(EntityHeader::ContentType(String::from("text/html")))
                .to_msg_header()
                .extract_name_val();
        headers.insert(k, v);
        Response {
            status_line: StatusLine {
                http_version: HTTPVersion { major: 1, minor: 1 },
                status_code: StatusCode::OK,
                reason_phrase: ReasonPhrase(String::new()),
            },
            headers,
            body: Some(msg),
        }
    }
}

impl<R: Read> Parsable<R> for Response {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let status_line = StatusLine::parse(parser)?;
        parser.expect_crlf()?;

        let mut headers = HashMap::new();
        let mut body_len = None;

        while let Ok(header) = ResponseHeaderMap::parse(parser) {
            if let ResponseHeaderType::EntityHeader(EntityHeader::ContentLength(len)) = header.ty {
                body_len = Some(len)
            }
            let (k, v) = header.ty.to_msg_header().extract_name_val();
            headers.insert(k, v);
        }

        let body = match body_len {
            Some(body_len) => {
                if body_len > 0 {
                    parser.expect_crlf()?;
                    // eprintln!("{}", parser.peek().unwrap() as char);
                    // parser.consume_or_err(|c| c == b'\n')?;
                    // eprintln!("hit");
                    Some(parser.consume_n(body_len))
                } else {
                    None
                }
            }
            None => None,
        };

        Ok(Response {
            status_line,
            headers,
            body,
        })
    }
}
impl<W: std::io::Write> StreamWritable<W> for Response {
    fn write_to_stream(self, stream: &mut W) -> StreamResult {
        self.status_line.write_to_stream(stream)?;
        for (name, val) in self.headers.into_iter() {
            write!(stream, "{}:{}\r\n", name, val)?;
        }
        if let Some(body) = self.body {
            write!(stream, "\r\n{}", body)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::steadfast_parsing::StrParser;

    use super::*;

    #[test]
    fn test_response() {
        let mut parser = StrParser::from_str(
            "HTTP/1.1 200\r\ndate: Tue, 30 Dec 2025 12:06:15 GMT\r\ncontent-type: text/plain; charset=UTF-8\r\ncontent-length: 0\r\n",
        );
        let mut headers = HashMap::new();
        let header = ResponseHeaderType::GeneralHeader(GeneralHeader::Date(
            "Tue, 30 Dec 2025 12:06:15 GMT".to_string(),
        ));
        let (k, v) = header.to_msg_header().extract_name_val();
        headers.insert(k, v);
        let header = ResponseHeaderType::EntityHeader(EntityHeader::ContentType(
            "text/plain; charset=UTF-8".to_string(),
        ));
        let (k, v) = header.to_msg_header().extract_name_val();
        headers.insert(k, v);
        let header = ResponseHeaderType::EntityHeader(EntityHeader::ContentLength(0));
        let (k, v) = header.to_msg_header().extract_name_val();
        headers.insert(k, v);
        assert_eq!(
            Response::parse(&mut parser),
            Ok(Response {
                status_line: StatusLine {
                    http_version: HTTPVersion { major: 1, minor: 1 },
                    status_code: StatusCode::OK,
                    reason_phrase: ReasonPhrase(String::new())
                },
                headers,
                body: None
            })
        );
    }
}
