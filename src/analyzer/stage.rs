use async_trait::async_trait;

use crate::analyzer::{event::Dispatcher, intercepted::InterceptedResponse};

#[async_trait]
pub trait Stage: Send + Sync {
    async fn process(&self, dispatcher: Dispatcher, resp: InterceptedResponse);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageId {
    Filter,
    Map,
    SaveFile,
    Scan,
    JsScan
}
