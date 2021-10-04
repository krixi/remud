use std::{
    io::{self},
    path::Path,
    sync::{
        atomic::{AtomicU16, Ordering},
        mpsc,
    },
    time::Duration,
};

use once_cell::sync::Lazy;
use remud_lib::{run_remud, RemudError, WebOptions};
use telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};
use tokio::time::timeout;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter, FmtSubscriber};

static PORT_COUNTER: Lazy<AtomicU16> = Lazy::new(|| AtomicU16::new(49152));
pub static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "remud_test=info,remud_lib=debug".to_string();

    init_subscriber(default_filter_level, TestWriter::default());
});

pub struct Server {
    telnet: u16,
    web: u16,
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,
    ready: Option<tokio::sync::mpsc::Receiver<()>>,
}

impl Server {
    pub fn telnet(&self) -> u16 {
        self.telnet
    }

    pub fn web(&self) -> u16 {
        self.web
    }

    pub fn ready(&mut self) {
        let mut ready = self.ready.take().unwrap();

        let (result, ready) = self.runtime.block_on(async move {
            let result = timeout(Duration::from_secs(5), ready.recv()).await;
            (result, ready)
        });

        self.ready = Some(ready);

        match result {
            Ok(_) => (),
            Err(e) => {
                panic!("server failed to restart: {}", e);
            }
        }
    }

    pub fn new_connect<S1: AsRef<str>, S2: AsRef<str>>(
        player: S1,
        password: S2,
    ) -> (Server, TelnetPlayer) {
        let mut server = Server::default();
        let telnet = server.create_user(player, password);
        (server, telnet)
    }

    pub fn restart(&mut self, mut client: TelnetPlayer) -> TelnetPlayer {
        client.send("restart");
        let (player, password) = (client.player.clone(), client.password.clone());
        drop(client);

        self.ready();
        self.login_user(player, password)
    }

    pub fn create_user<S1: AsRef<str>, S2: AsRef<str>>(
        &mut self,
        player: S1,
        password: S2,
    ) -> TelnetPlayer {
        let mut connection = TelnetConnection::new(self.telnet());

        connection.info("create user");
        connection.recv_contains("Connected to");
        connection.recv_contains("Name?");
        connection.recv_prompt();
        connection.send(player.as_ref());
        connection.recv_contains("New user detected.");
        connection.recv_contains("Password?");
        connection.recv_prompt();
        connection.send(password.as_ref());
        connection.recv_contains("Password accepted.");
        connection.recv_contains("Verify?");
        connection.recv_prompt();
        connection.send(password.as_ref());
        connection.recv_contains("Password verified.");
        connection.recv(); // blank line 1
        connection.recv_contains("Welcome to City Six.");
        connection.recv(); // blank line 2
        connection.recv(); // ignore the look that happens when we log in
        connection.recv_prompt();

        TelnetPlayer {
            player: player.as_ref().to_string(),
            password: password.as_ref().to_string(),
            connection,
        }
    }

    pub fn login_user<S1: AsRef<str>, S2: AsRef<str>>(
        &mut self,
        player: S1,
        password: S2,
    ) -> TelnetPlayer {
        let mut connection = TelnetConnection::new(self.telnet());

        connection.info("login user");
        connection.recv_contains("Connected to");
        connection.recv_contains("Name?");
        connection.recv_prompt();
        connection.send(player.as_ref());
        connection.recv_contains("User located.");
        connection.recv_contains("Password?");
        connection.recv_prompt();
        connection.send(password.as_ref());
        connection.recv_contains("Password verified.");
        connection.recv(); // blank line 1
        connection.recv_contains("Welcome to City Six.");
        connection.recv(); // blank line 2
        connection.recv(); // ignore the look that happens when we log in
        connection.recv_prompt();

        TelnetPlayer {
            player: player.as_ref().to_string(),
            password: password.as_ref().to_string(),
            connection,
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Lazy::force(&TRACING);

        let (tx, rx) = mpsc::channel();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .unwrap();

        runtime.spawn(async move {
            let external_tx = tx;
            let mut telnet_port ;
            let mut web_port ;

            let ready_rx = 'connect_loop: loop {
                telnet_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
                web_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);

                let web = WebOptions::new(web_port, Path::new("./keys"), vec!["http://localhost"], None);
                let (ready_tx, mut ready_rx) = tokio::sync::mpsc::channel(16);

                let spawn = tokio::spawn(async move {
                    run_remud(None, telnet_port, web, Some(ready_tx)).await
                });

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

            external_tx.send((telnet_port, web_port, ready_rx)).unwrap_or_else(|e| panic!("failed to start server: {}", e));
        });

        let (telnet, web, ready_rx) = rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|e| panic!("failed to receive server init message: {}", e));

        Server {
            telnet,
            web,
            runtime,
            ready: Some(ready_rx),
        }
    }
}

pub struct TelnetConnection {
    connection: Telnet,
}

impl TelnetConnection {
    // Creates a new client and performs initial TELNET options negotiation.
    fn new(port: u16) -> Self {
        // set up connection
        let mut connection =
            Telnet::connect(("127.0.0.1", port), 1024).expect("failed to connect to ReMUD");

        if let TelnetEvent::Negotiation(NegotiationAction::Do, TelnetOption::TTYPE) = connection
            .read_timeout(Duration::from_secs(5))
            .expect("did not receive DO TTYPE")
        {
            connection.negotiate(NegotiationAction::Wont, TelnetOption::TTYPE);
        } else {
            panic!("received unexpected message waiting for DO TTYPE");
        }

        TelnetConnection { connection }
    }

    pub fn info<S: AsRef<str>>(&mut self, text: S) {
        if !text.as_ref().is_empty() {
            tracing::info!("---------- {} ----------", text.as_ref());
        }
    }

    pub fn send<S: AsRef<str>>(&mut self, line: S) {
        self.connection
            .write(format!("{}\r\n", line.as_ref()).as_bytes())
            .unwrap_or_else(|_| panic!("failed to send '{}'", line.as_ref()));
    }

    pub fn recv(&mut self) -> String {
        let event = self
            .connection
            .read_timeout(Duration::from_secs(5))
            .unwrap_or_else(|_| panic!("failed to read from telnet connection",));

        if let TelnetEvent::Data(data) = event {
            String::from_utf8(data.to_vec()).expect("server sent invalid UTF-8 string")
        } else {
            panic!(
                "did not receive expected DATA event, got this instead: {:?}",
                event
            );
        }
    }

    pub fn recv_contains<S: AsRef<str>>(&mut self, text: S) {
        let message = self.recv();
        assert!(
            message.contains(text.as_ref()),
            "did not find '{}' in message {:?}",
            text.as_ref(),
            message,
        )
    }

    pub fn recv_contains_all<S: AsRef<str>>(&mut self, msgs: Vec<S>) {
        let message = self.recv();
        for text in msgs.iter() {
            assert!(
                message.contains(text.as_ref()),
                "did not find '{}' in message {:?}",
                text.as_ref(),
                message,
            )
        }
    }
    pub fn recv_contains_none<S: AsRef<str>>(&mut self, msgs: Vec<S>) {
        let message = self.recv();
        for text in msgs.iter() {
            assert!(
                !message.contains(text.as_ref()),
                "found unwanted '{}' in message {:?}",
                text.as_ref(),
                message,
            )
        }
    }

    pub fn recv_prompt(&mut self) {
        self.recv_contains(">");
    }
}

pub struct TelnetPlayer {
    player: String,
    password: String,
    connection: TelnetConnection,
}

impl std::ops::Deref for TelnetPlayer {
    type Target = TelnetConnection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl std::ops::DerefMut for TelnetPlayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

impl TelnetPlayer {
    pub fn test<S1: AsRef<str>, S2: AsRef<str>, S3: AsRef<str>>(
        &mut self,
        scenario: S1,
        command: S2,
        response_contains: Vec<S3>,
    ) {
        self.validate(scenario, command, Validate::Includes(response_contains));
    }

    pub fn test_many<S1: AsRef<str>, S2: AsRef<str>, S3: AsRef<str> + Clone>(
        &mut self,
        scenario: S1,
        command: S2,
        response_contains: Vec<Vec<S3>>,
    ) {
        self.info(scenario);
        self.send(command);
        for expected in response_contains.iter() {
            self.recv_contains_all(expected.clone());
        }
        self.recv_prompt();
    }

    pub fn test_exclude<S1: AsRef<str>, S2: AsRef<str>, S3: AsRef<str>>(
        &mut self,
        scenario: S1,
        command: S2,
        response_excludes: Vec<S3>,
    ) {
        self.validate(scenario, command, Validate::Excludes(response_excludes));
    }

    fn validate<S1: AsRef<str>, S2: AsRef<str>, S3: AsRef<str>>(
        &mut self,
        scenario: S1,
        command: S2,
        validate: Validate<S3>,
    ) {
        self.info(scenario);
        self.send(command);

        let (is_include, items) = match validate {
            Validate::Includes(items) => (true, items),
            Validate::Excludes(items) => (false, items),
        };

        if is_include {
            self.recv_contains_all(items);
        } else {
            self.recv_contains_none(items);
        }
        self.recv_prompt();
    }
}

enum Validate<S: AsRef<str>> {
    Includes(Vec<S>),
    Excludes(Vec<S>),
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
