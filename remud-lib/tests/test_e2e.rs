use remud_test::{start_server, TelnetClient};

#[tokio::test(flavor = "multi_thread")]
async fn test_login() {
    let (telnet_port, _web_port) = start_server().await;

    let mut client = TelnetClient::new(telnet_port);

    client.create_user("krixi", "password");
    client.recv_contains("Welcome to City Six.");
    client.recv_contains("The Void");
    client.recv_prompt();
}
