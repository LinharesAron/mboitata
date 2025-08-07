use async_trait::async_trait;

use crate::analyzer::{
    event::Dispatcher,
    intercepted::InterceptedResponse,
    stage::{Stage, StageId},
    stages::{js::analyzer::run_js_analysis, utils::file_name},
};

pub struct ScanJsStage;

impl ScanJsStage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Stage for ScanJsStage {
    async fn process(&self, dispatcher: Dispatcher, resp: InterceptedResponse) {
        if let Some(text) = resp.get_body() {
            match run_js_analysis(&resp.path, text.as_str()) {
                Ok(result) => {
                    let scheme = resp.scheme.clone();
                    let host = resp.host.clone();
                    let path = file_name(&resp.path);

                    if !result.vars.is_empty() {
                        let mut vars = result
                            .vars
                            .iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect::<Vec<_>>();

                        vars.sort();
                        let content = vars.join("\n");

                        let label = "VARS";
                        dispatcher.emit(
                            StageId::SaveFile,
                            InterceptedResponse {
                                scheme: scheme.clone(),
                                host: host.clone(),
                                path: format!("findings/{}/{}", &path, label),
                                content_encoding: "identity".into(),
                                content_type: "".into(),
                                body: content.as_bytes().to_vec(),
                            },
                        );
                    }

                    if !result.calls.is_empty() {
                        let content = &result
                            .calls
                            .iter()
                            .map(|c| {
                                let auth = c.authorization.as_deref().unwrap_or("-");
                                format!("Url: {}\nMethod: {}\nAuth: {}\n", c.url, c.method, auth)
                            })
                            .collect::<Vec<_>>()
                            .join("==================================================\n");

                        let label = "CALLS";
                        dispatcher.emit(
                            StageId::SaveFile,
                            InterceptedResponse {
                                scheme: scheme.clone(),
                                host: host.clone(),
                                path: format!("findings/{}/{}", &path, label),
                                content_encoding: "identity".into(),
                                content_type: "".into(),
                                body: content.as_bytes().to_vec(),
                            },
                        );
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Erro ao extrair informações do javascript {}: {}",
                        resp.path, err
                    );
                }
            }
        }
    }
}
