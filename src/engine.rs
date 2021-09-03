use bevy_ecs::{schedule::Schedule, world::World};
use bytes::Bytes;
use std::collections::HashMap;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

#[derive(Debug)]
pub enum ClientMessage {
    Connect(usize, mpsc::Sender<EngineMessage>),
    Disconnect(usize),
    Ready(usize),
    Input(usize, Bytes),
}

#[derive(Debug)]
pub enum EngineMessage {
    Output(Bytes),
}

pub struct Engine {
    rx: mpsc::Receiver<ClientMessage>,
    client_txs: HashMap<usize, mpsc::Sender<EngineMessage>>,
    ticker: Interval,
    world: World,
    schedule: Schedule,
}

impl Engine {
    pub fn new(rx: mpsc::Receiver<ClientMessage>) -> Self {
        Engine {
            rx,
            client_txs: HashMap::new(),
            ticker: interval(Duration::from_millis(15)),
            world: World::new(),
            schedule: Schedule::default(),
        }
    }

    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.ticker.tick() => {
                    self.schedule.run_once(&mut self.world);
                }
                maybe_message = self.rx.recv() => {
                    if let Some(message) = maybe_message {
                        self.process(message).await;
                    }
                }
            }
        }
    }

    async fn process(&mut self, message: ClientMessage) {
        match message {
            ClientMessage::Connect(client, client_tx) => {
                self.client_txs.insert(client, client_tx);
                tracing::info!("Client {} connected", client);
            }
            ClientMessage::Disconnect(client) => {
                self.client_txs.remove(&client);
                tracing::info!("Client {} disconnected", client);
            }
            ClientMessage::Ready(client) => {
                if let Some(tx) = self.client_txs.get(&client) {
                    let message = EngineMessage::Output(Bytes::from("Welcome to the world.\r\n"));
                    if tx.send(message).await.is_err() {
                        self.client_txs.remove(&client);
                    }
                }
                tracing::info!("Client {} ready", client)
            }
            ClientMessage::Input(client, input) => {
                tracing::info!("Client {} sent {:?}", client, input)
            }
        }
    }
}
