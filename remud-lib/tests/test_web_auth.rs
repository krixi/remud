use std::time::Duration;

use remud_test::{Server, StatusCode};

#[tokio::test]
async fn test_web_auth_success() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let _web = server.login_web(&t).await;
}

#[tokio::test]
async fn test_web_auth_failure() {
    let server = Server::new().await;
    let web = server.connect_web();
    match web.login("Shane", "and how").await {
        Err(StatusCode::UNAUTHORIZED) => (),
        _ => panic!("expected unauthorized"),
    }
}

#[tokio::test]
async fn test_web_auth_token_refresh() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let mut web = server.login_web(&t).await;
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().await.unwrap();
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().await.unwrap();
}

#[tokio::test]
async fn test_web_auth_old_access_kills_tokens() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let mut web = server.login_web(&t).await;
    let old_web = web.clone();

    // Refresh and verify old is unauthorized
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().await.unwrap();
    std::thread::sleep(Duration::from_secs(1));

    match old_web.list_scripts().await {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got {:?}", e),
    }

    // New tokens should now fail as well
    match web.list_scripts().await {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got {:?}", e),
    }
    match web.refresh_auth().await {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got {:?}", e),
    }
}

#[tokio::test]
async fn test_web_auth_old_refresh_kills_tokens() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let mut web = server.login_web(&t).await;
    let mut old_web = web.clone();

    // Refresh and verify old is unauthorized
    std::thread::sleep(Duration::from_secs(1));
    web.refresh_auth().await.unwrap();
    std::thread::sleep(Duration::from_secs(1));

    match old_web.refresh_auth().await {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got: {:?}", e),
    }

    // New tokens should now fail as well
    match web.list_scripts().await {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got: {:?}", e),
    }
    match web.refresh_auth().await {
        Err(StatusCode::UNAUTHORIZED) => (),
        e => panic!("expected unauthorized, got: {:?}", e),
    }
}
