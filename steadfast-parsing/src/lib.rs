use std::{
    io::{BufReader, Cursor, Read},
    num::ParseIntError,
    str::Utf8Error,
};

pub mod prelude {
    pub use super::{Parsable, ParseErr, ParseResult, Parser};
}

/// Trait to add standardized parsing methods
pub trait Parsable<R: Read>: StreamParser<R> {
    fn parse(parser: &mut Parser<R>) -> ParseResult<Self>;
}

/// Auto-impl trait to allow for parsing of streams.
///
/// This trait was made mainly for TCP stream parsing.
pub trait StreamParser<R: Read>: Sized {
    fn parse_from_stream(stream: R) -> ParseResult<Self>;
}

impl<T, R: Read> StreamParser<R> for T
where
    T: Parsable<R>,
{
    fn parse_from_stream(stream: R) -> ParseResult<Self> {
        let mut parser = Parser::<R>::from_stream(stream);
        Self::parse(&mut parser)
    }
}

/// Main error type for parsable trait.
///
/// Should only be used with ParseResult Type.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseErr {
    General,
    InvalidStatusCode {
        found: u16,
    },
    InvalidStatusCodeStr {
        found: u8,
    },
    ExpectedStatusCode,
    InvalidRequestOption {
        found: String,
    },
    InvalidUTF8(Utf8Error),
    InvalidScheme,
    InvalidPctEncoding {
        found: String,
    },
    InvalidIPv4Num {
        found: String,
    },
    InvalidIPv4Char {
        found: char,
    },
    InvalidIPv4Len {
        found: usize,
    },
    InvalidPath,
    InvalidUserInfo,
    BlankHeaderFieldName,
    NotUserInfo {
        presumed_host: String,
    }, // not really an err, just needs a breakout for an edgecase
    EmptyStack,
    ExpectedValidKeyword {
        found: String,
        at: usize,
    },
    ExpectedStr {
        expected: String,
        found_char: char,
        at: usize,
    },
    FoundLeadingZero,
    FailedToConsume {
        found: Option<u8>,
        at: usize,
    },
    FailedToParseNum {
        found: String,
        radix: u32,
    },
    ParseIntErr(ParseIntError),
    FailedToSeekDuringPop {
        tried_seeking_to: usize,
    },
    ZeroLenDispositionTy,
}

impl From<ParseIntError> for ParseErr {
    fn from(value: ParseIntError) -> Self {
        ParseErr::ParseIntErr(value)
    }
}

impl From<Utf8Error> for ParseErr {
    fn from(value: Utf8Error) -> Self {
        ParseErr::InvalidUTF8(value)
    }
}

/// Result type for Parsable trait
pub type ParseResult<T> = Result<T, ParseErr>;

/// Simple wrapper type to allow parsing of strings as a stream
///
/// This type is mainly used for testing parsable structs
pub type StrParser<'a> = Parser<Cursor<&'a str>>;

/// Used by Parsable trait
///
/// This Parser contains a variety of methods that
/// are fairly universal to parsing. Although there
/// are some protocol specific parsing methods, adding
/// non-universal methods should be avoided.
pub struct Parser<R: Read> {
    reader: BufReader<R>,
    idx: usize,
    peek: Option<u8>,
}

impl<R: Read> Parser<R> {
    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn from_str(s: &str) -> Parser<Cursor<&str>> {
        let stream = Cursor::new(s);
        Parser {
            reader: BufReader::new(stream),
            idx: 0,
            peek: None,
        }
    }

    pub fn from_stream(stream: R) -> Parser<R> {
        Parser {
            reader: BufReader::new(stream),
            idx: 0,
            peek: None,
        }
    }

    /// Gives access to the current value under the buffers seeking head. This is usually
    /// used in tandom with `consume` after the seeking head has a value that meets certain
    /// conditions
    pub fn peek(&mut self) -> Option<u8> {
        if self.peek.is_none() {
            let mut buf = [0; 1];
            match self.reader.read_exact(&mut buf) {
                Ok(_) => {
                    self.peek = Some(buf[0]);
                    self.peek
                }
                Err(_) => None,
            }
        } else {
            self.peek
        }
    }

    /// Reads the value under the seeking head, moves the seeking head forward by 1, then returns the value.
    pub fn consume(&mut self) -> Option<u8> {
        if self.peek.is_none() {
            let mut buf = [0; 1];
            match self.reader.read_exact(&mut buf) {
                Ok(_) => {
                    self.idx += 1;
                    Some(buf[0])
                }
                Err(_) => None,
            }
        } else {
            self.peek.take()
        }
    }

    /// Builds a string while the value under the seeking head is found to meet conditions provided by the closure `f`.
    pub fn consume_while<F: Fn(&mut Self) -> bool>(&mut self, f: F) -> String {
        let mut s = String::new();

        while f(self) {
            if let Some(c) = self.consume() {
                s.push(c as char);
            } else {
                break;
            }
        }

        s
    }
    pub fn consume_n(&mut self, n: usize) -> String {
        let mut s = String::new();

        for _ in 0..n {
            if let Some(c) = self.consume() {
                s.push(c as char);
            } else {
                break;
            }
        }

        s
    }

    pub fn consume_while_lower<F: Fn(&mut Self) -> bool>(&mut self, f: F) -> String {
        let mut s = String::new();

        while f(self) {
            if let Some(c) = self.consume() {
                s.push((c as char).to_ascii_lowercase());
            } else {
                break;
            }
        }

        s
    }

    pub fn skip_whitespace(&mut self) {
        while self.is_linear_whitespace() {
            self.consume();
        }
    }
    pub fn skip_whitespace_and_lines(&mut self) {
        while self.is_linear_whitespace() || self.matches(|b| b == b'\n') {
            self.consume();
        }
    }

    pub fn consume_escaped<F: Fn(&mut Self) -> bool, FF: Fn(&mut Self) -> bool>(
        &mut self,
        is_escape: FF,
        f: F,
    ) -> String {
        let mut s = String::new();

        while f(self) || is_escape(self) {
            if is_escape(self) {
                self.consume();
                if let Some(c) = self.consume() {
                    s.push(c as char);
                } else {
                    break;
                }
            } else if let Some(c) = self.consume() {
                s.push(c as char);
            } else {
                break;
            }
        }

        s
    }

    pub fn consume_str_lit(&mut self) -> String {
        self.consume_escaped(|c| c.matches(|c| c == b'\\'), |c| c.matches(|c| c != b'"'))
    }

    // HTTP spec section 2.2

    // HTTP spec section 2.2 ALPHA
    pub fn is_alpha(&mut self) -> bool {
        self.peek().is_some_and(|c| c.is_ascii_alphabetic())
    }

    // HTTP spec section 2.2 UPALPHA
    pub fn is_upalpha(&mut self) -> bool {
        self.peek().is_some_and(|c| c.is_ascii_uppercase())
    }

    // HTTP spec section 2.2 LOALPHA
    pub fn is_loalpha(&mut self) -> bool {
        self.peek().is_some_and(|c| c.is_ascii_lowercase())
    }

    // HTTP spec section 2.2 DIGIT
    pub fn is_digit(&mut self) -> bool {
        self.peek().is_some_and(|c| c.is_ascii_digit())
    }

    // HTTP spec section 2.2 CTL
    pub fn is_control_char(&mut self) -> bool {
        self.peek().is_some_and(|c| c.is_ascii_control())
    }

    // HTTP spec section 2.2 CR
    pub fn is_carriage_return(&mut self) -> bool {
        self.peek().is_some_and(|c| c == 13)
    }

    // HTTP spec section 2.2 LF
    pub fn is_linefeed(&mut self) -> bool {
        self.peek().is_some_and(|c| c == 10)
    }

    // HTTP spec section 2.2 SP
    pub fn is_space(&mut self) -> bool {
        self.peek().is_some_and(|c| c == b' ')
    }

    // HTTP spec section 2.2 HT
    pub fn is_horizontal_tab(&mut self) -> bool {
        self.peek().is_some_and(|c| c == b'\t')
    }

    // HTTP spec section 2.2 <">
    pub fn is_dquote(&mut self) -> bool {
        self.peek().is_some_and(|c| c == b'"')
    }

    // HTTP spec section 2.2 LWS
    pub fn is_linear_whitespace(&mut self) -> bool {
        self.is_space() || self.is_horizontal_tab()
    }

    // HTTP spec section 2.2 TEXT
    pub fn is_text(&mut self) -> bool {
        !self.is_control_char() || self.is_linear_whitespace()
    }

    // HTTP spec section 2.2 HEX
    pub fn is_hex_digit(&mut self) -> bool {
        self.peek().is_some_and(|c| c.is_ascii_hexdigit())
    }

    // HTTP spec section 2.2 token
    // not full token definition as a token is many chars
    pub fn is_token_char(&mut self) -> bool {
        !(self.is_control_char() || self.is_separator())
    }

    // HTTP spec section 2.2 separator
    pub fn is_separator(&mut self) -> bool {
        self.peek().is_some_and(|c| {
            c == b'('
                || c == b')'
                || c == b'<'
                || c == b'>'
                || c == b'@'
                || c == b','
                || c == b';'
                || c == b':'
                || c == b'\\'
                || c == b'"'
                || c == b'/'
                || c == b'['
                || c == b']'
                || c == b'?'
                || c == b'='
                || c == b'{'
                || c == b'}'
                || c == b' '
                || c == b'\t'
        })
    }

    pub fn is_ctext(&mut self) -> bool {
        self.is_text() && self.peek().is_some_and(|c| !(c == b'(' || c == b')'))
    }

    pub fn is_qdtext(&mut self) -> bool {
        self.is_text() && self.peek().is_some_and(|c| c != b'\"')
    }

    pub fn matches<F: Fn(u8) -> bool>(&mut self, f: F) -> bool {
        self.peek().is_some_and(f)
    }

    pub fn consume_or_err<F: Fn(u8) -> bool>(&mut self, f: F) -> ParseResult<u8> {
        let peek = self.peek();
        if peek.is_some_and(f) {
            Ok(self.consume().unwrap())
        } else {
            Err(ParseErr::FailedToConsume {
                found: peek,
                at: self.idx(),
            })
        }
    }

    pub fn expect_str(&mut self, s: &str) -> ParseResult<()> {
        for c in s.bytes() {
            if self.consume().is_none_or(|found| c != found) {
                return Err(ParseErr::ExpectedStr {
                    expected: s.to_string(),
                    found_char: c as char,
                    at: self.idx,
                });
            }
        }

        Ok(())
    }

    pub fn expect_crlf(&mut self) -> ParseResult<()> {
        self.expect_str("\r\n")
    }
}
