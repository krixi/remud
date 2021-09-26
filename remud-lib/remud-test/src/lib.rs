use std::{
    io::{self},
    sync::{
        atomic::{AtomicU16, Ordering},
        mpsc,
    },
    time::Duration,
};

use once_cell::sync::Lazy;
use remud_lib::{run_remud, RemudError};
use telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};
use tokio::sync::oneshot;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter, FmtSubscriber};

static PORT_COUNTER: Lazy<AtomicU16> = Lazy::new(|| AtomicU16::new(49152));
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "remud_test=info,remud_lib=debug".to_string();

    init_subscriber(default_filter_level, TestWriter::default());
});

pub struct Server {
    telnet: u16,
    web: u16,
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,
}

impl Server {
    pub fn new() -> Self {
        Lazy::force(&TRACING);

        let (tx, rx) = mpsc::channel();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .unwrap();

        runtime.spawn(async move {
            let external_tx = tx;
            let mut telnet_port ;
            let mut web_port ;

            'connect_loop: loop {
                telnet_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
                let telnet_addr = format!("127.0.0.1:{}", telnet_port);
                web_port = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
                let web_addr = format!("127.0.0.1:{}", web_port);

                let (tx, rx) = oneshot::channel();

                let spawn = tokio::spawn(async move {
                    run_remud(&telnet_addr, &web_addr, None, Some(tx)).await
                });

                tokio::select! {
                    join_result = spawn => {
                        match join_result {
                            Ok(remud_result) => {
                                match remud_result {
                                    // ReMUD did not stop to listen for requests and the run function returned early.
                                    Ok(_) => panic!("ReMUD exited early"),
                                    Err(e) => match e {
                                        RemudError::BindError(_) => {
                                            tracing::info!("port {} or {} in use, selecting next ports", telnet_port, web_port);
                                        },
                                        RemudError::EngineError(e) => panic!("ReMUD failed to start: {}", e)
                                    }
                                }
                            }
                            Err(_) => {
                                panic!("Failed to join ReMUD task")
                            }
                        }
                    }
                    _ = rx => {
                        break 'connect_loop
                    }
                }
            }

            external_tx.send((telnet_port, web_port)).unwrap_or_else(|e| panic!("failed to start server: {}", e));
        });

        let (telnet, web) = rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_else(|e| panic!("failed to receive server init message: {}", e));

        Server {
            telnet,
            web,
            runtime,
        }
    }

    pub fn telnet(&self) -> u16 {
        self.telnet
    }

    pub fn web(&self) -> u16 {
        self.web
    }
}

pub struct TelnetClient {
    connection: Telnet,
}

impl TelnetClient {
    // Creates a new client and performs initial TELNET options negotiation.
    pub fn new(port: u16) -> Self {
        // set up connection
        let mut connection =
            Telnet::connect(("127.0.0.1", port), 1024).expect("failed to connect to ReMUD");

        if let TelnetEvent::Negotiation(NegotiationAction::Do, TelnetOption::TTYPE) = connection
            .read_timeout(Duration::from_secs(1))
            .expect("did not receive DO TTYPE")
        {
            connection.negotiate(NegotiationAction::Wont, TelnetOption::TTYPE);
        } else {
            panic!("received unexpected message waiting for DO TTYPE");
        }

        TelnetClient { connection }
    }

    pub fn recv(&mut self) -> String {
        let event = self
            .connection
            .read_timeout(Duration::from_secs(1))
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

    pub fn send<S: AsRef<str>>(&mut self, line: S) {
        self.connection
            .write(format!("{}\r\n", line.as_ref()).as_bytes())
            .unwrap_or_else(|_| panic!("failed to send '{}'", line.as_ref()));
    }

    pub fn recv_prompt(&mut self) {
        self.recv_contains(">");
    }

    pub fn create_user(&mut self, name: &str, password: &str) {
        self.info("create user");
        self.recv_contains("Name?");
        self.send(name);
        self.recv_contains("Password?");
        self.send(password);
        self.recv_contains("Verify?");
        self.send(password);
        self.recv_contains("Welcome to City Six.");
        self.recv(); // ignore the look that happens when we log in
        self.recv_prompt();
    }

    pub fn info<S: AsRef<str>>(&mut self, text: S) {
        if !text.as_ref().is_empty() {
            tracing::info!("---------- {} ----------", text.as_ref());
        }
    }

    pub fn test<S1: AsRef<str>, S2: AsRef<str>, S3: AsRef<str>>(
        &mut self,
        scenario: S1,
        command: S2,
        response_contains: Vec<S3>,
    ) {
        self.validate(scenario, command, Validate::Includes(response_contains));
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
