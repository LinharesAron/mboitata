use std::pin::Pin;

use http_body_util::combinators::BoxBody;
use hyper::{
    Method, Request, Response,
    body::{Bytes, Incoming},
};
use tokio::{net::TcpStream, sync::mpsc::Sender};

use crate::{
    proxy::{Intercept, empty, extract_host_port, handle_response, https::HttpsIntercept},
    analyzer::intercepted::InterceptedResponse,
};

#[derive(Debug, Clone)]
pub struct HttpIntercept {
    pub tx: Sender<InterceptedResponse>,
    pub upgraded: HttpsIntercept,
}

const SCHEME: &str = "http";

impl Intercept for HttpIntercept {
    type Request = Request<Incoming>;
    type Response = Response<BoxBody<Bytes, hyper::Error>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let tx = self.tx.clone();
        let upgraded = self.upgraded.clone();
        Box::pin(async move {
            if Method::CONNECT == req.method() {
                upgraded.upgraded(req).await
            } else {
                let (host, port) =
                    match extract_host_port(&req, Some(SCHEME)) {
                        Ok((h, p)) => (h, p),
                        Err(e) => {
                            eprintln!("Erro ao extrair host/port: {:?}", e);
                            return Ok(Response::new(empty()));
                        }
                    };

                match TcpStream::connect((host.clone(), port)).await {
                    Ok(server_tls_stream) => {
                        handle_response(SCHEME.to_string(), host, req, tx.clone(), server_tls_stream).await
                    }
                    Err(err) => {
                        eprintln!("Erro ao estabelecer TLS com o servidor {}: {}", host, err);
                        Ok(Response::new(empty()))
                    }
                }
            }
        })
    }
}
