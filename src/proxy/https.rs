use std::{pin::Pin, sync::Arc};

use http_body_util::combinators::BoxBody;
use hyper::{
    Request, Response,
    body::{Bytes, Incoming},
    http,
    upgrade::Upgraded,
};
use hyper_util::rt::TokioIo;
use pki_types::ServerName;
use rustls::RootCertStore;
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::{
    proxy::{
        Intercept, InterceptService, ServerBuilder, certs::CertificateManager, empty,
        extract_host_port, full, handle_response,
    },
    stages::intercepted::InterceptedResponse,
};

#[derive(Debug, Clone)]
pub struct HttpsIntercept {
    pub tx: Sender<InterceptedResponse>,
    pub cert_manager: Arc<CertificateManager>,
}

const SCHEME: &str = "https";

impl HttpsIntercept {
    pub async fn upgraded(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        if let Some(addr) = host_addr(req.uri()) {
            let this = self.clone();

            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = this.mitm_tunnel(upgraded, addr).await {
                            eprintln!("HTTPS MITM error: {}", e);
                        }
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });

            Ok(Response::new(empty()))
        } else {
            eprintln!("CONNECT host is not socket addr: {:?}", req.uri());
            let mut resp = Response::new(full("CONNECT must be to a socket address"));
            *resp.status_mut() = http::StatusCode::BAD_REQUEST;
            Ok(resp)
        }
    }

    async fn mitm_tunnel(&self, upgraded: Upgraded, addr: String) -> anyhow::Result<()> {
        let hostname = addr
            .split_once(':')
            .map(|(h, _)| h)
            .unwrap_or(addr.as_str());

        let server_config = self.cert_manager.get_server_config(hostname).await?;
        let acceptor = TlsAcceptor::from(server_config);

        let client_tls_stream = acceptor.accept(TokioIo::new(upgraded)).await?;

        let client_io = TokioIo::new(client_tls_stream);

        let https_server = InterceptService { base: self.clone() };

        if let Err(err) = ServerBuilder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .serve_connection(client_io, https_server)
            .await
        {
            eprintln!("Failed to serve HTTPS connection: {:?}", err);
        }

        Ok(())
    }
}

impl Intercept for HttpsIntercept {
    type Request = Request<Incoming>;
    type Response = Response<BoxBody<Bytes, hyper::Error>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let tx = self.tx.clone();

        Box::pin(async move {
            let (host, port) =
                match extract_host_port(&req, Some(SCHEME)) {
                    Ok((h, p)) => (h, p),
                    Err(e) => {
                        eprintln!("Erro ao extrair host/port: {:?}", e);
                        return Ok(Response::new(empty()));
                    }
                };

            let server_name = ServerName::try_from(host.clone()).unwrap();

            let stream = TcpStream::connect((host.clone(), port)).await.unwrap();

            let root_store = RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            };
            let client_config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(client_config));
            match connector.connect(server_name, stream).await {
                Ok(server_tls_stream) => {
                    handle_response(SCHEME.to_string(), host, req, tx.clone(), server_tls_stream).await
                }
                Err(err) => {
                    eprintln!("Erro ao estabelecer TLS com o servidor {}: {}", host, err);
                    Ok(Response::new(empty()))
                }
            }
        })
    }
}

fn host_addr(uri: &http::Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}
