pub enum ExtractType {
    Instance(GenericType),
    Method,
    Path(GenericType),
    Query(GenericType),
    HTTPVersion,
    RequestHeaders,
    Body(GenericType),
}

impl ExtractType {
    pub const INSTANCE: &'static Self = &Self::Instance(GenericType::T);
    pub const METHOD: &'static Self = &Self::Method;
    pub const PATH: &'static Self = &Self::Path(GenericType::A(ExtractTrait::ToPath));
    pub const QUERY: &'static Self = &Self::Query(GenericType::B(ExtractTrait::ToQuery));
    pub const HTTP_VERSION: &'static Self = &Self::HTTPVersion;
    pub const REQUEST_HEADERS: &'static Self = &Self::RequestHeaders;
    pub const BODY: &'static Self = &Self::Body(GenericType::C(ExtractTrait::ToBody));
    const fn identity_name(&self) -> &'static str {
        match self {
            Self::Instance(_) => "instance",
            Self::Method => "method",
            Self::Path(_) => "path",
            Self::Query(_) => "query",
            Self::HTTPVersion => "http_version",
            Self::RequestHeaders => "headers",
            Self::Body(_) => "body",
        }
    }

    const fn type_no_trait(&self) -> &'static str {
        match self {
            Self::Instance(_) => "Instance<T>",
            Self::Method => "Method",
            Self::Path(_) => "Path<A>",
            Self::Query(_) => "Query<B>",
            Self::HTTPVersion => "HTTPVersion",
            Self::RequestHeaders => "RequestHeaders",
            Self::Body(_) => "Body<C>",
        }
    }

    // impl<$instance_ty $($(,$part_subty$(:$trait)?)?)+> Extract<$instance_ty, InstanceRequest<$instance_ty>, Self> for ($($part_ty$(<$part_subty>)?,)*) {
    //     fn from_request(instance: PhantomData<$instance_ty>, req: InstanceRequest<$instance_ty>) -> Result<Self, ()> {
    //         Ok(($(<$part_ty$(<$part_subty>)?>::from_request(instance, req.$part_name)?,)*))
    //     }
    // }
    //
    fn make_extract_impl(selections: &[&'static Self]) -> String {
        let t = GenericType::T.main_type_str();
        let impl_generics: String = selections
            .iter()
            .map(|g| match g {
                Self::Path(g) | Self::Query(g) | Self::Body(g) => format!(",{}", g),
                _ => String::new(),
            })
            .collect();

        let self_def: String = selections
            .iter()
            .map(|g| format!("{},", g.type_no_trait()))
            .collect();

        let tuple: String = selections
            .iter()
            .map(|g| match g {
                Self::Instance(_) => format!("\n\t\treq.{},", g.identity_name()),
                _ => format!(
                    "\n\t\t<{}>::from_request(instance, req.{})?,",
                    g.type_no_trait(),
                    g.identity_name()
                ),
            })
            .collect();

        format!(
            r#"/// This is macro generated. See actual trait documentation instead
impl<{}: Send + Sync {}> Extract<{}, InstanceRequest<{}>, Self> for ({}) {{
    fn from_request(instance: PhantomData<{}>, req: InstanceRequest<{}>) -> Result<Self, ()> {{
        Ok(({}
        ))
    }}
}}
"#,
            t, impl_generics, t, t, self_def, t, t, tuple
        )
    }

    pub fn make_combinations(choices: [&'static Self; 7]) -> String {
        let mut result = String::new();
        let n = choices.len();

        // Use bitmasking to generate all ordered subsequences
        for mask in 1..(1 << n) {
            let mut selections = Vec::new();

            for i in 0..n {
                if (mask & (1 << i)) != 0 {
                    selections.push(choices[i]);
                }
            }

            let sub_seq = Self::make_extract_impl(&selections);
            // eprintln!("{}", sub_seq);

            result.push_str(&sub_seq);
        }

        result
    }

    pub const fn all_choices() -> [&'static Self; 7] {
        [
            Self::INSTANCE,
            Self::METHOD,
            Self::PATH,
            Self::QUERY,
            Self::HTTP_VERSION,
            Self::REQUEST_HEADERS,
            Self::BODY,
        ]
    }
}

impl std::fmt::Display for ExtractType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Instance(g) => write!(f, "Instance<{}>", g),
            Self::Method => write!(f, "Method"),
            Self::Path(g) => write!(f, "Path<{}>", g),
            Self::Query(g) => write!(f, "Query<{}>", g),
            Self::HTTPVersion => write!(f, "HTTPVersion"),
            Self::RequestHeaders => write!(f, "RequestHeaders",),
            Self::Body(g) => write!(f, "Body<{}>", g),
        }
    }
}

#[allow(clippy::enum_variant_names)]
pub enum ExtractTrait {
    ToPath,
    ToQuery,
    ToBody,
}

impl std::fmt::Display for ExtractTrait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToPath => write!(f, "ToPath"),
            Self::ToQuery => write!(f, "ToQuery"),
            Self::ToBody => write!(f, "ToBody"),
        }
    }
}

pub enum GenericType {
    T,
    A(ExtractTrait),
    B(ExtractTrait),
    C(ExtractTrait),
}
impl GenericType {
    const fn main_type_str(&self) -> &'static str {
        match self {
            Self::T => "T",
            Self::A(_) => "A",
            Self::B(_) => "B",
            Self::C(_) => "C",
        }
    }
}
impl std::fmt::Display for GenericType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::T => write!(f, "T"),
            Self::A(g) => write!(f, "A: {}", g),
            Self::B(g) => write!(f, "B: {}", g),
            Self::C(g) => write!(f, "C: {}", g),
        }
    }
}
