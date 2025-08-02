use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::time::Duration;
use tokio::{sync::broadcast, task::{self, JoinError}};

pub async fn run(urls: Vec<String>, workers: usize, proxy: String, kill_sign: broadcast::Sender<()>) {
    println!("🕐 Esperando proxy ficar pronto...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let chunked = urls
        .chunks((urls.len() + workers - 1) / workers)
        .map(|c| c.to_vec())
        .collect::<Vec<_>>();

    let mut handles = vec![];

    for (id, urls_chunk) in chunked.into_iter().enumerate() {
        let handle = navigate_block(urls_chunk, id, proxy.clone());
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    println!("🏁 Navegação finalizada. Enviando kill para os outros módulos.");
    let _ = kill_sign.send(());
}

async fn navigate_block(urls: Vec<String>, id: usize, proxy: String) -> Result<(), JoinError> {
    task::spawn_blocking(move || {
        println!("🔥 Mboîtatá worker {id} acendeu sua tocha");

        let launch_options = LaunchOptionsBuilder::default()
            .headless(true)
            .proxy_server(Some(proxy.as_str()))
            .ignore_certificate_errors(true)
            .build()
            .unwrap();

        let browser = match Browser::new(launch_options) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("🚫 Worker {id} falhou ao acender o Chrome: {e}");
                return;
            }
        };

        let tab = match browser.new_tab() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("⚠️ Worker {id} não conseguiu abrir nova aba: {e}");
                return;
            }
        };

        for url in urls {
            println!("🌐 Worker {id} navegando para: {url}");
            if let Err(e) = tab.navigate_to(&url) {
                eprintln!("❌ Worker {id} falhou ao navegar: {e}");
                continue;
            }
            let _ = tab.wait_for_element("body");
            std::thread::sleep(Duration::from_secs(2));
        }

        println!("✅ Worker {id} apagou sua chama com sucesso.");
    })
    .await
}
