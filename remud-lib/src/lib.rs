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
use once_cell::sync::{Lazy, OnceCell};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::{
    engine::{db::Db, Engine, EngineMessage},
    web::run_web_server,
};

use cadence::{Counted, Gauged, StatsdClient, DEFAULT_PORT};
use tokio::net::UdpSocket;
use tokio_cadence::TokioBatchUdpMetricSink;
pub use web::{TlsOptions, WebOptions};

static CLIENT_ID_COUNTER: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(1));
static METRICS: OnceCell<StatsdClient> = OnceCell::new();

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ClientId(usize);

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client {}", self.0)
    }
}

impl ClientId {
    fn id(&self) -> usize {
        self.0
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
    #[error("failed to init metrics socket: {0}")]
    IoError(#[from] std::io::Error),
    #[error("failed to init metrics sink: {0}")]
    MetricError(#[from] cadence::MetricError),
}

pub async fn run_remud(
    db_path: Option<&str>,
    telnet_port: u16,
    web: WebOptions<'_>,
    ready_tx: Option<mpsc::Sender<()>>,
) -> Result<(), RemudError> {
    let db = Db::new(db_path).await.map_err(engine::Error::from)?;

    let host = ("telegraf", DEFAULT_PORT);
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let (sink, process) = TokioBatchUdpMetricSink::from(host, socket)?;
    let metrics_task = tokio::spawn(process);
    let client = StatsdClient::from_sink("remud", sink);
    METRICS.get_or_init(|| client);

    'program: loop {
        let (client_tx, client_rx) = mpsc::channel(256);
        let (engine_tx, mut engine_rx) = mpsc::channel(16);
        let (web_tx, web_rx) = mpsc::channel(16);

        let mut engine = Engine::new(db.clone(), client_rx, engine_tx, web_rx).await?;
        let engine_handle = tokio::spawn(async move {
            engine.run().await;
        });

        let telnet_address = format!("0.0.0.0:{}", telnet_port);
        let telnet = telnet::Server::new(telnet_address.as_str()).await?;

        let web_handle = run_web_server(&web, db.clone(), web_tx, client_tx.clone()).await?;

        if let Some(tx) = ready_tx.clone() {
            tracing::info!("server ready");
            tx.send(()).await.ok();
        }

        let mut join_handles = HashMap::new();

        'main: loop {
            tokio::select! {
                handle = telnet.accept(client_tx.clone()) => {
                    match handle {
                        Some((client_id, handle)) => {
                            METRICS.get().unwrap().incr("telnet.client_connected").ok();
                            join_handles.insert(client_id, handle);
                            METRICS.get().unwrap().gauge("telnet.num_clients", join_handles.len() as u64).ok();
                        },
                        None => break 'main
                    }
                }
                message = engine_rx.recv() => {
                    match message {
                        Some(message) => {
                            match message {
                                EngineMessage::Disconnect(client_id) => {
                                    METRICS.get().unwrap().incr("telnet.client_disconnected").ok();
                                    join_handles.remove(&client_id);
                                    METRICS.get().unwrap().gauge("telnet.num_clients", join_handles.len() as u64).ok();
                                },
                                EngineMessage::Restart => {
                                    tracing::warn!("engine restart, rebooting server");
                                    break 'main
                                },
                                EngineMessage::Shutdown => {
                                    tracing::warn!("engine shutdown, halting server");
                                    break 'program
                                },
                            }
                        },
                        None => {
                            tracing::error!("engine control closed, halting server");
                            break 'main
                        },
                    }
                }
            }
        }

        // Join all telnet connections - they should be shutting down as the server channel shuts down
        tracing::info!("joining client handles");
        join_all(join_handles.values_mut()).await;

        // Make sure everything is shutdown before restarting
        tracing::info!("aborting web server");
        web_handle.abort();
        match web_handle.await {
            Ok(_) => (),
            Err(e) => {
                if !e.is_cancelled() {
                    tracing::error!("error halting web server: {}", e);
                    break;
                }
            }
        };

        tracing::info!("joining engine");
        match engine_handle.await {
            Ok(_) => (),
            Err(e) => {
                tracing::error!("error halting engine: {}", e);
                break;
            }
        }

        tracing::warn!("servers halted, restarting game server");
    }

    tracing::warn!("awaiting metrics shutdown");
    metrics_task.await.unwrap();

    tracing::warn!("server shutdown complete");

    Ok(())
}
