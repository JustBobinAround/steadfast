use super::response::{Response, StatusCode};
use super::routing::Router;
use crate::stream_writer::StreamWritable;
use crate::{errors::ZeroErr, http::request::Request, steadfast_parsing::StreamParser};
use std::net::TcpListener;
use std::sync::Arc;

pub struct HttpServer<T: Send + Sync + 'static> {
    router: Arc<Router<T>>,
}

// type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

impl<T: Send + Sync> HttpServer<T> {
    pub fn from_router(router: Router<T>) -> Self {
        HttpServer {
            router: router.into(),
        }
    }
    pub async fn serve<IP>(&mut self, ip: IP) -> Result<(), ZeroErr>
    where
        IP: std::fmt::Display,
    {
        let listener = TcpListener::bind(ip.to_string()).map_err(|_| ZeroErr::FailedToOpen)?;

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let router = self.router.clone();
                    match Request::parse_from_stream(&mut stream) {
                        Ok(request) => {
                            let response = router.apply_request(request).await;
                            let _ = response.write_to_stream(&mut stream);
                        }
                        Err(_) => {
                            let response = Response::new_simple(StatusCode::BadRequest, None);
                            let _ = response.write_to_stream(&mut stream);
                        }
                    }
                }
                Err(e) => eprintln!("connection failed: {}", e),
            }
        }

        Ok(())
    }
}
