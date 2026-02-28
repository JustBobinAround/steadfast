mod extract_macro;
mod token_parser;

use crate::{
    extract_macro::ExtractType,
    token_parser::{Struct, TokenParser},
};
use proc_macro::{TokenStream, TokenTree};
use uuid::UUID;

#[proc_macro]
pub fn impl_extract_permutations(_item: TokenStream) -> TokenStream {
    let choices = ExtractType::all_choices();
    ExtractType::make_combinations(choices).parse().unwrap()
}

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(item);

    parser
        .consume_if(|p| p.is_ident("async"))
        .expect("async token");
    parser.consume_if(|p| p.is_ident("fn")).expect("fn token");
    parser
        .consume_if(|p| p.is_ident("main"))
        .expect("main token");
    parser
        .consume_if(|p| p.is_any_group())
        .expect("empty fn parameters");
    parser.consume_if(|p| p.is_punct("-")).expect("-> token");
    parser.consume_if(|p| p.is_punct(">")).expect("-> token");
    let return_type: String = parser
        .consume_type()
        .expect("a return type")
        .into_iter()
        .map(|t| t.to_string())
        .collect();
    let function_block = parser
        .consume_if(|p| p.is_any_group())
        .expect("main function block");

    let s = format!(
        r#"async fn async_main() -> {} {}
fn main() -> {} {{
    ::zero::async_runtime::run(async_main())
}}"#,
        &return_type, function_block, &return_type,
    );
    eprintln!("{}", s);
    s.parse().expect("Failed to parse proc macro str")
}

fn parse_attrs(attrs: TokenStream) -> Result<String, ()> {
    let mut parser = TokenParser::new(attrs);

    let mut tokens = String::new();

    while parser.has_tokens_left() {
        let key = if parser.is_any_ident() {
            let name: String = parser
                .consume_while(|p| p.is_any_ident() || p.is_punct("-"))
                .into_iter()
                .map(|t| t.to_string())
                .collect();
            format!("\"{}\"", name)
        } else {
            if parser.is_any_punct() || parser.is_any_ident() {
                panic!("Expected attribute key, found punctuation or ident");
            } else if let Some(t) = parser.consume() {
                t.to_string()
            } else {
                break;
            }
        };
        parser.consume_if(|p| p.is_punct(":"))?;
        if parser.is_any_punct() {
            panic!("Expected attribute val, found punctuation");
        }
        let val = match parser.consume_as_str() {
            Some(s) => s,
            None => break,
        };
        tokens.push_str(&format!(".set_attr({}.into(),{}.into())", key, val));

        if !parser.has_tokens_left() {
            break;
        } else if parser.is_any_punct() {
            parser.consume();
        } else {
            panic!("Expected punctuation or end of html attributes")
        }
    }

    Ok(tokens)
}

#[proc_macro]
pub fn html(item: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(item);

    let mut tokens = String::new();
    while parser.has_tokens_left() {
        let tag_name = match parser.consume() {
            Some(TokenTree::Ident(i)) => i,
            Some(TokenTree::Literal(l)) => {
                return format!("Into::<::zero::html::Markup>::into({})", l)
                    .parse()
                    .unwrap();
            }
            Some(TokenTree::Group(g)) => {
                return format!("({}).into()", g.stream())
                    .parse()
                    .expect("Failed to parse inner HTML");
            }
            Some(t) => panic!("Expected TagType, found {:#?}", t),
            None => return "()".parse().unwrap(),
        };

        if parser.is_any_ident() {
            tokens.push_str(&format!(
                "{{::zero::html::Tag::new(::zero::html::TagType::{})}},\n",
                tag_name
            ));
            // parser.consume();
            // continue;
        } else {
            let tt = parser.consume();

            let attrs = if let Some(TokenTree::Group(g)) = tt {
                parse_attrs(g.stream()).expect("expected valid attribute")
            } else if tt.is_some() {
                panic!("Expected Grouping for Attributes")
            } else {
                String::new()
            };

            if parser.is_any_group() {
                let inner = match parser.consume() {
                    Some(TokenTree::Group(g)) => html(g.stream()),
                    None => "".parse().unwrap(),
                    _ => {
                        panic!("Expected Grouping for inner markup")
                    }
                };

                tokens.push_str(&format!(
                    "{{::zero::html::Tag::new(::zero::html::TagType::{}){}.set_content({}) }},\n",
                    tag_name, attrs, inner
                ));
            } else {
                tokens.push_str(&format!(
                    "{{::zero::html::Tag::new(::zero::html::TagType::{}){} }},\n",
                    tag_name, attrs
                ));
            }
        }
    }

    let s = format!("Into::<::zero::html::Markup>::into(vec![{}])", tokens);

    s.parse().unwrap()
}

fn parse_deserialize_struct(mut parser: TokenParser, is_public: bool) -> TokenStream {
    let data_struct = parser.consume_struct(is_public).expect("a valid struct");

    let struct_name = data_struct.name();
    // let generic_idents = data_struct.generic_idents();
    // let generic_traits = data_struct.generic_traits();
    // if generics.len() > 0 {
    //     // TODO: add generic support
    //     unimplemented!("deriving deserialize with generics is not currently supported");
    // }
    let fields: String = data_struct
        .fields()
        .iter()
        .map(|(name, field_data)| {
            format!(
                "{}: match dh.remove(\"{}\") {{
                    Some(dh) => <{}>::deserialize(dh)?,
                    None => return Err(())
                }},",
                name,
                name,
                field_data.ty_str()
            )
        })
        .collect();

    let output = format!(
        r#"impl ::zero::serializer::Deserialize for {} {{
    fn deserialize(dh: ::zero::serializer::DataHolder) -> Result<Self, ()> {{
        match dh {{
            ::zero::serializer::DataHolder::Struct(mut dh) => Ok(Self {{
                {}
            }}),
            _ => Err(())
        }}
    }}
}}"#,
        struct_name, fields
    );

    // tokens.push(group);
    eprintln!("{}", &output);

    output.parse().unwrap()
}

#[proc_macro_derive(Deserialize)]
pub fn derive_deserialize(items: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(items);

    let is_pub = parser.is_ident("pub");
    if is_pub {
        parser.consume();
    }

    match parser.consume_if(|p| p.is_ident("struct")) {
        Ok(_) => parse_deserialize_struct(parser, is_pub),
        Err(_) => match parser.consume_if(|p| p.is_ident("enum")) {
            Ok(_) => {
                unimplemented!("Enum serialization is not supported at this time")
            }

            Err(_) => panic!("Expected a struct or enum"),
        },
    }
}

fn parse_db_bytes_struct(
    other_traits: String,
    mut parser: TokenParser,
    is_public: bool,
    data_struct: Struct,
) -> TokenStream {
    let struct_name = data_struct.name();
    let generic_idents: String = data_struct
        .generic_idents()
        .iter()
        .map(|i| i.to_string())
        .collect();
    let generic_traits: String = data_struct
        .generic_traits()
        .iter()
        .map(|i| i.to_string())
        .collect();
    // if generics.len() > 0 {
    //     // TODO: add generic support
    //     unimplemented!("deriving ToDatabaseBytes with generics is not currently supported");
    // }
    let (fields, to_reverse): (String, Vec<String>) = data_struct
        .fields()
        .iter()
        .map(|(name, field_data)| {
            (
                format!("\n\t.push_into(self.{})", name,),
                format!(
                    "{}: <{}>::from_db_bytes(bytes)?,\n",
                    name,
                    field_data.ty_str()
                ),
            )
        })
        .collect();
    let reversed: String = to_reverse.into_iter().rev().collect();

    let output = format!(
        r#"{}impl{} ::zero::db::ToDatabaseBytes for {}{} {{
            fn to_db_bytes(self) -> ::zero::db::DatabaseBytes {{
                ::zero::db::DatabaseBytes::default(){}
            }}

            fn from_db_bytes(bytes: &mut ::zero::db::DatabaseBytes) -> Result<Self, ()> {{
                Ok(Self {{
                    {}
                }})
            }}
        }}"#,
        other_traits, generic_traits, struct_name, generic_idents, fields, reversed
    );

    // tokens.push(group);
    eprintln!("{}", &output);

    output.parse().unwrap()
}

#[proc_macro_derive(ToDatabaseBytes)]
pub fn derive_to_db_bytes(items: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(items);

    let is_pub = parser.is_ident("pub");
    if is_pub {
        parser.consume();
    }

    match parser.consume_if(|p| p.is_ident("struct")) {
        Ok(_) => {
            let data_struct = parser.consume_struct(is_pub).expect("a valid struct");
            parse_db_bytes_struct(String::new(), parser, is_pub, data_struct)
        }
        Err(_) => match parser.consume_if(|p| p.is_ident("enum")) {
            Ok(_) => {
                unimplemented!("Enum serialization is not supported at this time")
            }

            Err(_) => panic!("Expected a struct or enum"),
        },
    }
}
#[proc_macro_derive(ZeroTable)]
pub fn derive_zero_table(items: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(items);

    let is_pub = parser.is_ident("pub");
    if is_pub {
        parser.consume();
    }

    match parser.consume_if(|p| p.is_ident("struct")) {
        Ok(_) => {
            let data_struct = parser.consume_struct(is_pub).expect("a valid struct");
            let traits: String = data_struct
                .generic_traits()
                .iter()
                .map(|t| t.to_string())
                .collect();
            let idents: String = data_struct
                .generic_idents()
                .iter()
                .map(|t| t.to_string())
                .collect();
            let zero_table_trait = format!(
                r#"impl{} ::zero::db::ZeroTable for {}{} {{
                    fn table_name() -> &'static str {{
                        "{}"
                    }}
                    fn table_version_hash() -> ::zero::UUID {{
                        ::zero::{}
                    }}
                }}"#,
                traits,
                data_struct.name(),
                idents,
                data_struct.name(),
                UUID::from_table_hash(data_struct.struct_signature())
                    .expect("Failed to build table uuid")
                    .as_token_string(),
            );

            let name = data_struct.name().clone();

            let t = parse_db_bytes_struct(zero_table_trait, parser, is_pub, data_struct);
            if name == "UUID" {
                eprintln!("{:#?}", t);
            }
            t
        }
        Err(_) => match parser.consume_if(|p| p.is_ident("enum")) {
            Ok(_) => {
                unimplemented!("Enum serialization is not supported at this time")
            }

            Err(_) => panic!("Expected a struct or enum"),
        },
    }
}
