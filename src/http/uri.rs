use crate::{
    parsing::{Parsable, ParseErr, ParseResult, Parser, StrParser},
    serializer::{DataHolder, PrimType},
};
use std::{cmp::Ordering, collections::HashMap, fmt::Display, io::Read};

/// Based on rfc3986 Section 3.1
///
/// # Augmented Backus-Naur Form
/// ```text
/// scheme      = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scheme(String);

impl Scheme {
    pub fn new(s: String) -> Self {
        Scheme(s)
    }
    pub fn from(s: &str) -> Self {
        Scheme(String::from(s))
    }
    pub fn is_valid_char(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'+' || c == b'-' || c == b'.'
    }
}

impl<R: Read> Parsable<R> for Scheme {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        if parser.matches(|c| c.is_ascii_alphabetic()) {
            Ok(Scheme(
                parser.consume_while_lower(|p| p.matches(Self::is_valid_char)),
            ))
        } else {
            Err(ParseErr::InvalidScheme)
        }
    }
}

/// Based on rfc3986 Section 2.1
///
/// # Augmented Backus-Naur Form
/// ```text
/// pct-encoded = "%" HEXDIG HEXDIG
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PctEncoding(char);
impl PctEncoding {
    pub fn from(c: char) -> Self {
        Self(c)
    }
}

impl<R: Read> Parsable<R> for PctEncoding {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        parser.consume_or_err(|c| c == b'%')?;
        let b1 = parser.consume_or_err(|c| c.is_ascii_hexdigit())?;
        let b2 = parser.consume_or_err(|c| c.is_ascii_hexdigit())?;
        let hex = format!("{}{}", b1 as char, b2 as char);
        if hex.len() == 2 {
            let radix = 16;
            let byte = u8::from_str_radix(hex.as_str(), radix)
                .map_err(|_| ParseErr::FailedToParseNum { found: hex, radix })?;
            Ok(PctEncoding(byte as char))
        } else {
            Err(ParseErr::InvalidPctEncoding { found: hex })
        }
    }
}

/// Based on rfc3986 Section 3.2.1
///
/// # Augmented Backus-Naur Form
/// ```text
/// userinfo    = *( unreserved / pct-encoded / sub-delims / ":" )
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UserInfo(String);

impl UserInfo {
    pub fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}

impl<R: Read> Parsable<R> for UserInfo {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let mut s = String::new();
        let mut valid = false;
        while let Some(c) = parser.peek() {
            if URI::is_unreserved(c) || URI::is_sub_delim(c) || c == b':' {
                s.push(c as char);
                parser.consume();
            } else if c == b'%' {
                let pct_encoding = PctEncoding::parse(parser)?;
                s.push(pct_encoding.0);
            } else if c == b'@' {
                parser.consume();
                valid = true;
                break;
            } else {
                return Err(ParseErr::NotUserInfo { presumed_host: s });
            }
        }
        if valid {
            Ok(UserInfo(s))
        } else {
            Err(ParseErr::InvalidUserInfo)
        }
    }
}

/// Based on rfc3986 Section 3.2.2
///
/// # Augmented Backus-Naur Form
/// ```text
/// userinfo    = *( unreserved / pct-encoded / sub-delims / ":" )
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Host {
    IPvFuture(String),
    IPv6(String),
    IPv4((u8, u8, u8, u8)),
    Domain(String),
}

impl Host {
    fn parse_ip_lit<R: Read>(_parser: &mut Parser<R>) -> ParseResult<Self> {
        unimplemented!("Haven't worked on ipv6 yet, sorry")
        // parser.consume_or_err(|c| c == b'[')?;
        // parser.consume_or_err(|c| c == b']')?;
    }

    fn ipv4_from_str(s: &str) -> ParseResult<Self> {
        let mut nums = Vec::new();
        let mut s_num = String::new();
        for c in s.chars() {
            if c == '.' {
                if let Ok(num) = u8::from_str_radix(s_num.as_str(), 10) {
                    nums.push(num);
                    s_num = String::new();
                } else {
                    return Err(ParseErr::InvalidIPv4Num { found: s_num });
                }
            } else if c.is_ascii_digit() {
                s_num.push(c);
            } else {
                return Err(ParseErr::InvalidIPv4Char { found: c });
            }
        }

        if let Ok(num) = u8::from_str_radix(s_num.as_str(), 10) {
            nums.push(num);
        } else {
            return Err(ParseErr::InvalidIPv4Num { found: s_num });
        }

        if nums.len() == 4 {
            Ok(Self::IPv4((nums[0], nums[1], nums[2], nums[3])))
        } else {
            Err(ParseErr::InvalidIPv4Len { found: nums.len() })
        }
    }

    fn parse_ipv4_or_domain<R: Read>(parser: &mut Parser<R>) -> ParseResult<Self> {
        let mut s = String::new();

        let mut is_ipv4 = true;
        while let Some(c) = parser.peek() {
            if URI::is_unreserved(c) {
                if !c.is_ascii_digit() && c != b'.' {
                    is_ipv4 = false;
                }
                s.push((c as char).to_ascii_lowercase());
                parser.consume();
            } else if URI::is_sub_delim(c) {
                is_ipv4 = false;
                s.push(c as char);
                parser.consume();
            } else if c == b'%' {
                is_ipv4 = false;
                let pct_encoding = PctEncoding::parse(parser)?;
                s.push(pct_encoding.0.to_ascii_lowercase());
            } else {
                break;
            }
        }

        if is_ipv4 {
            Self::ipv4_from_str(s.as_str())
        } else {
            Ok(Host::Domain(s))
        }
    }
}

impl<R: Read> Parsable<R> for Host {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        if parser.matches(|c| c == b'[') {
            Self::parse_ip_lit(parser)
        } else {
            Self::parse_ipv4_or_domain(parser)
        }
    }
}

/// Based on rfc3986 Section 3.2.3
///
/// # Augmented Backus-Naur Form
/// ```text
/// port        = *DIGIT
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Port(u16);

impl<R: Read> Parsable<R> for Port {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        parser.consume_or_err(|c| c == b':')?;
        let port_num_str = parser.consume_while(|p| p.is_digit());
        let radix = 10;
        let port = u16::from_str_radix(port_num_str.as_str(), radix).map_err(|_| {
            ParseErr::FailedToParseNum {
                found: port_num_str,
                radix,
            }
        })?;
        Ok(Port(port))
    }
}

/// Based on rfc3986 Section 3.2
///
/// # Augmented Backus-Naur Form
/// ```text
/// authority   = [ userinfo "@" ] host [ ":" port ]
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Authority {
    user_info: Option<UserInfo>,
    host: Host,
    port: Option<Port>,
}

impl<R: Read> Parsable<R> for Authority {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let (user_info, host) = match UserInfo::parse(parser) {
            Ok(user_info) => {
                let host = Host::parse(parser)?;
                (Some(user_info), host)
            }
            Err(ParseErr::NotUserInfo {
                presumed_host: host_str,
            }) => {
                let mut str_parser = StrParser::from_str(host_str.as_str());
                let host = Host::parse(&mut str_parser)?;
                (None, host)
            }
            Err(e) => return Err(e),
        };
        let port = if parser.matches(|c| c == b':') {
            let port = Port::parse(parser)?;
            Some(port)
        } else {
            None
        };

        Ok(Authority {
            user_info,
            host,
            port,
        })
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathType {
    Relative,
    Absolute,
}

/// Based on rfc3986 Section 3.2
///
/// # Augmented Backus-Naur Form
/// ```text
/// path          = path-abempty    ; begins with "/" or is empty
///   / path-absolute   ; begins with "/" but not "//"
///   / path-noscheme   ; begins with a non-colon segment
///   / path-rootless   ; begins with a segment
///   / path-empty      ; zero characters
///
/// path-abempty  = *( "/" segment )
/// path-absolute = "/" [ segment-nz *( "/" segment ) ]
/// path-noscheme = segment-nz-nc *( "/" segment )
/// path-rootless = segment-nz *( "/" segment )
/// path-empty    = 0<pchar>
///
/// segment       = *pchar
/// segment-nz    = 1*pchar
/// segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
/// ; non-zero-length segment without any colon ":"
///
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct URIPath {
    ty: PathType,
    segments: Vec<String>,
    entire_path: String,
}

impl URIPath {
    /// See URIPath Augmented Backus-Naur Form for justification
    fn is_valid_segment(c: u8) -> bool {
        URI::is_unreserved(c) || URI::is_sub_delim(c) || c == b':' || c == b'@'
    }

    pub fn path_type(&self) -> &PathType {
        &self.ty
    }

    pub fn entire_path(&self) -> &String {
        &self.entire_path
    }

    pub fn into_segments(self) -> Vec<String> {
        self.segments
    }
    pub fn into_entire_path(self) -> String {
        self.entire_path
    }
}

impl<R: Read> Parsable<R> for URIPath {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let mut segments = Vec::new();
        let mut entire_path = String::new();
        let ty = if parser.matches(|c| c == b'/') {
            parser.consume();
            entire_path.push('/');
            PathType::Absolute
        } else {
            PathType::Relative
        };

        let mut s = String::new();
        while let Some(c) = parser.peek() {
            if URIPath::is_valid_segment(c) {
                s.push(c as char);
                entire_path.push(c as char);
                parser.consume();
            } else if c == b'%' {
                let pct = PctEncoding::parse(parser)?;
                s.push(pct.0);
                entire_path.push(pct.0);
            } else if c == b'/' {
                entire_path.push('/');
                segments.push(s);
                s = String::new();
                parser.consume();
            } else {
                break;
            }
        }

        if !s.is_empty() {
            segments.push(s);
        }

        Ok(URIPath {
            ty,
            segments,
            entire_path,
        })
    }
}

/// Based on rfc3986 section 3.4
///
/// # Augmented Backus-Naur Form
/// ```text
/// query       = *( pchar / "/" / "?" )
/// ```
///
/// This struct assumes standardization of query parameters which is technically not true.
///
/// For defensive reasons, this will error if parameter is invalid, even if RFC says otherwise when accounting for more "raw" querries.
#[derive(Debug, PartialEq, Eq)]
pub struct RequestQuery {
    pub parameters: DataHolder,
}

impl Default for RequestQuery {
    fn default() -> Self {
        RequestQuery {
            parameters: DataHolder::Struct(HashMap::new()),
        }
    }
}

impl Display for RequestQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.parameters {
            DataHolder::Primitive { ty: _, val: _ } => {}
            DataHolder::Struct(ref s) => {
                for (k, v) in s.iter() {
                    match v {
                        DataHolder::Primitive { ty: _, val } => {
                            writeln!(f, "{}:{},", k, val)?;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

impl RequestQuery {
    fn sorted_keys(&self) -> Vec<&String> {
        match &self.parameters {
            DataHolder::Primitive { ty: _, val: _ } => Vec::new(),
            DataHolder::Struct(s) => {
                let mut keys: Vec<&String> = s.keys().collect();
                keys.sort();
                keys
            }
        }
    }
}

impl<R: Read> Parsable<R> for RequestQuery {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        // parser.consume_or_err(|c| c == b'?')?;
        let mut parameters = HashMap::new();

        while let Some(c) = parser.peek()
            && c != b'#'
            && !parser.is_linear_whitespace()
        {
            let mut key = String::new();
            while let Some(c) = parser.peek()
                && c != b'='
                && !parser.is_linear_whitespace()
            {
                if c == b'+' {
                    key.push(' ');
                    parser.consume();
                } else if URIPath::is_valid_segment(c) || c == b'/' || c == b'?' {
                    key.push(c as char);
                    parser.consume();
                } else if c == b'%' {
                    let pct = PctEncoding::parse(parser)?;
                    key.push(pct.0);
                } else {
                    break;
                }
            }

            parser.consume_or_err(|c| c == b'=')?;
            let mut val = String::new();

            while let Some(c) = parser.peek()
                && !(URI::is_sub_delim(c) && c != b'+')
                && !parser.is_linear_whitespace()
            {
                if c == b'+' {
                    val.push(' ');
                    parser.consume();
                } else if URIPath::is_valid_segment(c) || c == b'/' || c == b'?' {
                    val.push(c as char);
                    parser.consume();
                } else if c == b'%' {
                    let pct = PctEncoding::parse(parser)?;
                    val.push(pct.0);
                } else {
                    break;
                }
            }

            parameters.insert(
                key,
                DataHolder::Primitive {
                    ty: serializer::PrimType::String,
                    val,
                },
            );
            if parser.matches(|c| c == b'#' || c.is_ascii_whitespace()) {
                break;
            } else {
                parser.consume();
            }
        }

        Ok(RequestQuery {
            parameters: DataHolder::Struct(parameters),
        })
    }
}

impl PartialOrd for RequestQuery {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RequestQuery {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sorted_keys().cmp(&other.sorted_keys())
    }
}

/// Based on rfc3986 section 3.5
///
/// # Augmented Backus-Naur Form
/// ```text
/// fragment    = *( pchar / "/" / "?" )
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fragment(String);

impl<R: Read> Parsable<R> for Fragment {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        parser.consume_or_err(|c| c == b'#')?;
        let mut fragment = String::new();
        while let Some(c) = parser.peek() {
            if URIPath::is_valid_segment(c) || c == b'/' || c == b'?' {
                fragment.push(c as char);
                parser.consume();
            } else if c == b'%' {
                let pct = PctEncoding::parse(parser)?;
                fragment.push(pct.0);
            } else {
                break;
            }
        }

        Ok(Fragment(fragment))
    }
}

/// Based on See rfc3986 - Mainly Section 3
///
/// # Augmented Backus-Naur Form
/// ```text
/// URI         = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
///
/// hier-part   = "//" authority path-abempty
///   / path-absolute
///   / path-rootless
///   / path-empty
/// ```
///
/// # Example
/// ```text
/// foo://example.com:8042/over/there?name=ferret#nose
/// \_/   \______________/\_________/ \_________/ \__/
///  |           |            |            |        |
/// scheme   authority       path        query   fragment
/// ```
///
/// For now, this is only supporting authority format
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct URI {
    scheme: Scheme,
    authority: Authority,
    path: URIPath,
    query: Option<RequestQuery>,
    fragment: Option<Fragment>,
}

impl URI {
    /// Based on See rfc3986 section 3.3
    pub fn is_unreserved(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'-' || c == b'.' || c == b'_' || c == b'~'
    }
    pub fn is_reserved(c: u8) -> bool {
        Self::is_gen_delim(c) || Self::is_sub_delim(c)
    }

    pub fn is_gen_delim(c: u8) -> bool {
        c == b':' || c == b',' || c == b'?' || c == b'#' || c == b'[' || c == b']' || c == b'@'
    }

    pub fn is_sub_delim(c: u8) -> bool {
        c == b'!'
            || c == b'$'
            || c == b'&'
            || c == b'\''
            || c == b'('
            || c == b')'
            || c == b'*'
            || c == b'+'
            || c == b','
            || c == b';'
            || c == b'='
    }
}

impl<R: Read> Parsable<R> for URI {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self> {
        let scheme = Scheme::parse(parser)?;
        parser.expect_str("://")?;
        let authority = Authority::parse(parser)?;
        let path = URIPath::parse(parser)?;

        let query = if parser.matches(|c| c == b'?') {
            parser.consume_or_err(|c| c == b'?')?;
            Some(RequestQuery::parse(parser)?)
        } else {
            None
        };

        let fragment = if parser.matches(|c| c == b'#') {
            Some(Fragment::parse(parser)?)
        } else {
            None
        };

        Ok(URI {
            scheme,
            authority,
            path,
            query,
            fragment,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::StrParser;

    use super::*;

    #[test]
    fn test_valid_scheme() {
        let mut parser = StrParser::from_str("https://");
        assert_eq!(Scheme::parse(&mut parser), Ok(Scheme::from("https")));
        let mut parser = StrParser::from_str("http://");
        assert_eq!(Scheme::parse(&mut parser), Ok(Scheme::from("http")));
    }

    #[test]
    fn test_invalid_scheme() {
        let mut parser = StrParser::from_str("1https://");
        assert_eq!(Scheme::parse(&mut parser), Err(ParseErr::InvalidScheme));
    }

    #[test]
    fn test_pct_encoding() {
        let mut parser = StrParser::from_str("%3D");
        assert_eq!(PctEncoding::parse(&mut parser), Ok(PctEncoding::from('=')));
        let mut parser = StrParser::from_str("%3d");
        assert_eq!(PctEncoding::parse(&mut parser), Ok(PctEncoding::from('=')));
        let mut parser = StrParser::from_str("%3J");
        assert_eq!(
            PctEncoding::parse(&mut parser),
            Err(ParseErr::FailedToConsume {
                found: Some(74),
                at: parser.idx()
            })
        );
    }

    #[test]
    fn test_user_info() {
        let mut parser = StrParser::from_str("someuser@some_domain.com");
        assert_eq!(UserInfo::parse(&mut parser), Ok(UserInfo::from("someuser")));
        let mut parser = StrParser::from_str("someusersome_domain.com/apath");
        assert_eq!(
            UserInfo::parse(&mut parser),
            Err(ParseErr::NotUserInfo {
                presumed_host: String::from("someusersome_domain.com")
            })
        );
        let mut parser = StrParser::from_str("someusersome_domain.com");
        assert_eq!(UserInfo::parse(&mut parser), Err(ParseErr::InvalidUserInfo));
    }

    #[test]
    fn test_valid_ipv4_host() {
        let mut parser = StrParser::from_str("127.0.0.1");
        assert_eq!(Host::parse(&mut parser), Ok(Host::IPv4((127, 0, 0, 1))));
    }

    #[test]
    fn test_invalid_ipv4_host() {
        let mut parser = StrParser::from_str("257.0.0.1");
        assert_eq!(
            Host::parse(&mut parser),
            Err(ParseErr::InvalidIPv4Num {
                found: String::from("257")
            })
        );
    }

    #[test]
    fn test_valid_domain_host() {
        let mut parser = StrParser::from_str("www.example.com");
        assert_eq!(
            Host::parse(&mut parser),
            Ok(Host::Domain(String::from("www.example.com")))
        );
    }

    #[test]
    fn test_valid_authority() {
        let mut parser = StrParser::from_str("someuser@someemaildomain.com:8000");
        assert_eq!(
            Authority::parse(&mut parser),
            Ok(Authority {
                user_info: Some(UserInfo::from("someuser")),
                host: Host::Domain(String::from("someemaildomain.com")),
                port: Some(Port(8000)),
            })
        );
    }

    #[test]
    fn test_valid_authority_pct() {
        let mut parser = StrParser::from_str("some%3duser@some%3demaildomain.com:8000");
        assert_eq!(
            Authority::parse(&mut parser),
            Ok(Authority {
                user_info: Some(UserInfo::from("some=user")),
                host: Host::Domain(String::from("some=emaildomain.com")),
                port: Some(Port(8000)),
            })
        );
    }
    #[test]
    fn test_path_empty_case() {
        let mut parser = StrParser::from_str("/");
        assert_eq!(
            URIPath::parse(&mut parser),
            Ok(URIPath {
                ty: PathType::Absolute,
                segments: vec![],
                entire_path: String::from("/")
            })
        );
    }

    #[test]
    fn test_valid_path() {
        let mut parser = StrParser::from_str("/somerandompath/ye%3dp");
        assert_eq!(
            URIPath::parse(&mut parser),
            Ok(URIPath {
                ty: PathType::Absolute,
                segments: vec![String::from("somerandompath"), String::from("ye=p")],
                entire_path: String::from("/somerandompath/ye=p")
            })
        );

        let mut parser = StrParser::from_str("somerandompath/ye%3dp");
        assert_eq!(
            URIPath::parse(&mut parser),
            Ok(URIPath {
                ty: PathType::Relative,
                segments: vec![String::from("somerandompath"), String::from("ye=p")],
                entire_path: String::from("somerandompath/ye=p")
            })
        );
    }

    #[test]
    fn test_valid_query() {
        let mut parser = StrParser::from_str("some_param=some_val  "); //needs to break on white space for http
        let mut map = HashMap::new();
        map.insert(
            String::from("some_param"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("some_val"),
            },
        );

        let map = DataHolder::Struct(map);
        assert_eq!(
            RequestQuery::parse(&mut parser),
            Ok(RequestQuery { parameters: map })
        );

        let mut parser = StrParser::from_str(
            "some_param=some_val&some_param2=some_val&some_param3=val#someflag",
        );
        let mut map = HashMap::new();
        map.insert(
            String::from("some_param"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("some_val"),
            },
        );
        map.insert(
            String::from("some_param2"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("some_val"),
            },
        );
        map.insert(
            String::from("some_param3"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("val"),
            },
        );
        let map = DataHolder::Struct(map);
        assert_eq!(
            RequestQuery::parse(&mut parser),
            Ok(RequestQuery { parameters: map })
        );

        let mut parser = StrParser::from_str(
            "some_param=some+val&some_param2=some_val&some_param3=val#someflag",
        );
        let mut map = HashMap::new();
        map.insert(
            String::from("some_param"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("some val"),
            },
        );
        map.insert(
            String::from("some_param2"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("some_val"),
            },
        );
        map.insert(
            String::from("some_param3"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("val"),
            },
        );
        let map = DataHolder::Struct(map);
        assert_eq!(
            RequestQuery::parse(&mut parser),
            Ok(RequestQuery { parameters: map })
        );
    }

    #[test]
    fn test_valid_fragment() {
        let mut parser = StrParser::from_str("#some_param=some_val");
        assert_eq!(
            Fragment::parse(&mut parser),
            Ok(Fragment(String::from("some_param=some_val")))
        );
    }

    #[test]
    fn test_valid_uri() {
        let mut parser =
            StrParser::from_str("http://someaddress.com/apath?with=query#some_param=some_val");

        let mut parameters = HashMap::new();
        parameters.insert(
            String::from("with"),
            DataHolder::Primitive {
                ty: PrimType::String,
                val: String::from("query"),
            },
        );
        let parameters = DataHolder::Struct(parameters);
        assert_eq!(
            URI::parse(&mut parser),
            Ok(URI {
                scheme: Scheme(String::from("http")),
                authority: Authority {
                    user_info: None,
                    host: Host::Domain(String::from("someaddress.com")),
                    port: None
                },
                path: URIPath {
                    ty: PathType::Absolute,
                    segments: vec![String::from("apath")],
                    entire_path: String::from("/apath")
                },
                query: Some(RequestQuery { parameters }),
                fragment: Some(Fragment(String::from("some_param=some_val")))
            })
        );
    }
}
