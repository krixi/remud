use bytes::Bytes;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum ClientMessage {
    Ready,
    Input(Bytes),
}

pub struct Engine {
    rx: mpsc::UnboundedReceiver<ClientMessage>,
}

impl Engine {
    pub fn new(rx: mpsc::UnboundedReceiver<ClientMessage>) -> Self {
        Engine { rx }
    }

    pub async fn run(&mut self) {
        while let Some(message) = self.rx.recv().await {
            tracing::info!("engine: {:?}", message);
        }
    }
}
