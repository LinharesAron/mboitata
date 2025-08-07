use crate::analyzer::{event::Dispatcher, intercepted::InterceptedResponse, stage::StageId};
use tokio::{sync::mpsc::Receiver, task::JoinHandle};

pub async fn start_consumer(mut rx: Receiver<InterceptedResponse>, dispatcher: Dispatcher) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(resp) = rx.recv().await {
            dispatcher.emit(StageId::Filter, resp);
        }

        println!("✅ Fila fechada. Consumer parando.");
        dispatcher.close_gracefully().await;
    })
}
