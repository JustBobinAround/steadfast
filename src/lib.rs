#![allow(clippy::from_str_radix_10)] //<< I just prefer this, idk
#![doc = include_str!("../README.md")]
extern crate self as zero;
pub mod async_runtime;
// pub mod db;
pub mod errors;
pub mod html;
pub mod http;
// pub mod parsing;
pub mod stream_writer;
pub mod variadics;

pub use db;
pub use json;
/// proc macro to wrap main around async executor
///
/// This macro is written pretty badly right now. See the macros workspace for implementation details
///
/// # Example Usage
/// ```rust
/// use zero::http::routing::Router;
/// use zero::http::server::HttpServer;
/// #[zero::main]
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
/// Additionally, this macro expects the crate to have a name of "zero". Anything
/// else will break the macro.
pub use macros::{Deserialize, ToDatabaseBytes, ZeroTable, html, main};
pub use parsing;
pub use serializer;
pub use uuid::UUID;
