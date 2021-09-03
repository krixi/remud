use bevy_ecs::{schedule::Schedule, world::World};
use bytes::Bytes;
use std::collections::HashMap;
use tokio::{
    sync::mpsc,
    time::{interval, Duration, Interval},
};

#[derive(Debug)]
pub enum ClientMessage {
    Connect(usize, mpsc::UnboundedSender<EngineMessage>),
    Disconnect(usize),
    Ready(usize),
    Input(usize, Bytes),
}

#[derive(Debug)]
pub enum EngineMessage {
    Output(Bytes),
}

pub struct Engine {
    rx: mpsc::UnboundedReceiver<ClientMessage>,
    client_txs: HashMap<usize, mpsc::UnboundedSender<EngineMessage>>,
    ticker: Interval,
    world: World,
    schedule: Schedule,
}

impl Engine {
    pub fn new(rx: mpsc::UnboundedReceiver<ClientMessage>) -> Self {
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
                        self.process(message);
                    }
                }
            }
        }
    }

    fn process(&mut self, message: ClientMessage) {
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
                    match tx.send(EngineMessage::Output(Bytes::from(
                        "Welcome to the world.\r\n",
                    ))) {
                        Err(_) => {
                            self.client_txs.remove(&client);
                        }
                        _ => (),
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
