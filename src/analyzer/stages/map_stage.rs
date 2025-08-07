use async_trait::async_trait;
use hyper::StatusCode;
use regex::Regex;
use reqwest::Client;
use sourcemap::{DecodedMap, decode_slice};

use crate::analyzer::{
    event::Dispatcher,
    intercepted::InterceptedResponse,
    stage::{Stage, StageId},
};

pub struct MapStage;

impl MapStage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Stage for MapStage {
    async fn process(&self, dispatcher: Dispatcher, resp: InterceptedResponse) {
        if let Some(body_str) = resp.get_body() {
            let re = Regex::new(r"(?m)//[#@]\s*sourceMappingURL\s*=\s*(?P<map>[^\s]+)").unwrap();

            let mut to_search = vec![];
            if let Ok(base) = resp.get_url() {
                for caps in re.captures_iter(&body_str) {
                    if let Some(path) = caps.name("map") {
                        if let Ok(url) = base.join(path.as_str()) {
                            to_search.push(url);
                        }
                    }
                }

                if to_search.is_empty() {
                    println!("[Map] Nenhum sourceMappingURL explícito. Tentando fallback.");
                    if let Ok(fallback) = base.join(&format!("{}.map", base.path())) {
                        to_search.push(fallback);
                    }
                }
            }

            let client = Client::builder().user_agent("mboi-tata/0.1").build();

            if let Ok(client) = client {
                for search in to_search {
                    if let Some(content) = fetch_map(&client, search.as_str()).await {
                        let scheme = resp.scheme.clone();
                        let host = resp.host.clone();
                        for (name, content) in extract_source_maps(content.as_bytes()) {
                            if let Ok(url) = search.join(&name) {
                                dispatcher.emit(
                                    StageId::SaveFile,
                                    InterceptedResponse {
                                        scheme: scheme.clone(),
                                        host: host.clone(),
                                        path: url.path().to_string(),
                                        content_encoding: "identity".to_string(),
                                        content_type: "application/javascript".into(),
                                        body: content.as_bytes().to_vec(),
                                    },
                                );
                            }
                        }
                    }
                }
            }

            dispatcher.emit(StageId::SaveFile, resp.clone());
            dispatcher.emit(StageId::Scan, resp.clone());
            dispatcher.emit(StageId::JsScan, resp);
        }
    }
}

fn extract_source_maps(content: &[u8]) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = Vec::new();

    if let Ok(DecodedMap::Regular(sm)) = decode_slice(content) {
        for i in 0..sm.get_source_count() {
            if let (Some(name), Some(content)) = (sm.get_source(i), sm.get_source_contents(i)) {
                result.push((
                    format!(
                        "/sourcemap/{}",
                        name.replace("webpack://", "")
                            .replace("webpack:/", "")
                            .replace("..", "")
                            .replace(":", "")
                            .replace("//", "/")
                            .strip_prefix('/')
                            .unwrap_or(&name)
                    ),
                    content.to_string(),
                ));
            }
        }
    }

    result
}

async fn fetch_map(client: &Client, url: &str) -> Option<String> {
    match client.get(url).send().await {
        Ok(resp) => {
            if resp.status() == StatusCode::OK {
                match resp.text().await {
                    Ok(content) => {
                        println!("[MapFetcher] ✅ Sucesso: {} ({} bytes)", url, content.len());
                        Some(content)
                    }
                    Err(e) => {
                        println!("[MapFetcher] ❌ Erro ao ler corpo de {}: {}", url, e);
                        None
                    }
                }
            } else {
                println!("[MapFetcher] ❌ {} → {}", url, resp.status());
                None
            }
        }
        Err(e) => {
            println!("[MapFetcher] ❌ Falha na requisição {}: {}", url, e);
            None
        }
    }
}
