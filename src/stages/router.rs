use std::{collections::HashMap, sync::Arc};

use tokio::sync::mpsc::unbounded_channel;

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

    pub fn build(self) -> Dispatcher {
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
) -> Dispatcher {
    let (tx, mut rx) = unbounded_channel();
    let dispatcher = Dispatcher { tx: tx.clone() };

    let stages = Arc::new(stages);
    let tk_stages = stages.clone();
    let tk_dispatcher = dispatcher.clone();

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Some(stage) = tk_stages.get(&event.stage) {
                stage.process(tk_dispatcher.clone(), event.resp).await;
            }
        }
    });

    dispatcher
}
