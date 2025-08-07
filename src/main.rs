mod allow_list;
mod config;
mod consumer;
mod navigator;
mod proxy;
mod analyzer;

use std::fs;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use tokio::signal;
use tokio::sync::{broadcast, mpsc};

use std::future::Future;

use crate::analyzer::intercepted::InterceptedResponse;
use crate::{
    proxy::{
        certs::{CertificateManager, create_ca_certificate},
        start_proxy,
    },
    analyzer::setup::initialize_stages,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install CryptoProvider");

    let (config, allow_list) = config::load();

    let (tx, rx) = mpsc::channel::<InterceptedResponse>(1000);
    let (kill, _): (broadcast::Sender<()>, broadcast::Receiver<()>) = broadcast::channel(1);

    let cert_dir = config.get_certs_dir();
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

    let (dispatcher, stage_handle) = initialize_stages(allow_list, config.output);

    let proxy_server = format!("0.0.0.0:{}", &config.port);
    let kill_signal = kill.clone();
    let urls_futures: Pin<Box<dyn Future<Output = ()> + Send>> = match config.urls.clone() {
        Some(urls) if !urls.is_empty() => {
            println!("🌐 Iniciando navegador com URLs...");
            Box::pin(navigator::run(
                urls,
                4,
                proxy_server.to_string(),
                kill_signal,
            ))
        }
        _ => {
            println!("⚠️ Nenhuma URL fornecida. Navegador não será executado.");
            Box::pin(async {})
        }
    };

    let kill_signal = kill.clone();
    let mut kill_receiver = kill_signal.subscribe();
    let kill_listener = tokio::spawn(async move {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("🧨 Ctrl+C detectado. Enviando kill...");
                let _ = kill_signal.send(());
            }

            _ = kill_receiver.recv() => {
                println!("📴 Kill já foi enviado. kill_listener encerrando.");
            }
        }
    });

    let _ = tokio::join!(
        start_proxy(tx, kill.subscribe(), cert_manager, &proxy_server),
        consumer::start_consumer(rx, dispatcher),
        urls_futures,
        stage_handle,
        kill_listener
    );
    Ok(())
}
