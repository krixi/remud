use std::time::Duration;

use remud_test::Server;

#[test]
fn test_login_create_player() {
    let (_server, _t) = Server::new_create_player("Shane", "s;kladjf");
}

#[test]
fn test_login_create_verify_failed() {
    let (_server, mut t) = Server::new_connect_telnet();

    t.recv_contains("Connected to");
    t.recv_contains("Name?");
    t.recv_prompt();

    t.test_many(
        "enter name",
        "Shane",
        vec![vec!["New user detected"], vec!["Password?"]],
    );

    t.test_many(
        "enter password",
        "some pw",
        vec![vec!["Password accepted."], vec!["Verify?"]],
    );

    t.test_many(
        "fail verify password",
        "some other pw",
        vec![vec!["Verification failed"], vec!["Password?"]],
    );

    t.test_many(
        "enter password",
        "some pw",
        vec![vec!["Password accepted."], vec!["Verify?"]],
    );

    t.test_many(
        "verify password",
        "some pw",
        vec![
            vec!["Password verified."],
            vec![],
            vec!["Welcome to City Six"],
            vec![],
            vec!["The Void"],
        ],
    );
}

#[test]
fn test_login_login_player() {
    let (mut server, t) = Server::new_create_player("Shane", "s;kladjf");

    drop(t);
    std::thread::sleep(Duration::from_secs(1));

    server.login_player("Shane", "s;kladjf");
}

#[test]
fn test_login_already_online() {
    let (server, t) = Server::new_create_player("Shane", "some pw");

    let mut t2 = server.connect_telnet();
    t2.recv_contains("Connected to");
    t2.recv_contains("Name?");
    t2.recv_prompt();

    t2.test_many(
        "enter name",
        "Shane",
        vec![vec!["User currently online."], vec!["Name?"]],
    );

    drop(t);
    std::thread::sleep(Duration::from_secs(1));

    t2.test_many(
        "enter name",
        "Shane",
        vec![vec!["User located."], vec!["Password?"]],
    );

    t2.test_many(
        "verify password",
        "some pw",
        vec![
            vec!["Password verified."],
            vec![],
            vec!["Welcome to City Six"],
            vec![],
            vec!["The Void"],
        ],
    );
}

#[test]
fn test_login_bad_player_name() {
    let (_server, mut t) = Server::new_connect_telnet();

    t.recv_contains("Connected to");
    t.recv_contains("Name?");
    t.recv_prompt();

    t.test_many(
        "enter name",
        "$@()* ()% (#%)#%(( ",
        vec![vec!["Invalid username."], vec!["Name?"]],
    );

    t.test_many(
        "enter name",
        "Shane",
        vec![vec!["New user detected"], vec!["Password?"]],
    );

    t.test_many(
        "enter password",
        "some pw",
        vec![vec!["Password accepted."], vec!["Verify?"]],
    );

    t.test_many(
        "verify password",
        "some pw",
        vec![
            vec!["Password verified."],
            vec![],
            vec!["Welcome to City Six"],
            vec![],
            vec!["The Void"],
        ],
    );
}

#[test]
fn test_login_bad_password() {
    let (_server, mut t) = Server::new_connect_telnet();

    t.recv_contains("Connected to");
    t.recv_contains("Name?");
    t.recv_prompt();

    t.test_many(
        "enter name",
        "Shane",
        vec![vec!["New user detected."], vec!["Password?"]],
    );

    t.test_many(
        "enter bad password",
        "ok",
        vec![vec!["Weak password detected"], vec!["Password?"]],
    );

    t.test_many(
        "enter password",
        "some pw",
        vec![vec!["Password accepted."], vec!["Verify?"]],
    );

    t.test_many(
        "verify password",
        "some pw",
        vec![
            vec!["Password verified."],
            vec![],
            vec!["Welcome to City Six"],
            vec![],
            vec!["The Void"],
        ],
    );
}

#[test]
fn test_login_verify_failed() {
    let (server, t) = Server::new_create_player("Shane", "password");

    drop(t);
    std::thread::sleep(Duration::from_secs(1));

    let mut t = server.connect_telnet();
    t.recv_contains("Connected to");
    t.recv_contains("Name?");
    t.recv_prompt();

    t.test_many(
        "enter name",
        "Shane",
        vec![vec!["User located."], vec!["Password?"]],
    );

    t.test_many(
        "enter bad password",
        "ok",
        vec![vec!["Verification failed."], vec!["Name?"]],
    );

    t.test_many(
        "enter name",
        "Shane",
        vec![vec!["User located."], vec!["Password?"]],
    );

    t.test_many(
        "verify password",
        "password",
        vec![
            vec!["Password verified."],
            vec![],
            vec!["Welcome to City Six"],
            vec![],
            vec!["The Void"],
        ],
    );
}
