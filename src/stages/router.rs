use std::{collections::HashMap, sync::Arc};

use tokio::{sync::{broadcast, mpsc::unbounded_channel}, task::JoinHandle};

use crate::stages::{
    event::Dispatcher,
    stage::{Stage, StageId},
};

pub struct StageRegistry {
    stages: HashMap<StageId, Box<dyn Stage + Send + Sync>>,
}

impl StageRegistry {
    pub fn new() -> Self {
        Self {
            stages: HashMap::new(),
        }
    }

    pub fn register(mut self, id: StageId, stage: Box<dyn Stage + Send + Sync>) -> Self {
        self.stages.insert(id, stage);
        self
    }

    pub fn build(self) -> (Dispatcher, JoinHandle<()>) {
        start_stage_router(self.stages)
    }
}

impl Default for StageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn start_stage_router(
    stages: HashMap<StageId, Box<dyn Stage + Send + Sync>>,
) -> (Dispatcher, JoinHandle<()>) {
    let (tx, mut rx) = unbounded_channel();
    let (closed, mut closed_rx) = broadcast::channel(1);
    let dispatcher = Dispatcher::new(tx, closed);

    let stages = Arc::new(stages);
    let tk_stages = stages.clone();
    let tk_dispatcher = dispatcher.clone();
    
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    if let Some(stage) = tk_stages.get(&event.stage) {
                        stage.process(tk_dispatcher.clone(), event.resp).await;
                    }
                    tk_dispatcher.complete();
                }

                _ = closed_rx.recv() => {
                    break;
                }
            }
        }
    });

    (dispatcher, handle)
}
