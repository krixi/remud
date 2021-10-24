use crate::{
    color::{Color256, ColorTrue, COLOR_NAME_MAP, COLOR_TAG_MATCHER},
    engine::{ClientMessage, EngineResponse, Output},
    metrics::{stats_gauge, stats_incr},
    ClientId, CLIENT_ID_COUNTER,
};
use futures::{SinkExt, StreamExt};
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use std::{borrow::Cow, convert::TryFrom, str::FromStr, sync::atomic::Ordering};
use thiserror::Error;
use tokio::sync::mpsc;
use warp::{filters::ws::WebSocket, ws::Message, Filter, Rejection};

static WS_CONNECTION_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

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
    Game { message: String },
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
    Game {
        segments: Vec<WsMessageSegment>,
        #[serde(skip_serializing_if = "is_false")]
        is_prompt: bool,
        #[serde(skip_serializing_if = "is_false")]
        is_sensitive: bool,
    },
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl WsResponse {
    fn to_message(&self) -> Message {
        Message::text(serde_json::to_string(self).unwrap())
    }
}

impl From<Output> for WsResponse {
    fn from(value: Output) -> Self {
        let (is_prompt, message, sensitive) = match value {
            Output::Message(message) => (false, message, false),
            Output::Prompt { format, sensitive } => (true, format, sensitive),
        };

        let segments = colorize_web(message.as_str());
        WsResponse::Game {
            segments,
            is_prompt,
            is_sensitive: sensitive,
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(tag = "t", content = "d")]
pub enum WsMessageSegment {
    #[serde(rename(serialize = "cs"))]
    ColorStart { color: ColorTrue },
    #[serde(rename(serialize = "ce"))]
    ColorEnd,
    #[serde(rename(serialize = "t"))]
    Text { text: String },
}

impl WsMessageSegment {
    pub fn color(color: ColorTrue) -> Self {
        WsMessageSegment::ColorStart { color }
    }

    pub fn end_color() -> Self {
        WsMessageSegment::ColorEnd
    }

    pub fn text(text: Cow<'_, str>) -> Self {
        WsMessageSegment::Text {
            text: text.to_owned().to_string(),
        }
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
    stats_incr("ws.client_connected");
    stats_gauge(
        "ws.num_clients",
        WS_CONNECTION_COUNTER.fetch_add(1, Ordering::SeqCst) + 1,
    );
    let client_id = ClientId(CLIENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
    let (engine_tx, engine_rx) = mpsc::channel(16);

    if client_tx
        .send(ClientMessage::Connect(
            client_id,
            client_tx.clone(),
            engine_tx,
        ))
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
                                WsRequest::Game { message } => {
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
                        EngineResponse::Output(outputs) => {
                            for output in outputs {
                                let response = WsResponse::from(output);
                                if ws_tx.send(response.to_message()).await.is_err() {
                                    break
                                }
                            }
                        },
                    }
                } else {
                    let response = WsResponse::from(Output::Message("\r\nServer shutting down. Thanks for playing. <3\r\n".to_string()));
                    if ws_tx.send(response.to_message()).await.is_err() {
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
    stats_incr("ws.client_disconnected");
    stats_gauge(
        "ws.num_clients",
        WS_CONNECTION_COUNTER.fetch_sub(1, Ordering::SeqCst) - 1,
    );
}

fn colorize_web(message: &str) -> Vec<WsMessageSegment> {
    let mut vec = Vec::new();
    let mut open = 0;
    let mut next_start = 0;

    // move through the string one capture at a time, adding the text in between
    while let Some(captures) = COLOR_TAG_MATCHER.captures(&message[next_start..]) {
        // If the previous capture ended before this capture started - i.e. there's something in between
        let capture_start = captures.get(0).unwrap().start();
        if capture_start > 0 {
            vec.push(WsMessageSegment::text(
                message[next_start..next_start + capture_start].into(),
            ))
        }

        // record capture
        if captures.name("escape").is_some() {
            next_start += captures.get(0).unwrap().end();
            vec.push(WsMessageSegment::text("|".into()))
        } else if let Some(m) = captures.name("byte") {
            next_start += captures.get(0).unwrap().end();
            if let Ok(color) = Color256::from_str(m.as_str()) {
                let color = ColorTrue::from(color);
                open += 1;
                vec.push(WsMessageSegment::color(color));
            } else {
                tracing::warn!("failed to capture matched 256 color: {}", m.as_str());
            }
        } else if let Some(m) = captures.name("true") {
            next_start += captures.get(0).unwrap().end();
            if let Ok(color) = ColorTrue::from_str(m.as_str()) {
                open += 1;
                vec.push(WsMessageSegment::color(color));
            } else {
                tracing::warn!("failed to capture matched true color: {}", m.as_str());
            }
        } else if let Some(m) = captures.name("name") {
            next_start += captures.get(0).unwrap().end();
            if let Some(index) = COLOR_NAME_MAP.get(m.as_str().to_lowercase().as_str()) {
                let color = ColorTrue::from(Color256::new(*index));
                open += 1;
                vec.push(WsMessageSegment::color(color));
            } else {
                tracing::warn!("failed to match color name: {}", m.as_str());
            }
        } else if captures.name("clear").is_some() {
            next_start += captures.get(0).unwrap().end();
            if open > 0 {
                open -= 1;
                vec.push(WsMessageSegment::end_color());
            }
        } else {
            let capture = captures
                .iter()
                .flat_map(|m| m.map(|m| format!("'{}'", m.as_str())))
                .join(", ");
            tracing::warn!("unknown color tag(s) captured: {}", capture);
        }

        if next_start >= message.len() {
            break;
        }
    }

    // check for end-of-message text
    if next_start < message.len() {
        vec.push(WsMessageSegment::text(
            message[next_start..message.len()].into(),
        ))
    }

    // close all remaining open tags
    while open > 0 {
        vec.push(WsMessageSegment::end_color());
        open -= 1;
    }

    vec
}

#[cfg(test)]
mod tests {
    use crate::{
        color::ColorTrue,
        web::ws::{colorize_web, WsMessageSegment},
    };

    #[test]
    fn colorize_web_just_text() {
        let result = colorize_web("some text");
        assert_eq!(
            &[WsMessageSegment::text("some text".into())],
            result.as_slice()
        );
    }

    #[test]
    fn colorize_web_wrapped() {
        tracing::info!("wrapped");
        let result = colorize_web("|white|some text|-|");
        tracing::info!("wrapped check");
        assert_eq!(
            &[
                WsMessageSegment::color(ColorTrue::new(255, 255, 255)),
                WsMessageSegment::text("some text".into()),
                WsMessageSegment::end_color()
            ],
            result.as_slice()
        );
    }

    #[test]
    fn colorize_web_auto_close() {
        let result = colorize_web("|white|some text");
        assert_eq!(
            &[
                WsMessageSegment::color(ColorTrue::new(255, 255, 255)),
                WsMessageSegment::text("some text".into()),
                WsMessageSegment::end_color()
            ],
            result.as_slice()
        );
    }

    #[test]
    fn colorize_web_auto_close_twice() {
        tracing::info!("auto close twice");
        let result = colorize_web("|white|some |black|text");
        tracing::info!("auto close twice check");
        assert_eq!(
            &[
                WsMessageSegment::color(ColorTrue::new(255, 255, 255)),
                WsMessageSegment::text("some ".into()),
                WsMessageSegment::color(ColorTrue::new(0, 0, 0)),
                WsMessageSegment::text("text".into()),
                WsMessageSegment::end_color(),
                WsMessageSegment::end_color()
            ],
            result.as_slice()
        );
    }

    #[test]
    fn colorize_web_pipe_escape() {
        let result = colorize_web("|white||||-|");
        assert_eq!(
            &[
                WsMessageSegment::color(ColorTrue::new(255, 255, 255)),
                WsMessageSegment::text("|".into()),
                WsMessageSegment::end_color(),
            ],
            result.as_slice()
        );
        let result = colorize_web("|white|||");
        assert_eq!(
            &[
                WsMessageSegment::color(ColorTrue::new(255, 255, 255)),
                WsMessageSegment::text("|".into()),
                WsMessageSegment::end_color(),
            ],
            result.as_slice()
        );
        let result = colorize_web("||");
        assert_eq!(&[WsMessageSegment::text("|".into()),], result.as_slice());
    }

    #[test]
    fn colorize_web_preserve_whitespace() {
        let result = colorize_web("|white|ID 10|-|\ta flower pot");
        assert_eq!(
            &[
                WsMessageSegment::color(ColorTrue::new(255, 255, 255)),
                WsMessageSegment::text("ID 10".into()),
                WsMessageSegment::end_color(),
                WsMessageSegment::text("\ta flower pot".into()),
            ],
            result.as_slice()
        );
    }
}
