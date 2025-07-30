use std::path::PathBuf;

use clap::Parser;
use url::Url;

use crate::allow_list::AllowList;

#[derive(Debug, Parser)]
#[command(
    name = "mboitata",
    version,
    about = "HTTP(S) MITM proxy para capturar e analisar JS/JS.map"
)]
pub struct Config {
    #[arg(short, long, value_delimiter = ',', required = false)]
    pub urls: Vec<String>,

    #[arg(short, long, env = "MBOITATA_PORT", default_value = "8085")]
    pub port: String,

    #[arg(short, long, env = "MBOITATA_OUTPUT", default_value = "output")]
    pub output: PathBuf,

    #[arg(short, long, env = "MBOITATA_CERTS", default_value = "certs")]
    pub certs_dir: PathBuf,

    #[arg(long, default_value_t = false)]
    pub pretty: bool,

    #[arg(long, env = "MB_ALLOWLIST", value_delimiter = ',')]
    pub allow_list: Vec<String>,
}

pub fn load() -> (Config, AllowList) {
    let config = Config::parse();

    let mut allow_list = config.allow_list.clone();

    if !config.urls.is_empty() {
        println!("[INFO] Rodando crawler nas URLs fornecidas...");
        let domains = extract_domains_from_urls(&config.urls);
        println!("[INFO] Domínios extraídos do crawler: {:?}", domains);

        for domain in domains {
            if !allow_list.contains(&domain) {
                allow_list.push(domain);
            }
        }
    }

    if allow_list.is_empty() {
        println!(
            "[WARN] Nenhuma allowlist fornecida. O proxy irá capturar de todos os domínios.\n\
            [TIP] Use --allowlist ou --urls para limitar o escopo."
        );
    }

    (config, AllowList::new(allow_list))
}

fn extract_domains_from_urls(urls: &[String]) -> Vec<String> {
    let mut domains = vec![];

    for u in urls {
        if let Ok(parsed) = Url::parse(u) {
            if let Some(host) = parsed.host_str() {
                domains.push(host.to_string());
            }
        }
    }

    domains
}
