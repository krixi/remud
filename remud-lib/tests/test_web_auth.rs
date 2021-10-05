use std::time::Duration;

use remud_test::{Server, StatusCode};

#[test]
fn test_web_auth_success() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd");
    let _web = server.login_web(&t);
}

#[test]
fn test_web_auth_failure() {
    let server = Server::default();
    let web = server.connect_web();
    match web.login("Shane", "and how") {
        Err(StatusCode::UNAUTHORIZED) => (),
        _ => panic!("expected unauthorized"),
    }
}

#[test]
fn test_web_auth_token_refresh() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd");
    let mut web = server.login_web(&t);
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().unwrap();
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().unwrap();
}

#[test]
fn test_web_auth_old_access_kills_tokens() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd");
    let mut web = server.login_web(&t);
    let old_web = web.clone();

    // Refresh and verify old is unauthorized
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().unwrap();
    std::thread::sleep(Duration::from_secs(1));

    match old_web.list_scripts() {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got {:?}", e),
    }

    // New tokens should now fail as well
    match web.list_scripts() {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got {:?}", e),
    }
    match web.refresh_auth() {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got {:?}", e),
    }
}

#[test]
fn test_web_auth_old_refresh_kills_tokens() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd");
    let mut web = server.login_web(&t);
    let mut old_web = web.clone();

    // Refresh and verify old is unauthorized
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().unwrap();
    std::thread::sleep(Duration::from_secs(1));

    match old_web.refresh_auth() {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got: {:?}", e),
    }

    // New tokens should now fail as well
    match web.list_scripts() {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got: {:?}", e),
    }
    match web.refresh_auth() {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got: {:?}", e),
    }
}
