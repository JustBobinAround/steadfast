use proc_macro::{TokenStream, TokenTree, token_stream::IntoIter};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug)]
pub struct StructField {
    name: Arc<String>,
    is_public: bool,
    ty: Vec<TokenTree>,
}

impl StructField {
    pub fn ty_str(&self) -> String {
        self.ty.iter().map(|t| t.to_string()).collect()
    }
}

#[derive(Debug)]
pub struct Struct {
    is_public: bool,
    name: String,
    generic_idents: Vec<TokenTree>,
    generic_traits: Vec<TokenTree>,
    fields: BTreeMap<Arc<String>, StructField>,
}

impl Struct {
    pub fn new(
        is_public: bool,
        name: String,
        generic_idents: Vec<TokenTree>,
        generic_traits: Vec<TokenTree>,
    ) -> Self {
        Struct {
            is_public,
            name,
            generic_idents,
            generic_traits,
            fields: BTreeMap::new(),
        }
    }

    fn hash_str(mut start: u64, s: String) -> u64 {
        for b in s.bytes() {
            start = start.rotate_left(1) ^ b as u64;
        }
        start
    }

    pub fn struct_signature(&self) -> u64 {
        let mut start = 0;
        let mut generic_traits: Vec<String> = self
            .generic_traits()
            .iter()
            .map(|s| s.to_string())
            .collect();

        generic_traits.sort();

        for t in generic_traits {
            start = Self::hash_str(start, t);
        }

        for (name, field) in self.fields() {
            start = Self::hash_str(start, name.to_string());
            for t in field.ty.iter() {
                start = Self::hash_str(start, t.to_string());
            }
        }

        start
    }

    pub fn is_public(&self) -> bool {
        self.is_public
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn generic_idents(&self) -> &Vec<TokenTree> {
        &self.generic_idents
    }
    pub fn generic_traits(&self) -> &Vec<TokenTree> {
        &self.generic_traits
    }

    pub fn fields(&self) -> &BTreeMap<Arc<String>, StructField> {
        &self.fields
    }

    pub fn add_field(&mut self, name: String, is_public: bool, ty: Vec<TokenTree>) {
        let name = Arc::new(name);
        self.fields.insert(
            name.clone(),
            StructField {
                name,
                is_public,
                ty,
            },
        );
    }
}

pub struct TokenParser {
    stream: IntoIter,
    peek: Option<TokenTree>,
}

macro_rules! impl_is_any {
    ($name:ident, $tt:tt) => {
        pub fn $name(&mut self) -> bool {
            match self.peek() {
                Some(TokenTree::$tt(_)) => true,
                _ => false,
            }
        }
    };
}

impl TokenParser {
    pub fn new(stream: TokenStream) -> Self {
        let stream = stream.into_iter();
        Self { stream, peek: None }
    }

    pub fn peek(&mut self) -> &Option<TokenTree> {
        if self.peek.is_none() {
            self.peek = self.stream.next();
            &self.peek
        } else {
            &self.peek
        }
    }

    pub fn consume(&mut self) -> Option<TokenTree> {
        self.peek();
        self.peek.take()
    }

    pub fn consume_as_str(&mut self) -> Option<String> {
        self.peek();
        match self.peek.take() {
            Some(TokenTree::Ident(i)) => Some(format!("{}", i)),
            Some(TokenTree::Literal(l)) => Some(format!("{}", l)),
            Some(TokenTree::Group(g)) => Some(format!("{}", g)),
            Some(TokenTree::Punct(p)) => Some(format!("{}", p)),
            None => None,
        }
    }

    pub fn has_tokens_left(&mut self) -> bool {
        self.peek().is_some()
    }

    pub fn is_void(&mut self) -> bool {
        match self.peek() {
            Some(TokenTree::Group(g)) => g.to_string() == "()",
            _ => false,
        }
    }

    impl_is_any!(is_any_ident, Ident);
    impl_is_any!(is_any_group, Group);
    impl_is_any!(is_any_punct, Punct);
    impl_is_any!(is_any_literal, Literal);

    pub fn is_ident(&mut self, s: &str) -> bool {
        match self.peek() {
            Some(TokenTree::Ident(i)) => i.to_string() == s,
            _ => false,
        }
    }

    pub fn is_punct(&mut self, s: &str) -> bool {
        match self.peek() {
            Some(TokenTree::Punct(p)) => p.to_string() == s,
            _ => false,
        }
    }

    pub fn is_literal(&mut self, s: &str) -> bool {
        match self.peek() {
            Some(TokenTree::Literal(l)) => l.to_string() == format!("\"{}\"", s),
            _ => false,
        }
    }

    pub fn consume_while<F: Fn(&mut Self) -> bool>(&mut self, f: F) -> Vec<TokenTree> {
        let mut s = Vec::new();

        while f(self) {
            if let Some(tt) = self.consume() {
                s.push(tt);
            } else {
                break;
            }
        }

        s
    }

    pub fn consume_if<F: Fn(&mut Self) -> bool>(&mut self, f: F) -> Result<TokenTree, ()> {
        if f(self) {
            if let Some(tt) = self.consume() {
                Ok(tt)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    fn consume_sub_type(&mut self, mut tokens: Vec<TokenTree>) -> Result<Vec<TokenTree>, ()> {
        tokens.push(self.consume_if(|p| p.is_punct("<"))?);
        while self.has_tokens_left() {
            tokens = self.consume_type_impl(tokens)?;
            if let Ok(t) = self.consume_if(|p| p.is_punct(",")) {
                tokens.push(t)
            } else {
                break;
            }
        }

        tokens.push(self.consume_if(|p| p.is_punct(">"))?);
        Ok(tokens)
    }

    fn consume_type_impl(&mut self, mut tokens: Vec<TokenTree>) -> Result<Vec<TokenTree>, ()> {
        if let Ok(t) = self.consume_if(|p| p.is_punct("&")) {
            tokens.push(t);
            tokens.push(self.consume_if(|p| p.is_punct("'"))?);
            tokens.push(self.consume_if(|p| p.is_any_ident())?);
        }

        // handling array/slice edge case
        if self.is_any_group() {
            match self.consume() {
                Some(t) => {
                    tokens.push(t);
                }
                _ => {
                    unreachable!("Consumed tokentree other than group even though check said group")
                }
            }
            Ok(tokens)
        } else {
            tokens.push(self.consume_if(|p| p.is_any_ident() || p.is_void())?);
            if self.is_punct("<") {
                self.consume_sub_type(tokens)
            } else {
                Ok(tokens)
            }
        }
    }

    pub fn consume_type(&mut self) -> Result<Vec<TokenTree>, ()> {
        let tokens = Vec::new();
        self.consume_type_impl(tokens)
    }

    pub fn consume_generics_impl(
        &mut self,
        mut idents: Vec<TokenTree>,
        mut traits: Vec<TokenTree>,
    ) -> Result<(Vec<TokenTree>, Vec<TokenTree>), ()> {
        let start = self.consume_if(|p| p.is_punct("<"))?;
        idents.push(start.clone());
        traits.push(start);
        while self.has_tokens_left() {
            if let Ok(t) = self.consume_if(|p| p.is_punct("'")) {
                idents.push(t.clone());
                traits.push(t);
                let ident = self.consume_if(|p| p.is_any_ident())?;
                idents.push(ident.clone());
                traits.push(ident);
            } else if let Ok(t) = self.consume_if(|p| p.is_any_ident()) {
                idents.push(t.clone());
                traits.push(t);
                if let Ok(t) = self.consume_if(|p| p.is_punct(":")) {
                    traits.push(t);
                    while let Ok(ident) = self.consume_if(|p| p.is_any_ident()) {
                        traits.push(ident);
                        if let Ok(t) = self.consume_if(|p| p.is_punct("+")) {
                            traits.push(t);
                        } else if let Ok(g) = self.consume_if(|p| p.is_any_group()) {
                            unimplemented!(
                                "parsing of generic closures are not currently supported"
                            )
                        }
                    }
                }
            }
            if let Ok(t) = self.consume_if(|p| p.is_punct(",")) {
                idents.push(t.clone());
                traits.push(t);
            } else {
                break;
            }
        }
        let end = self.consume_if(|p| p.is_punct(">"))?;
        idents.push(end.clone());
        traits.push(end);

        Ok((idents, traits))
    }

    pub fn consume_generics(&mut self) -> Result<(Vec<TokenTree>, Vec<TokenTree>), ()> {
        let idents = Vec::new();
        let traits = Vec::new();
        self.consume_generics_impl(idents, traits)
    }

    pub fn consume_struct(&mut self, is_public: bool) -> Result<Struct, ()> {
        let name = self.consume_if(|p| p.is_any_ident())?.to_string();

        let (generic_idents, generic_traits) = if self.is_punct("<") {
            self.consume_generics()?
        } else {
            (Vec::new(), Vec::new())
        };

        let mut data_struct = Struct::new(is_public, name, generic_idents, generic_traits);

        let fields = match self.consume() {
            Some(TokenTree::Group(g)) => g,
            _ => return Err(()),
        };

        let mut inner_parser = TokenParser::new(fields.stream());

        while inner_parser.has_tokens_left() {
            let ident = inner_parser.consume_if(|p| p.is_any_ident())?.to_string();

            let is_pub = ident == "pub";

            let ident = if is_pub {
                inner_parser.consume_if(|p| p.is_any_ident())?.to_string()
            } else {
                ident
            };

            inner_parser.consume_if(|p| p.is_punct(":"))?;

            let ty = inner_parser.consume_type()?;

            data_struct.add_field(ident, is_pub, ty);

            let _ = inner_parser.consume_if(|p| p.is_punct(","));
        }

        Ok(data_struct)
    }

    pub fn to_token_stream(s: Vec<TokenTree>) -> TokenStream {
        s.into_iter().map(|tt| tt).collect()
    }
}
