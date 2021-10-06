use std::{borrow::Cow, time::Duration};

use ::telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};

use itertools::Itertools;

pub struct TelnetConnection {
    connection: Telnet,
}

impl TelnetConnection {
    // Creates a new client and performs initial TELNET options negotiation.
    pub fn new(port: u16) -> Self {
        // set up connection
        let mut connection =
            Telnet::connect(("127.0.0.1", port), 1024).expect("failed to connect to ReMUD");

        if let TelnetEvent::Negotiation(NegotiationAction::Do, TelnetOption::TTYPE) = connection
            .read_timeout(Duration::from_secs(10))
            .expect("did not receive DO TTYPE")
        {
            connection.negotiate(NegotiationAction::Wont, TelnetOption::TTYPE);
        } else {
            panic!("received unexpected message waiting for DO TTYPE");
        }

        TelnetConnection { connection }
    }

    pub fn info<'a, S>(&mut self, text: S)
    where
        S: Into<Cow<'a, str>>,
    {
        let text = text.into();
        if !text.is_empty() {
            tracing::info!("---------- {} ----------", text);
        }
    }

    pub fn send<'a, S>(&mut self, line: S)
    where
        S: Into<Cow<'a, str>>,
    {
        let line = line.into();
        self.connection
            .write(format!("{}\r\n", line.as_ref()).as_bytes())
            .unwrap_or_else(|_| panic!("failed to send '{}'", line));
    }

    pub fn recv(&mut self) -> String {
        let event = self
            .connection
            .read_timeout(Duration::from_secs(10))
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

    pub fn recv_contains<'a, S>(&mut self, text: S)
    where
        S: Into<Cow<'a, str>>,
    {
        let text = text.into();
        let message = self.recv();
        assert!(
            message.contains(text.as_ref()),
            "did not find '{}' in message {:?}",
            text,
            message,
        )
    }

    pub fn recv_contains_all<'a, S>(&mut self, msgs: Vec<S>)
    where
        S: Into<Cow<'a, str>>,
    {
        let message = self.recv();
        for text in msgs {
            let text = text.into();
            assert!(
                message.contains(text.as_ref()),
                "did not find '{}' in message {:?}",
                text,
                message,
            )
        }
    }
    pub fn recv_contains_none<'a, S>(&mut self, msgs: Vec<S>)
    where
        S: Into<Cow<'a, str>>,
    {
        let message = self.recv();
        for text in msgs {
            let text = text.into();
            assert!(
                !message.contains(text.as_ref()),
                "found unwanted '{}' in message {:?}",
                text,
                message,
            )
        }
    }

    pub fn recv_prompt(&mut self) {
        self.recv_contains(">");
    }

    pub fn test<'a, S1, S2, S3>(&mut self, scenario: S1, command: S2, response_contains: Vec<S3>)
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
        S3: Into<Cow<'a, str>>,
    {
        let includes = response_contains.into_iter().map(Into::into).collect_vec();
        self.validate(scenario, command, Validate::Includes(includes));
    }

    pub fn test_many<'a, S1, S2, S3>(
        &mut self,
        scenario: S1,
        command: S2,
        response_contains: Vec<Vec<S3>>,
    ) where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
        S3: Into<Cow<'a, str>> + Clone,
    {
        self.info(scenario);
        self.send(command);
        for expected in response_contains.iter() {
            self.recv_contains_all(expected.clone());
        }
        self.recv_prompt();
    }

    pub fn test_exclude<'a, S1, S2, S3>(
        &mut self,
        scenario: S1,
        command: S2,
        response_excludes: Vec<S3>,
    ) where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
        S3: Into<Cow<'a, str>>,
    {
        self.validate(
            scenario,
            command,
            Validate::Excludes(response_excludes.into_iter().map(Into::into).collect_vec()),
        );
    }

    fn validate<'a, S1, S2>(&mut self, scenario: S1, command: S2, validate: Validate)
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
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

pub struct TelnetPlayer {
    pub name: String,
    pub password: String,
    connection: TelnetConnection,
}

impl TelnetPlayer {
    pub fn new<'a, S1, S2>(connection: TelnetConnection, player: S1, password: S2) -> Self
    where
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        TelnetPlayer {
            name: player.into().to_owned().to_string(),
            password: password.into().to_owned().to_string(),
            connection,
        }
    }
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

impl TelnetPlayer {}

enum Validate<'a> {
    Includes(Vec<Cow<'a, str>>),
    Excludes(Vec<Cow<'a, str>>),
}
