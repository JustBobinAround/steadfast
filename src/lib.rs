#![allow(clippy::from_str_radix_10)] //<< I just prefer this, idk
#![doc = include_str!("../README.md")]
extern crate self as steadfast;
pub mod errors;
pub mod html;
pub mod http;
pub mod stream_writer;
pub mod variadics;

pub use steadfast_async;
pub use steadfast_db;
pub use steadfast_json;
/// proc macro to wrap main around async executor
///
/// This macro is written pretty badly right now. See the macros workspace for implementation details
///
/// # Example Usage
/// ```text
/// use steadfast::http::routing::Router;
/// use steadfast::http::server::HttpServer;
/// #[steadfast::main]
/// async fn main() -> Result<(), ()> {  // allows async usage
///    let router = Router::new(());
///    
///    let mut server = HttpServer::from_router(router);
///    
///    let serve = server.serve("127.0.0.1:8000").await; // allows await usage
///    
///    Ok(())
/// }
/// ```
///
/// ## Limitations
///
/// Currently this only supports `Result<(), ()>` types because I don't feel
/// like making a full token parser yet.
///
/// Additionally, this macro expects the crate to have a name of "steadfast". Anything
/// else will break the macro.
pub use steadfast_macros::{Deserialize, STable, ToDatabaseBytes, html, main};
pub use steadfast_parsing;
pub use steadfast_serializer;
pub use steadfast_uuid::UUID;
