use async_trait::async_trait;

use crate::{
    allow_list::AllowList,
    analyzer::{
        event::Dispatcher,
        intercepted::InterceptedResponse,
        stage::{Stage, StageId},
    },
};

pub struct FilterStage {
    allow_list: AllowList,
}

impl FilterStage {
    pub fn new(allow_list: AllowList) -> Self {
        Self { allow_list }
    }
}

#[async_trait]
impl Stage for FilterStage {
    async fn process(&self, dispatcher: Dispatcher, resp: InterceptedResponse) {
        if !self.allow_list.in_scope(&resp.host) {
            println!("[Filter] Fora do escopo: {}", resp.host);
            return;
        }

        if resp.content_type.starts_with("image/") || resp.path.ends_with("css") {
            println!("[Filter] Ignorando image/css: {}", resp.path);
            return;
        }

        if resp.content_type.contains("javascript") {
            println!("[Filter] JS/MAP detectado: {}", resp.path);
            dispatcher.emit(StageId::Map, resp.clone());
            return;
        }

        println!("[Filter] Conteúdo geral: {}", resp.path);
        dispatcher.emit(StageId::Scan, resp.clone());
    }
}
