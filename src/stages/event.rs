use tokio::sync::mpsc::UnboundedSender;

use crate::stages::{intercepted::InterceptedResponse, stage::StageId};


pub struct Event {
    pub stage: StageId,
    pub resp: InterceptedResponse,
}

#[derive(Clone)]
pub struct Dispatcher {
    pub tx: UnboundedSender<Event>,
}

impl Dispatcher {
    pub fn emit(&self, stage: StageId, resp: InterceptedResponse) {
        let _ = self.tx.send(Event { stage, resp });
    }
}

