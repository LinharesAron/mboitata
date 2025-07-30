pub mod allow_list;
pub mod config;
mod consumer;
pub mod proxy;
pub mod stages;

use std::fs;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::stages::intercepted::InterceptedResponse;
use crate::{
    proxy::{
        certs::{CertificateManager, create_ca_certificate},
        start_proxy,
    },
    stages::setup::initialize_stages,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install CryptoProvider");

    let (config, allow_list) = config::load();

    let (tx, rx) = mpsc::channel::<InterceptedResponse>(1000);

    let cert_dir = config.certs_dir;
    if !cert_dir.exists() {
        fs::create_dir_all(&cert_dir)?;
    }

    let ca_cert_path = cert_dir.join("ca-cert.pem");
    let ca_key_path = cert_dir.join("ca-key.pem");

    let (ca_cert_pem, ca_key_pem) =
        if Path::new(&ca_cert_path).exists() && Path::new(&ca_key_path).exists() {
            (
                fs::read_to_string(ca_cert_path)?,
                fs::read_to_string(ca_key_path)?,
            )
        } else {
            let (cert, key) = create_ca_certificate()?;
            fs::write(ca_cert_path, &cert)?;
            fs::write(ca_key_path, &key)?;
            (cert, key)
        };

    let cert_manager = Arc::new(CertificateManager::new(
        ca_cert_pem.as_str(),
        ca_key_pem.as_str(),
    )?);

    let dispatcher = initialize_stages(allow_list, config.output);
    let (_, _) = tokio::join!(
        consumer::start_consumer(rx, dispatcher),
        start_proxy(tx, cert_manager, &config.port)
    );
    Ok(())
}
