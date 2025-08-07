use std::collections::{HashMap, HashSet};

use crate::analyzer::{
    event::Dispatcher,
    intercepted::InterceptedResponse,
    stage::{Stage, StageId},
    stages::utils::file_name,
};
use async_trait::async_trait;
use regex::Regex;

pub struct ScanStage;

impl ScanStage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Stage for ScanStage {
    async fn process(&self, dispatcher: Dispatcher, resp: InterceptedResponse) {
        if let Some(body) = resp.get_body() {
            let token_regex =
                Regex::new(r"(eyJ[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+)").unwrap();
            let bearer_regex = Regex::new(r"(?i)bearer\s+([A-Za-z0-9\-_\.=]+)").unwrap();
            let api_key_regex = Regex::new(
                r#"(?i)(api|access|secret)[_\-]?key["']?\s*[:=]\s*["']?[A-Za-z0-9\-_]{16,}"#,
            )
            .unwrap();
            let url_regex = Regex::new(r#"https?://[^\s"'<>]+"#).unwrap();

            let mut findings: HashMap<&str, HashSet<String>> = HashMap::new();

            for (label, regex) in &[
                ("JWT", token_regex),
                ("Bearer Token", bearer_regex),
                ("API Key", api_key_regex),
                ("URL", url_regex),
            ] {
                for mat in regex.find_iter(body.as_str()) {
                    println!(
                        "[!] Possível {} detectado em resposta de {}: {}",
                        label,
                        resp.host,
                        mat.as_str()
                    );

                    findings
                        .entry(label)
                        .or_default()
                        .insert(mat.as_str().to_string());
                }
            }

            let scheme = resp.scheme.clone();
            let host = resp.host.clone();
            let path = file_name(&resp.path);

            for (label, items) in findings {
                if items.is_empty() {
                    continue;
                }

                let mut v = items.iter().cloned().collect::<Vec<String>>();
                v.sort();

                let content = v.join("\n");
                dispatcher.emit(
                    StageId::SaveFile,
                    InterceptedResponse {
                        scheme: scheme.clone(),
                        host: host.clone(),
                        path: format!("findings/{}/{}", path, label),
                        content_encoding: "identity".into(),
                        content_type: "".into(),
                        body: content.as_bytes().to_vec(),
                    },
                );
            }
        }
    }
}
