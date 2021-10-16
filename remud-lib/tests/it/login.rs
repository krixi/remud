use std::time::Duration;

use crate::support::Server;

#[tokio::test]
async fn test_login_create_player() {
    let (_server, _t) = Server::new_create_player("Shane", "s;kladjf").await;
}

#[tokio::test]
async fn test_login_create_verify_failed() {
    let (_server, mut t) = Server::new_connect_telnet().await;

    t.line_contains("Connected to").await;
    t.line_contains("Name?").await;
    t.assert_prompt().await;

    t.test(
        "enter name",
        "Shane",
        vec!["New user detected", "Password?"],
    )
    .await;

    t.test(
        "enter password",
        "some pw",
        vec!["Password accepted.", "Verify?"],
    )
    .await;

    t.test(
        "fail verify password",
        "some other pw",
        vec!["Verification failed", "Password?"],
    )
    .await;

    t.test(
        "enter password",
        "some pw",
        vec!["Password accepted.", "Verify?"],
    )
    .await;

    t.test(
        "verify password",
        "some pw",
        vec!["Password verified.", "Welcome to City Six", "The Void"],
    )
    .await;

    t.assert_prompt().await;
}

#[tokio::test]
async fn test_login_login_player() {
    let (mut server, mut t) = Server::new_create_player("Shane", "s;kladjf").await;
    t.assert_prompt().await;

    drop(t);
    std::thread::sleep(Duration::from_secs(1));

    let mut t = server.login_player("Shane", "s;kladjf").await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_login_already_online() {
    let (server, mut t) = Server::new_create_player("Shane", "some pw").await;
    t.assert_prompt().await;

    let mut t2 = server.connect_telnet();
    t2.line_contains("Connected to").await;
    t2.line_contains("Name?").await;
    t2.assert_prompt().await;

    t2.test(
        "enter name",
        "Shane",
        vec!["Error retrieving user.", "Name?"],
    )
    .await;

    drop(t);
    std::thread::sleep(Duration::from_secs(1));

    t2.test("enter name", "Shane", vec!["User located.", "Password?"])
        .await;

    t2.test(
        "verify password",
        "some pw",
        vec!["Password verified.", "Welcome to City Six", "The Void"],
    )
    .await;
    t2.assert_prompt().await;
}

#[tokio::test]
async fn test_login_bad_player_name() {
    let (_server, mut t) = Server::new_connect_telnet().await;

    t.line_contains("Connected to").await;
    t.line_contains("Name?").await;
    t.assert_prompt().await;

    t.test(
        "enter name",
        "$@()* ()% (#%)#%(( ",
        vec!["Invalid username.", "Name?"],
    )
    .await;

    t.test(
        "enter name",
        "Shane",
        vec!["New user detected", "Password?"],
    )
    .await;

    t.test(
        "enter password",
        "some pw",
        vec!["Password accepted.", "Verify?"],
    )
    .await;

    t.test(
        "verify password",
        "some pw",
        vec!["Password verified.", "Welcome to City Six", "The Void"],
    )
    .await;

    t.assert_prompt().await;
}

#[tokio::test]
async fn test_login_bad_password() {
    let (_server, mut t) = Server::new_connect_telnet().await;

    t.line_contains("Connected to").await;
    t.line_contains("Name?").await;
    t.assert_prompt().await;

    t.test(
        "enter name",
        "Shane",
        vec!["New user detected.", "Password?"],
    )
    .await;

    t.test(
        "enter bad password",
        "ok",
        vec!["Weak password detected", "Password?"],
    )
    .await;

    t.test(
        "enter password",
        "some pw",
        vec!["Password accepted.", "Verify?"],
    )
    .await;

    t.test(
        "verify password",
        "some pw",
        vec!["Password verified.", "Welcome to City Six", "The Void"],
    )
    .await;

    t.assert_prompt().await;
}

#[tokio::test]
async fn test_login_verify_failed() {
    let (server, t) = Server::new_create_player("Shane", "password").await;

    drop(t);
    std::thread::sleep(Duration::from_secs(1));

    let mut t = server.connect_telnet();
    t.line_contains("Connected to").await;
    t.line_contains("Name?").await;
    t.assert_prompt().await;

    t.test("enter name", "Shane", vec!["User located.", "Password?"])
        .await;

    t.test(
        "enter bad password",
        "ok",
        vec!["Weak password detected.", "Name?"],
    )
    .await;

    t.test("enter name", "Shane", vec!["User located.", "Password?"])
        .await;

    t.test(
        "verify password",
        "password",
        vec!["Password verified.", "Welcome to City Six", "The Void"],
    )
    .await;

    t.assert_prompt().await;
}

#[tokio::test]
async fn test_login_create_player_change_password() {
    const NAME: &'static str = "Shane";
    const FIRST_PW: &'static str = "my fine password";
    const NEW_PW: &'static str = "my new and improved password";
    let (mut server, mut t) = Server::new_create_player(NAME, FIRST_PW).await;

    t.test(
        "enter change password",
        "password",
        vec!["Password Update", "Current password?"],
    )
    .await;

    t.test("invalid password", "ok", vec!["Verification failed."])
        .await;

    t.test(
        "enter change password",
        "password",
        vec!["Password Update", "Current password?"],
    )
    .await;

    t.test(
        "enter password",
        FIRST_PW,
        vec!["Password verified.", "New password?"],
    )
    .await;

    t.test("weak password", "ok", vec!["Weak password detected."])
        .await;

    t.test(
        "enter change password",
        "password",
        vec!["Password Update", "Current password?"],
    )
    .await;

    t.test(
        "enter password",
        FIRST_PW,
        vec!["Password verified.", "New password?"],
    )
    .await;

    t.test(
        "new password",
        NEW_PW,
        vec!["Password accepted.", "Confirm?"],
    )
    .await;

    t.test("fail confirm", "ok", vec!["Confirmation failed."])
        .await;

    t.test(
        "enter change password",
        "password",
        vec!["Password Update", "Current password?"],
    )
    .await;

    t.test(
        "enter password",
        FIRST_PW,
        vec!["Password verified.", "New password?"],
    )
    .await;

    t.test(
        "new password",
        NEW_PW,
        vec!["Password accepted.", "Confirm?"],
    )
    .await;

    t.test("confirm new password", NEW_PW, vec!["Password updated."])
        .await;

    drop(t);

    let t = server.login_player(NAME, NEW_PW).await;

    let _ = server.restart(t).await;
}
