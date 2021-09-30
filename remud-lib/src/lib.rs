// #![warn(clippy::pedantic)]
#![allow(clippy::too_many_arguments)]

mod color;
mod ecs;
mod engine;
mod macros;
mod telnet;
mod text;
mod web;
mod world;

use std::{collections::HashMap, fmt, sync::atomic::AtomicUsize};

use futures::future::join_all;
use once_cell::sync::Lazy;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::{
    engine::{db::Db, Engine, EngineMessage},
    web::run_web_server,
};

pub use web::{TlsOptions, WebOptions};

static CLIENT_ID_COUNTER: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(1));

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ClientId(usize);

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client {}", self.0)
    }
}

#[derive(Debug, Error)]
pub enum RemudError {
    #[error("engine failed to execute: {0}")]
    EngineError(#[from] engine::Error),
    #[error("failed to initialize telnet server: {0}")]
    TelnetError(#[from] telnet::Error),
    #[error("failed to initialize web server: {0}")]
    WebError(#[from] web::Error),
    #[error("failed to initialize database: {0}")]
    DbError(#[from] engine::db::Error),
}

pub async fn run_remud<'a>(
    db_path: Option<&str>,
    telnet_port: u16,
    web: WebOptions<'a>,
    ready_tx: Option<oneshot::Sender<()>>,
) -> Result<(), RemudError> {
    let (client_tx, client_rx) = mpsc::channel(256);
    let (engine_tx, mut engine_rx) = mpsc::channel(16);
    let (web_tx, web_rx) = mpsc::channel(16);

    let db = Db::new(db_path).await.map_err(engine::Error::from)?;

    let mut engine = Engine::new(db.clone(), client_rx, engine_tx, web_rx).await?;
    let _engine_handle = tokio::spawn(async move {
        engine.run().await;
    });

    let telnet_address = format!("0.0.0.0:{}", telnet_port);
    let telnet = telnet::Server::new(telnet_address.as_str()).await?;

    let _web_handle = run_web_server(web, db, web_tx, client_tx.clone()).await?;

    if let Some(tx) = ready_tx {
        tx.send(()).ok();
    }

    let mut join_handles = HashMap::new();

    'main: loop {
        tokio::select! {
            handle = telnet.accept(client_tx.clone()) => {
                match handle {
                    Some((client_id, handle)) => {
                        join_handles.insert(client_id, handle);
                    },
                    None => break 'main
                }
            }
            message = engine_rx.recv() => {
                match message {
                    Some(message) => {
                        match message {
                            EngineMessage::Shutdown => {
                                tracing::warn!("Engine shutdown, halting server.");
                                break 'main
                            },
                            EngineMessage::Disconnect(client_id) => {
                                join_handles.remove(&client_id);
                            },
                        }
                    },
                    None => {
                        tracing::error!("Engine control closed, halting server");
                        break 'main
                    },
                }
            }
        }
    }

    join_all(join_handles.values_mut()).await;

    Ok(())
}
