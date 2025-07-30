use crate::stages::{event::Dispatcher, intercepted::InterceptedResponse, stage::StageId};
use tokio::sync::mpsc::Receiver;

pub async fn start_consumer(mut rx: Receiver<InterceptedResponse>, dispatcher: Dispatcher) {
    tokio::spawn(async move {
        while let Some(resp) = rx.recv().await {
            dispatcher.emit(StageId::Filter, resp);
        }
    });
}
