use std::{fs, path::PathBuf};

use async_trait::async_trait;

use crate::analyzer::{event::Dispatcher, intercepted::InterceptedResponse, stage::Stage};

pub struct SaveFileStage {
    output_dir: PathBuf
}

impl SaveFileStage {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }
}

#[async_trait]
impl Stage for SaveFileStage {
    async fn process(&self, _: Dispatcher, resp: InterceptedResponse) {
        if let Some(body) = resp.get_body() {
            if let Some(dir) = resp.safe_join(&self.output_dir) {
                if let Some(parent) = dir.parent() {
                    if let Err(e) = fs::create_dir_all(&parent) {
                        eprintln!("Erro ao criar diretório {:?}: {}", parent, e);
                        return;
                    }
                }
                
                println!("[SaveFile] salvado o arquivo {:?}", &dir);
                if let Err(e) = fs::write(&dir, &body) {
                    eprintln!("Erro ao salvar o arquivo {:?}: {}", dir, e);
                }
            }
        }
    }
}
