mod telnet;
mod web;

use std::{
    borrow::Cow,
    io::{self},
    path::Path,
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

use once_cell::sync::Lazy;
use remud_lib::{run_remud, RemudError, WebOptions};
use tokio::time::timeout;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter, FmtSubscriber};

pub use crate::telnet::{Match, Matcher, TelnetConnection, TelnetPlayer};
pub use crate::web::{
    AuthenticatedWebClient, JsonScript, JsonScriptName, JsonScriptResponse, Trigger, WebClient,
};
pub use reqwest::StatusCode;

static PORT_COUNTER: Lazy<AtomicU16> = Lazy::new(|| AtomicU16::new(49152));
pub static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "remud_test=info,remud_lib=debug".to_string();

    init_subscriber(default_filter_level, TestWriter::default());
});

pub struct Server {
    telnet: u16,
    web: u16,
    #[allow(dead_code)]
    ready: tokio::sync::mpsc::Receiver<()>,
}

impl Server {
    pub async fn new() -> Self {
        Lazy::force(&TRACING);

        let mut telnet_port;
        let mut web_port;

        let ready_rx = 'connect_loop: loop {
            telnet_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
            web_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);

            let web = WebOptions::new(
                web_port,
                Path::new("./keys"),
                vec!["http://localhost"],
                None,
            );
            let (ready_tx, mut ready_rx) = tokio::sync::mpsc::channel(16);

            let spawn =
                tokio::spawn(
                    async move { run_remud(None, telnet_port, web, Some(ready_tx)).await },
                );

            tokio::select! {
                join_result = spawn => {
                    match join_result {
                        Ok(remud_result) => {
                            match remud_result {
                                // ReMUD did not stop to listen for requests and the run function returned early.
                                Ok(_) => panic!("ReMUD exited early"),
                                Err(e) => match e {
                                    RemudError::TelnetError(_) => {
                                        tracing::info!("port {} or {} in use, selecting next ports", telnet_port, web_port);
                                    },
                                    e => panic!("ReMUD failed to start: {}", e)
                                }
                            }
                        }
                        Err(_) => {
                            panic!("Failed to join ReMUD task")
                        }
                    }
                }
                _ = ready_rx.recv() => {
                    break 'connect_loop ready_rx
                }
            }
        };

        Server {
            telnet: telnet_port,
            web: web_port,
            ready: ready_rx,
        }
    }

    pub async fn new_create_player<'a, S1, S2>(player: S1, password: S2) -> (Server, TelnetPlayer)
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        let mut server = Server::new().await;
        let telnet = server.create_player(player, password).await;
        (server, telnet)
    }

    pub async fn new_connect_telnet() -> (Server, TelnetConnection) {
        let server = Server::new().await;
        let connection = TelnetConnection::new(server.telnet());
        (server, connection)
    }

    pub async fn restart(&mut self, mut client: TelnetPlayer) -> TelnetPlayer {
        client.send("restart").await;
        let (player, password) = (client.name.clone(), client.password.clone());
        drop(client);

        self.ready().await;
        self.login_player(player, password).await
    }

    pub async fn ready(&mut self) {
        let result = timeout(Duration::from_secs(5), self.ready.recv()).await;

        match result {
            Ok(_) => (),
            Err(e) => {
                panic!("server failed to restart: {}", e);
            }
        }
    }

    pub fn telnet(&self) -> u16 {
        self.telnet
    }

    pub fn web(&self) -> u16 {
        self.web
    }

    pub fn connect_telnet(&self) -> TelnetConnection {
        TelnetConnection::new(self.telnet())
    }

    pub fn connect_web(&self) -> WebClient {
        WebClient::new(self.web())
    }

    pub async fn login_web(&self, player: &TelnetPlayer) -> AuthenticatedWebClient {
        let client = self.connect_web();
        client
            .login(player.name.as_str(), player.password.as_str())
            .await
            .expect("valid credentials from telnet player")
    }

    pub async fn create_player<'a, S1, S2>(&mut self, player: S1, password: S2) -> TelnetPlayer
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        let player = player.into();
        let password = password.into();
        let mut connection = self.connect_telnet();

        connection.info("create player");
        connection.line_contains("Connected to").await;
        connection.line_contains("Name?").await;
        connection.assert_prompt().await;

        connection
            .test(
                "enter player name",
                player.as_ref(),
                vec!["New user detected.", "Password?"],
            )
            .await;

        connection
            .test(
                "enter password",
                password.as_ref(),
                vec!["Password accepted.", "Verify?"],
            )
            .await;

        connection
            .test(
                "verify password",
                password.as_ref(),
                vec!["Password verified.", "Welcome to City Six."],
            )
            .await;

        TelnetPlayer::new(connection, player, password)
    }

    pub async fn login_player<'a, S1, S2>(&mut self, player: S1, password: S2) -> TelnetPlayer
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        let player = player.into();
        let password = password.into();
        let mut connection = self.connect_telnet();

        connection.info("login player");
        connection.line_contains("Connected to").await;
        connection.line_contains("Name?").await;
        connection.assert_prompt().await;

        connection
            .test(
                "enter player name",
                player.as_ref(),
                vec!["User located.", "Password?"],
            )
            .await;

        connection.assert_prompt().await;

        connection
            .test(
                "enter password",
                password.as_ref(),
                vec!["Password verified.", "Welcome to City Six."],
            )
            .await;

        connection.assert_prompt().await;

        TelnetPlayer::new(connection, player, password)
    }
}

#[derive(Default)]
pub struct TestWriter {}

impl io::Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        print!("{}", String::from_utf8(buf.to_vec()).unwrap());

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl MakeWriter for TestWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        TestWriter::default()
    }
}

pub fn init_subscriber(env_filter: String, sink: impl MakeWriter + Send + Sync + 'static) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(sink)
        .with_level(true)
        .init();
}
