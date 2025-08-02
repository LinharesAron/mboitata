use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use tokio::sync::{broadcast, mpsc::UnboundedSender, Notify};
use crate::stages::{intercepted::InterceptedResponse, stage::StageId};


pub struct Event {
    pub stage: StageId,
    pub resp: InterceptedResponse,
}

#[derive(Clone)]
pub struct Dispatcher {
    tx: UnboundedSender<Event>,
    closed: broadcast::Sender<()>,
    inflight: Arc<AtomicUsize>,
    notify_done: Arc<Notify>,
}

impl Dispatcher {
    pub fn new(tx: UnboundedSender<Event>, closed: broadcast::Sender<()>) -> Self {
        Self {
            tx,
            closed,
            inflight: Arc::new(AtomicUsize::new(0)),
            notify_done: Arc::new(Notify::new()),
        }
    }

    pub fn emit(&self, stage: StageId, resp: InterceptedResponse) {
        self.inflight.fetch_add(1, Ordering::SeqCst);

        let event = Event { stage, resp };
        let _ = self.tx.send(event);
    }

    pub fn complete(&self) {
        if self.inflight.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.notify_done.notify_waiters();
        }
    }

    pub async fn close_gracefully(&self) {
        while self.inflight.load(Ordering::SeqCst) > 0 {
            self.notify_done.notified().await;
        }

        let _ = self.closed.send(());
        println!("✅ Dispatcher finalizado com todos os eventos processados.");
    }
}

