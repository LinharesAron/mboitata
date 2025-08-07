use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use std::time::Duration;
use tokio::{
    sync::broadcast,
    task::{self, JoinError}, time::Instant,
};

pub async fn run(
    urls: Vec<String>,
    workers: usize,
    proxy: String,
    kill_sign: broadcast::Sender<()>,
) {
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
            .sandbox(false)
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

            if let Err(e) = wait_until_navigated_with_timeout(&tab, Duration::from_secs(15)) {
                eprintln!("⏱️ Worker {id} timeout ao esperar carregamento: {e}");
                continue;
            }

            if let Err(e) = wait_for_element_with_timeout(&tab, "body", Duration::from_secs(10)) {
                eprintln!("⚠️ Worker {id} body não apareceu: {e}");
                continue;
            }

            println!("✅ Worker {id} página carregada: {url}");
            std::thread::sleep(Duration::from_secs(3));
        }

        println!("✅ Worker {id} apagou sua chama com sucesso.");
    })
    .await
}

fn wait_until_navigated_with_timeout(tab: &Tab, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match tab.wait_until_navigated() {
            Ok(_) => return Ok(()),
            Err(e) => {
                if start.elapsed() >= timeout {
                    return Err(format!("timeout: {e}"));
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
    Err("timeout atingido".into())
}

fn wait_for_element_with_timeout(
    tab: &Tab,
    selector: &str,
    timeout: Duration,
) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match tab.wait_for_element_with_custom_timeout(selector, Duration::from_secs(2)) {
            Ok(_) => return Ok(()),
            Err(_) => {
                if start.elapsed() >= timeout {
                    return Err(format!("Elemento '{selector}' não encontrado"));
                }
            }
        }
    }
    Err(format!("timeout: elemento '{selector}'"))
}
