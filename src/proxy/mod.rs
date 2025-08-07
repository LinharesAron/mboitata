pub mod certs;

mod http;
mod https;

use std::sync::Arc;

use anyhow::bail;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{
    HeaderMap, Request, Response, StatusCode, Version,
    body::{Bytes, Incoming},
    header::{self, HeaderValue},
    service::Service,
};
use hyper_util::rt::TokioIo;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::{broadcast, mpsc::Sender},
};

use crate::proxy::http::HttpIntercept;
use crate::proxy::https::HttpsIntercept;
use crate::{proxy::certs::CertificateManager, analyzer::intercepted::InterceptedResponse};

type ClientBuilder = hyper::client::conn::http1::Builder;
type ServerBuilder = hyper::server::conn::http1::Builder;

trait Intercept: Send + Sync + 'static {
    type Request: Send + 'static;
    type Response: Send + 'static;
    type Error: Send + 'static;
    type Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;

    fn call(&self, req: Self::Request) -> Self::Future;
}

#[derive(Debug, Clone)]
struct InterceptService<I: Intercept> {
    base: I,
}

impl<I: Intercept> Service<I::Request> for InterceptService<I> {
    type Error = I::Error;
    type Response = I::Response;
    type Future = I::Future;

    fn call(&self, req: I::Request) -> Self::Future {
        self.base.call(req)
    }
}

pub async fn start_proxy(
    tx: Sender<InterceptedResponse>,
    mut kill_signal: broadcast::Receiver<()>,
    cert_manager: Arc<CertificateManager>,
    proxy_server: &str,
) -> anyhow::Result<()> {
    println!("Starting listener on {proxy_server}");

    let listener = TcpListener::bind(proxy_server).await?;

    loop {
        tokio::select! {
            _ = kill_signal.recv() => {
                println!("🛑 Proxy recebeu kill. Encerrando listener...");
                return Ok(());
            }
            res = listener.accept() => {
                match res {
                    Ok((stream, _)) => {
                        let io = TokioIo::new(stream);

                        let https_intercept = HttpsIntercept {
                            tx: tx.clone(),
                            cert_manager: cert_manager.clone(),
                        };

                        let http_intercept = HttpIntercept {
                            tx: tx.clone(),
                            upgraded: https_intercept.clone(),
                        };

                        let http_service = InterceptService {
                            base: http_intercept.clone(),
                        };

                        tokio::task::spawn(async move {
                            if let Err(err) = ServerBuilder::new()
                                .preserve_header_case(true)
                                .title_case_headers(true)
                                .serve_connection(io, http_service)
                                .with_upgrades()
                                .await
                            {
                                println!("Failed to serve connection: {:?}", err);
                            }
                        });
                    }

                    Err(err) => {
                        eprintln!("Erro ao aceitar conexão: {:?}", err);
                    }
                }
            }
        }
    }
}

fn rebuild_response(
    status: StatusCode,
    version: Version,
    headers: &HeaderMap,
    body: Bytes,
) -> Response<BoxBody<Bytes, hyper::Error>> {
    let mut builder = Response::builder().status(status).version(version);

    for (k, v) in headers {
        if let Ok(value) = HeaderValue::from_str(v.to_str().unwrap_or("")) {
            builder = builder.header(k.clone(), value);
        }
    }

    builder.body(full(body)).unwrap()
}

fn create_response(
    msg: String,
    status_code: StatusCode,
) -> Response<BoxBody<hyper::body::Bytes, hyper::Error>> {
    let mut resp = Response::new(full(msg));
    *resp.status_mut() = status_code;
    resp
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

async fn handle_response<T>(
    scheme: String,
    host: String,
    req: Request<Incoming>,
    tx: Sender<InterceptedResponse>,
    io: T,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let server_io = TokioIo::new(io);
    let uri = req.uri().clone();

    let (mut sender, conn) = ClientBuilder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(server_io)
        .await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
        }
    });

    let mut resp = sender.send_request(req).await?;
    let status = resp.status();
    let version = resp.version();

    let body_bytes = resp.body_mut().collect().await?.to_bytes();
    let body_for_client = body_bytes.clone();

    let intercepted = InterceptedResponse::new(
        uri.path().to_string(),
        scheme,
        host,
        resp.headers(),
        body_bytes.to_vec(),
    );

    if let Err(err) = tx.send(intercepted).await {
        eprintln!("Erro ao enviar para fila: {err}");
    }

    let new_resp = rebuild_response(status, version, resp.headers(), body_for_client);
    Ok(new_resp)
}

pub fn extract_host_port(
    req: &Request<Incoming>,
    scheme: Option<&str>,
) -> anyhow::Result<(String, u16)> {
    let uri = req.uri();

    if let Some(host) = uri.host() {
        let port = uri
            .port_u16()
            .unwrap_or_else(|| default_port(uri.scheme_str().or(scheme)));
        return Ok((host.to_string(), port));
    }

    if let Some(host_hdr) = host_like_header(req.headers()) {
        let (host, port) = match split_host_port(host_hdr) {
            Some((h, p)) => (h.to_string(), p),
            None => (host_hdr.to_string(), default_port(scheme)),
        };
        return Ok((host, port));
    }

    bail!("não foi possível determinar host/port (URI sem host e sem Host header)");
}

fn default_port(scheme: Option<&str>) -> u16 {
    match scheme {
        Some("https") => 443,
        _ => 80,
    }
}

fn host_like_header(headers: &HeaderMap) -> Option<&str> {
    if let Some(v) = headers.get(header::HOST) {
        if let Ok(s) = v.to_str() {
            return Some(s);
        }
    }
    if let Some(v) = headers.get(":authority") {
        if let Ok(s) = v.to_str() {
            return Some(s);
        }
    }
    None
}

fn split_host_port(s: &str) -> Option<(&str, u16)> {
    if let Some(stripped) = s.strip_prefix('[') {
        if let Some(end) = stripped.find(']') {
            let host = &stripped[..end];
            let rest = &stripped[end + 1..];
            let port = rest.strip_prefix(':').and_then(|p| p.parse().ok())?;
            return Some((host, port));
        }
        return None;
    }

    if let Some((h, p)) = s.rsplit_once(':') {
        if let Ok(port) = p.parse() {
            return Some((h, port));
        }
    }
    None
}
