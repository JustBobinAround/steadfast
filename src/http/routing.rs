use super::{
    Body, HTTPVersion, ToBody,
    request::{Method, Request, RequestBody, RequestHeaders},
    response::{Response as FullResponse, ResponseHeaderType, StatusCode},
    uri::{RequestQuery, URIPath},
};
use crate::{html::Markup, http::ToMessageHeader, serializer::Deserialize};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
};

/// Request + Instance wrapper function that makes code generation
/// and ownership a bit easier.
#[derive(Debug, PartialEq, Eq)]
pub struct InstanceRequest<T: Send + Sync> {
    instance: Arc<T>,
    method: Method,
    path: URIPath,
    query: RequestQuery,
    http_version: HTTPVersion,
    headers: RequestHeaders,
    body: RequestBody,
}

impl<T: Send + Sync> InstanceRequest<T> {
    /// Converts a `::http::request::Request` and a `Arc<T>` into a `InstanceRequest<T>`.
    pub fn from_request(instance: Arc<T>, r: Request) -> Self {
        InstanceRequest {
            instance,
            method: r.method,
            path: r.path,
            query: r.query,
            http_version: r.http_version,
            headers: r.headers,
            body: r.body,
        }
    }
}

/// Wrapper struct for the actual `::http::response::Response` struct so that fields can be optional
///
/// The reasoning behind making a wrapper struct instead of using the actual response field
/// is so that pieces of the response can be infered by the information left out. For instance,
/// if we convert a `Result<T,E>` into this response but we don't provide the status code, we can
/// assume a set of default status codes based on the result, i.e. `Ok(T) => 200` and `Err(E) => 500`.
/// Using a generic 500 response code is a bit against best practice, but it make writing routes quite
/// intuitive. With that said, and although quite counter intuitive, limiting to two error response types
/// allows for easy generic error handling abstractions
#[derive(Debug)]
pub struct Response {
    status: Option<StatusCode>,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
}

impl From<()> for Response {
    fn from(_: ()) -> Self {
        Response {
            status: None,
            headers: None,
            body: None,
        }
    }
}

impl From<StatusCode> for Response {
    fn from(status: StatusCode) -> Self {
        Response {
            status: Some(status),
            headers: None,
            body: None,
        }
    }
}

impl From<HashMap<String, String>> for Response {
    fn from(headers: HashMap<String, String>) -> Self {
        Response {
            status: None,
            headers: Some(headers),
            body: None,
        }
    }
}

impl From<String> for Response {
    fn from(body: String) -> Self {
        Response {
            status: None,
            headers: None,
            body: Some(body),
        }
    }
}

impl From<&str> for Response {
    fn from(body: &str) -> Self {
        Response {
            status: None,
            headers: None,
            body: Some(body.to_string()),
        }
    }
}

impl<'a> From<Markup<'a>> for Response {
    fn from(m: Markup<'a>) -> Self {
        let mut headers = HashMap::new();

        let header = ResponseHeaderType::EntityHeader(super::EntityHeader::ContentType(
            String::from("text/html"),
        ));

        let header_map = header.to_msg_header();
        let (k, v) = header_map.extract_name_val();
        headers.insert(k, v);
        (headers, m.to_string()).into()
    }
}

impl From<(StatusCode, HashMap<String, String>)> for Response {
    fn from((status, headers): (StatusCode, HashMap<String, String>)) -> Self {
        Response {
            status: Some(status),
            headers: Some(headers),
            body: None,
        }
    }
}

impl From<(StatusCode, String)> for Response {
    fn from((status, body): (StatusCode, String)) -> Self {
        Response {
            status: Some(status),
            headers: None,
            body: Some(body),
        }
    }
}

impl<'a> From<(StatusCode, Markup<'a>)> for Response {
    fn from((status, body): (StatusCode, Markup)) -> Self {
        Response {
            status: Some(status),
            headers: None,
            body: Some(body.to_string()),
        }
    }
}

impl From<(HashMap<String, String>, String)> for Response {
    fn from((headers, body): (HashMap<String, String>, String)) -> Self {
        Response {
            status: None,
            headers: Some(headers),
            body: Some(body),
        }
    }
}

impl From<(StatusCode, HashMap<String, String>, String)> for Response {
    fn from((status, headers, body): (StatusCode, HashMap<String, String>, String)) -> Self {
        Response {
            status: Some(status),
            headers: Some(headers),
            body: Some(body),
        }
    }
}

impl<'a> From<(StatusCode, HashMap<String, String>, Markup<'a>)> for Response {
    fn from((status, headers, body): (StatusCode, HashMap<String, String>, Markup<'a>)) -> Self {
        Response {
            status: Some(status),
            headers: Some(headers),
            body: Some(body.to_string()),
        }
    }
}

impl From<Result<Response, Response>> for FullResponse {
    fn from(r: Result<Response, Response>) -> Self {
        let (status_code, headers, body) = match r {
            Ok(r) => match (r.status, r.headers, r.body) {
                (Some(s), Some(h), b) => (s, h, b),
                (None, Some(h), b) => (StatusCode::OK, h, b),
                (Some(s), None, b) => (s, HashMap::new(), b),
                (None, None, b) => (StatusCode::OK, HashMap::new(), b),
            },
            Err(r) => match (r.status, r.headers, r.body) {
                (Some(s), Some(h), b) => (s, h, b),
                (None, Some(h), b) => (StatusCode::InternalServerError, h, b),
                (Some(s), None, b) => (s, HashMap::new(), b),
                (None, None, b) => (StatusCode::InternalServerError, HashMap::new(), b),
            },
        };

        FullResponse::new(status_code, headers, body)
    }
}

/// Return type placeholder for route functions
pub type ResponseResult = Result<Response, Response>;

/// This is a closure wrapper that allows for linking tuples of variadic
/// arguments to a concrete function
///
/// Without this trait, rust cannot understand that (A, B) can be associated
/// with Fn(A, B)...
struct Endpoint<F, A> {
    f: F,
    _marker: PhantomData<A>,
}

impl<F, A> Endpoint<F, A> {
    fn new(f: F) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

/// Used along with `FromRequest<T>` to implement A..G variadics for route functions
///
/// Both this trait and `FromRequest<T>` are mainly used for a bunch of code
/// gen. See `impl_handler` within the source code if you are curious how this
/// works.
pub trait Handler<A, T> {
    type Fn: Send + Sync + 'static;

    fn into_endpoint(self) -> Arc<dyn FromRequest<T>>;
}

/// Used along with `Handler<A,T>` to implement A..G variadics for route functions
///
/// Both this trait and `Handler<A,T>` are mainly used for a bunch of code
/// gen. See `impl_handler` within the source code if you are curious how this
/// works.
pub trait FromRequest<T: Send + Sync>: Send + Sync {
    fn apply_request(&self, req: InstanceRequest<T>) -> Result<BoxFuture, ()>;
}

macro_rules! impl_handler {
    ($($generic:ident),+) => {
        /// This is macro generated. See actual trait documentation instead
        impl<T, FF, Fut $(,$generic)+> Handler<($($generic,)+), T> for FF
        where
            T: Send + Sync,
            FF: Fn($($generic,)+) -> Fut + Send + Sync + 'static,
            ($($generic,)+): Extract<T, InstanceRequest<T>, ($($generic,)+)> + Send + Sync + 'static,
            Fut: Future<Output = ResponseResult> + Send + 'static,
        {
            type Fn = FF;

            fn into_endpoint(self) -> Arc<dyn FromRequest<T>> {
                Arc::new(Endpoint::new(self))
            }
        }
        /// This is macro generated. See actual trait documentation instead
        impl<T, FF, Fut $(,$generic)+> FromRequest<T> for Endpoint<FF, ($($generic,)+)>
        where
            T: Send + Sync,
            FF: Fn($($generic,)+) -> Fut + Send + Sync + 'static,
            ($($generic,)+): Extract<T, InstanceRequest<T>, ($($generic,)+)> + Send + Sync + 'static,
            Fut: Future<Output = ResponseResult> + Send + 'static,
        {
            fn apply_request(&self, req: InstanceRequest<T>) -> Result<BoxFuture, ()> {
                #[allow(non_snake_case)]
                let ($($generic,)+) = <($($generic,)+)>::from_request(PhantomData, req)?;
                Ok(Box::pin((self.f)($($generic,)+)))
            }
        }

    }
}

impl<T, FF, Fut> Handler<(), T> for FF
where
    T: Send + Sync,
    FF: Fn() -> Fut + Send + Sync + 'static,
    (): Extract<T, (), ()> + Send + Sync + 'static,
    Fut: Future<Output = ResponseResult> + Send + 'static,
{
    type Fn = FF;

    fn into_endpoint(self) -> Arc<dyn FromRequest<T>> {
        Arc::new(Endpoint::new(self))
    }
}
/// This is macro generated. See actual trait documentation instead
impl<T, FF, Fut> FromRequest<T> for Endpoint<FF, ()>
where
    T: Send + Sync,
    FF: Fn() -> Fut + Send + Sync + 'static,
    (): Extract<T, (), ()> + Send + Sync + 'static,
    Fut: Future<Output = ResponseResult> + Send + 'static,
{
    fn apply_request(&self, _req: InstanceRequest<T>) -> Result<BoxFuture, ()> {
        Ok(Box::pin((self.f)()))
    }
}
impl_handler!(A);
impl_handler!(A, B);
impl_handler!(A, B, C);
impl_handler!(A, B, C, D);
impl_handler!(A, B, C, D, E);
impl_handler!(A, B, C, D, E, F);
impl_handler!(A, B, C, D, E, F, G);

/// This wrapper is just `Arc<T>` and allows for the instance to be shared
/// across threads.
///
/// Whether to use `Mutex<T>` or `RwLock<T>` within this wrapper is up to
/// the dev.
pub type Instance<T> = Arc<T>;

pub struct Path<T>(pub T);

pub trait ToPath: Sized {
    fn into_path(path: URIPath) -> Result<Path<Self>, ()>;
}

impl ToPath for String {
    fn into_path(path: URIPath) -> Result<Path<Self>, ()> {
        Ok(Path(path.into_entire_path()))
    }
}

impl ToPath for Vec<String> {
    fn into_path(path: URIPath) -> Result<Path<Self>, ()> {
        Ok(Path(path.into_segments()))
    }
}

impl ToPath for HashSet<String> {
    fn into_path(path: URIPath) -> Result<Path<Self>, ()> {
        Ok(Path(path.into_segments().into_iter().collect()))
    }
}

pub struct Query<T: ToQuery>(pub T);

pub trait ToQuery: Sized + Deserialize {
    fn into_query(query: RequestQuery) -> Result<Query<Self>, ()>;
}

impl<T: Deserialize> ToQuery for T {
    fn into_query(query: RequestQuery) -> Result<Query<Self>, ()> {
        let s = query.parameters;
        match T::deserialize(s) {
            Ok(t) => Ok(Query(t)),
            Err(_) => Err(()),
        }
    }
}

// impl ToQuery for HashMap<String, String> {
//     fn into_query(query: RequestQuery) -> Result<Query<Self>, ()> {
//         Ok(Query(query.parameters))
//     }
// }

/// This trait helps rust figure out how to extract different combintations of tuples.
///
/// Outside of a few edge cases, implementations for this trait are mainly produced
/// via the `impl_extract_permutations!` proc macro.
///
/// # Order matters!
///
/// To reduce build time / combinatorial explosiveness, this trait only implements
/// ordered combination via proc_macro with the following order:
///
/// 1. Instance
/// 2. Method
/// 3. Path
/// 4. Query
/// 5. HTTPVersion
/// 6. RequestHeaders
/// 7. Body
///
/// ## Valid Example
///
/// ```rust
/// use zero::http::{Query, Method};
/// async fn some_valid_route(
///     method: Method,
///     Query(s): Query(String)
/// ) -> Result<(), ()> {
///     Ok(())
/// }
/// ```
/// ## Invalid Example
///
/// ```compile_fail
/// async fn some_invalid_route(
///     Query(s): Query(String), // Query must come after method, path, etc.
///     method: Method
/// ) -> Result<(), ()> {
///     Ok(())
/// }
/// ```
pub trait Extract<T, A, B>: Sized {
    fn from_request(_instance: PhantomData<T>, parts: A) -> Result<Self, ()>;
}

impl<T> Extract<T, (), ()> for () {
    fn from_request(_instance: PhantomData<T>, _req: ()) -> Result<Self, ()> {
        Ok(())
    }
}

impl<T> Extract<T, Instance<T>, Instance<T>> for Instance<T> {
    fn from_request(_instance: PhantomData<T>, req: Instance<T>) -> Result<Self, ()> {
        Ok(req)
    }
}

impl<T> Extract<T, Method, Method> for Method {
    fn from_request(_instance: PhantomData<T>, req: Method) -> Result<Self, ()> {
        Ok(req)
    }
}

impl<T, A: ToPath> Extract<T, URIPath, URIPath> for Path<A> {
    fn from_request(_instance: PhantomData<T>, path: URIPath) -> Result<Self, ()> {
        A::into_path(path)
    }
}

impl<T, A: ToQuery> Extract<T, RequestQuery, RequestQuery> for Query<A> {
    fn from_request(_instance: PhantomData<T>, query: RequestQuery) -> Result<Self, ()> {
        A::into_query(query)
    }
}

impl<T> Extract<T, HTTPVersion, HTTPVersion> for HTTPVersion {
    fn from_request(_instance: PhantomData<T>, version: HTTPVersion) -> Result<Self, ()> {
        Ok(version)
    }
}

impl<T> Extract<T, RequestHeaders, RequestHeaders> for RequestHeaders {
    fn from_request(_instance: PhantomData<T>, headers: RequestHeaders) -> Result<Self, ()> {
        Ok(headers)
    }
}

impl<T, A: ToBody> Extract<T, RequestBody, RequestBody> for Body<A> {
    fn from_request(_instance: PhantomData<T>, body: RequestBody) -> Result<Self, ()> {
        A::into_body(body)
    }
}

macros::impl_extract_permutations!();

type BoxFuture = Pin<Box<dyn Future<Output = ResponseResult> + Send>>;

pub struct Router<T: Send + Sync> {
    instance: Arc<T>,
    routes: HashMap<(&'static Method, &'static str), Arc<dyn FromRequest<T>>>,
}

impl<T: Send + Sync> Router<T> {
    pub fn new(instance: T) -> Self {
        Router {
            instance: instance.into(),
            routes: HashMap::new(),
        }
    }

    const OPTIONS: &'static Method = &Method::Options;
    const GET: &'static Method = &Method::Get;
    const HEAD: &'static Method = &Method::Head;
    const POST: &'static Method = &Method::Post;
    const PUT: &'static Method = &Method::Put;
    const DELETE: &'static Method = &Method::Delete;
    const TRACE: &'static Method = &Method::Trace;
    const CONNECT: &'static Method = &Method::Connect;

    /// This method is subject to change as role based
    /// routing is probably going to be a thing.
    pub fn route<A>(mut self, method: Method, s: &'static str, f: impl Handler<A, T>) -> Self {
        let m = match method {
            Method::Options => Self::OPTIONS,
            Method::Get => Self::GET,
            Method::Head => Self::HEAD,
            Method::Post => Self::POST,
            Method::Put => Self::PUT,
            Method::Delete => Self::DELETE,
            Method::Trace => Self::TRACE,
            Method::Connect => Self::CONNECT,
        };
        self.routes.insert((m, s), f.into_endpoint());
        self
    }
    pub fn include_zero_js(self) -> Self {
        async fn include_zero() -> ResponseResult {
            Ok(include_str!("../zero.js").into())
        }
        self.route(Method::Get, "/zero.js", include_zero)
    }

    pub async fn apply_request(&self, req: Request) -> FullResponse {
        let handle = match self.routes.get(&req.method_path()) {
            Some(handle) => handle.clone(),
            None => return FullResponse::new_simple(StatusCode::NotFound, None),
        };

        let req = InstanceRequest::from_request(self.instance.clone(), req);

        match handle.apply_request(req) {
            Ok(r) => {
                eprintln!("hit");
                r.await.into()
            }
            Err(_) => {
                eprintln!("hit2");
                FullResponse::new_simple(StatusCode::InternalServerError, None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::{StrParser, prelude::*};

    #[test]
    fn test_router() {
        // async fn body_handler(Body(s): Body<String>) -> Result<(), ()> {
        //     dbg!(s);
        //     Ok(())
        // }

        async fn method_handler3() -> ResponseResult {
            Ok("this is a test".into())
        }
        async fn method_handler(_method: Method) -> ResponseResult {
            Ok("this is a test".into())
        }
        async fn method_handler2(instance: Arc<usize>) -> ResponseResult {
            dbg!(instance);
            Ok(().into())
        }
        // async fn query_handler(Path(s): Path<String>) -> Result<(), ()> {
        //     dbg!(s);
        //     Ok(())
        // }

        // async fn qb_handler(Path(s): Path<String>, Body(b): Body<String>) -> Result<(), ()> {
        //     dbg!(s);
        //     dbg!(b);
        //     Ok(())
        // }
        // FIXME: need to implement from variadics to response trait
        //
        // e.g.
        // impl From<(A, B)> for FullResponse {
        //     fn from((a, b): (A, B)) -> Self {
        //         //DO STUFF
        //     }
        // }
        let _router = Router::new(1_usize)
            .route(Method::Get, "some_route3", method_handler3)
            .route(Method::Get, "some_route", method_handler)
            .route(Method::Get, "some_route2", method_handler2);

        let mut parser = StrParser::from_str(
            "GET /some_route?some=query HTTP/1.1\r\nHost: 127.0.0.1:8000\r\nUser-Agent: curl/8.14.1\r\nContent-Length: 14\r\nAccept: */*\r\n\r\nthis is a test    ",
        );
        let req = Request::parse(&mut parser).unwrap();

        // router.apply_request(req);
        let _ = InstanceRequest::from_request(0.into(), req);

        // assert!(false);
    }
}
