mod engine;
mod telnet;

use bytes::{Buf, BytesMut};
use engine::{ClientMessage, Engine};
use futures::{SinkExt, StreamExt};
use std::collections::VecDeque;
use telnet::{Codec, Frame, Telnet};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (tx, rx) = mpsc::unbounded_channel();

    let mut engine = Engine::new(rx);

    tokio::spawn(async move { engine.run().await });

    let listener = TcpListener::bind("127.0.0.1:2004")
        .await
        .expect("can listen");

    tracing::info!("Listening on 127.0.0.1:2004");

    loop {
        let (socket, _) = listener.accept().await.expect("can accept");

        let tx = tx.clone();

        tokio::spawn(async move {
            process(socket, tx).await;
        });
    }
}

async fn process(socket: TcpStream, tx: mpsc::UnboundedSender<ClientMessage>) {
    let mut framed = Framed::new(socket, Codec);
    let mut telnet = Telnet::new();
    let mut ready = false;

    // Negotiate telnet options
    for frame in telnet.initiate() {
        framed.send(frame).await.unwrap();
    }

    let mut input = VecDeque::new();
    let mut input_buffer = BytesMut::new();

    while let Some(frame) = framed.next().await {
        match frame {
            Ok(frame) => match frame {
                Frame::Command(_command) => (),
                Frame::Negotiate(command, option) => {
                    for frame in telnet.negotiate(command, option) {
                        framed.send(frame).await.unwrap()
                    }
                }
                Frame::Subnegotiate(option, data) => {
                    for frame in telnet.subnegotiate(option, data) {
                        framed.send(frame).await.unwrap()
                    }
                }
                Frame::Data(mut data) => {
                    while let Some(end) = data
                        .as_ref()
                        .windows(2)
                        .position(|b| b[0] == b'\r' && b[1] == b'\n')
                    {
                        input_buffer.extend(data.split_to(end));
                        data.advance(2);

                        input.push_back(input_buffer.freeze());

                        input_buffer = BytesMut::new();
                    }

                    input_buffer.extend(data);
                }
            },
            Err(e) => {
                tracing::error!("err: {:?}", e);
                break;
            }
        }

        if ready {
            // send input and do things
            for input in input.drain(..) {
                tx.send(ClientMessage::Input(input)).unwrap();
            }
        } else {
            if !telnet.configured() {
                for frame in telnet.configure() {
                    framed.send(frame).await.unwrap();
                }
            } else {
                ready = true;
                tx.send(ClientMessage::Ready).unwrap();
            }
        }
    }

    // Enter game
    tracing::info!("closing session");
}
