use crate::{
    engine::{ClientMessage, EngineResponse},
    ClientId, CLIENT_ID_COUNTER,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, sync::atomic::Ordering};
use thiserror::Error;
use tokio::sync::mpsc;
use warp::{filters::ws::WebSocket, ws::Message, Filter, Rejection};

pub(crate) fn websocket_filters(
    engine_tx: mpsc::Sender<ClientMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::path("ws")
        .and(warp::ws())
        .and(with_engine_tx(engine_tx))
        .map(|web_socket: warp::ws::Ws, engine_tx| {
            web_socket.on_upgrade(move |socket| websocket_connect(socket, engine_tx))
        })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "data")]
enum WsRequest {
    Input { message: String },
}

impl TryFrom<Message> for WsRequest {
    type Error = WsRequestParseError;

    fn try_from(value: Message) -> Result<Self, Self::Error> {
        if value.is_text() {
            match serde_json::from_str(value.to_str().unwrap()) {
                Ok(request) => Ok(request),
                Err(e) => {
                    tracing::warn!("failed to deserialize websocket message: {}", e);
                    Err(WsRequestParseError::InvalidRequest(value))
                }
            }
        } else if value.is_binary() {
            tracing::warn!("received binary message from ws: {:?}", value.as_bytes());
            Err(WsRequestParseError::InvalidRequest(value))
        } else if value.is_close() {
            Err(WsRequestParseError::Closed)
        } else {
            Err(WsRequestParseError::InvalidRequest(value))
        }
    }
}

#[derive(Debug, Error)]
enum WsRequestParseError {
    #[error("received close request")]
    Closed,
    #[error("received invalid request")]
    InvalidRequest(Message),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "data")]
enum WsResponse {
    Output { message: String },
}

impl WsResponse {
    fn output(message: String) -> Self {
        WsResponse::Output { message }
    }

    fn to_message(&self) -> Message {
        Message::text(serde_json::to_string(self).unwrap())
    }
}

fn with_engine_tx(
    engine_tx: mpsc::Sender<ClientMessage>,
) -> impl Filter<Extract = (mpsc::Sender<ClientMessage>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || engine_tx.clone())
}

#[tracing::instrument(name = "websocket connect", skip_all)]
async fn websocket_connect(websocket: WebSocket, client_tx: mpsc::Sender<ClientMessage>) {
    let client_id = ClientId(CLIENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
    let (engine_tx, engine_rx) = mpsc::channel(16);

    if client_tx
        .send(ClientMessage::Connect(client_id, engine_tx))
        .await
        .is_err()
    {
        return;
    }

    if client_tx
        .send(ClientMessage::Ready(client_id))
        .await
        .is_err()
    {
        return;
    }

    process(client_id, websocket, client_tx, engine_rx).await
}

#[tracing::instrument(name = "process websocket", skip(websocket, client_tx, engine_rx))]
async fn process(
    client_id: ClientId,
    websocket: WebSocket,
    client_tx: mpsc::Sender<ClientMessage>,
    mut engine_rx: mpsc::Receiver<EngineResponse>,
) {
    let (mut ws_tx, mut ws_rx) = websocket.split();

    loop {
        tokio::select! {
            maybe_message = ws_rx.next() => {
                if let Some(Ok(message)) = maybe_message {
                    match WsRequest::try_from(message) {
                        Ok(request) => {
                            match request {
                                WsRequest::Input { message } => {
                                    if client_tx.send(ClientMessage::Input(client_id, message)).await.is_err() {
                                        break
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            match e {
                                WsRequestParseError::Closed => break,
                                WsRequestParseError::InvalidRequest(e) => {
                                    tracing::warn!("failed to deserialize websocket request: {:?}", e);
                                },
                            }
                        },
                    }
                } else {
                    break
                }
            }
            maybe_message = engine_rx.recv() => {
                if let Some(message) = maybe_message {
                    match message {
                        EngineResponse::Output(message) => {
                            let message = WsResponse::output(message);
                            if ws_tx.send(message.to_message()).await.is_err() {
                                break
                            }
                        },
                        EngineResponse::EndOutput => (),
                    }
                } else {
                    let message = WsResponse::output("\r\nServer shutting down. Thanks for playing. <3\r\n".to_string());
                    if ws_tx.send(Message::text(serde_json::to_string(&message).unwrap())).await.is_err() {
                        break
                    }
                    break
                }
            }
        }
    }

    client_tx
        .send(ClientMessage::Disconnect(client_id))
        .await
        .ok();
}
