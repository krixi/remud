use crate::{engine::ClientMessage, ClientId, CLIENT_ID_COUNTER};
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;
use warp::{filters::ws::WebSocket, Filter, Rejection};

pub fn websocket_filters(
    engine_tx: mpsc::Sender<ClientMessage>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::path("ws")
        .and(warp::ws())
        .and(with_engine_tx(engine_tx))
        .map(|web_socket: warp::ws::Ws, engine_tx| {
            web_socket.on_upgrade(move |socket| user_connected(socket, engine_tx))
        })
}

fn with_engine_tx(
    engine_tx: mpsc::Sender<ClientMessage>,
) -> impl Filter<Extract = (mpsc::Sender<ClientMessage>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || engine_tx.clone())
}

async fn user_connected(ws: WebSocket, engine_tx: mpsc::Sender<ClientMessage>) {
    let client_id = ClientId(CLIENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
    // TODO: see example https://github.com/seanmonstar/warp/blob/master/examples/websockets_chat.rs
    todo!()
}
