use std::thread;
use std::time::Duration;
use telnet::{NegotiationAction, Telnet, TelnetEvent, TelnetOption};
use tracing_test::traced_test;

/// spawn the server and wait for it to start
pub async fn init_server() {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("able to create Tokio runtime");
        rt.block_on(crate::run()).expect("run the program");
    });
    thread::sleep(Duration::from_millis(100));
}

/// initialize the basic telnet connection.
fn connect() -> Telnet {
    // set up connection
    let mut connection =
        Telnet::connect(("127.0.0.1", 2004), 256).expect("Couldn't connect to the server...");
    if let TelnetEvent::Negotiation(NegotiationAction::Do, TelnetOption::TTYPE) =
        connection.read().expect("Read Error")
    {
        connection.negotiate(NegotiationAction::Wont, TelnetOption::TTYPE);
    } else {
        panic!("unable to negotiate TTYPE");
    }
    if let TelnetEvent::Negotiation(NegotiationAction::Do, TelnetOption::NAWS) =
        connection.read().expect("Read Error")
    {
        connection.negotiate(NegotiationAction::Will, TelnetOption::NAWS);
    } else {
        panic!("unable to negotiate NAWS");
    }

    connection
}

fn login(t: &mut Telnet) -> bool {
    if let TelnetEvent::Data(data) = t.read().expect("server sends name request") {
        assert!(
            String::from_utf8_lossy(&*data).contains("Name?"),
            "asks for name"
        )
    }
    t.write(b"krixi\r\n").expect("write login");

    if let TelnetEvent::Data(data) = t.read().expect("server sends password prompt") {
        //println!("{:?}", String::from_utf8_lossy(&*data));
        assert!(
            String::from_utf8_lossy(&*data).contains("Password?"),
            "asks for password"
        )
    }
    t.write(b"krixi\r\n").expect("write pass");

    // TODO: change password, send twice once we are loading a fresh database for tests.
    // if let TelnetEvent::Data(data) = t.read().expect("server sends password challenge") {
    //     println!("{:?}", String::from_utf8_lossy(&*data));
    // }
    // t.write(b"krixi\r\n").expect("write pass 2nd time");

    if let TelnetEvent::Data(data) = t.read().expect("server sends welcome") {
        //println!("{:?}", String::from_utf8_lossy(&*data));
        assert!(
            String::from_utf8_lossy(&*data).contains("Welcome to City Six"),
            "server is polite"
        )
    }

    true
}

#[tokio::test]
#[traced_test]
async fn test_login() {
    init_server().await;

    let mut telnet = connect();

    assert!(login(&mut telnet), "unable to login");
}
