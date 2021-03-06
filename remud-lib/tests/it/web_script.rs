use std::str::FromStr;

use crate::support::{JsonScript, JsonScriptName, JsonScriptResponse, Server, StatusCode, Trigger};

#[tokio::test]
async fn test_web_script_create() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const S1_NAME: &'static str = "ts_1";
    const S1_CODE: &'static str = "let x = 1;";

    // create initial script
    match web
        .create_script(&JsonScript::new(S1_NAME, Trigger::Init, S1_CODE))
        .await
    {
        Ok(None) => (),
        _ => panic!("expected no errors"),
    }

    // error on duplicate scripts
    match web
        .create_script(&JsonScript::new(S1_NAME, Trigger::Init, S1_CODE))
        .await
    {
        Err(StatusCode::CONFLICT) => (),
        _ => panic!("expected duplicate error"),
    }

    const S2_NAME: &'static str = "ts_2";
    const S2_CODE: &'static str = "kj asldjkf kjlasdfj sdf ;;;;;;;;";

    // bad code returns errors
    match web
        .create_script(&JsonScript::new(S2_NAME, Trigger::Init, S2_CODE))
        .await
    {
        Ok(Some(_)) => (),
        _ => panic!("expected no errors"),
    }
}

#[tokio::test]
async fn test_web_script_read() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const S1_NAME: &'static str = "ts_1";
    const S1_CODE: &'static str = "let x = 1;";

    // test read nonexistent
    match web.read_script(&JsonScriptName::from(S1_NAME)).await {
        Err(StatusCode::NOT_FOUND) => (),
        _ => panic!("expected script response"),
    }

    // test create and read
    web.create_script(&JsonScript::new(S1_NAME, Trigger::Init, S1_CODE))
        .await
        .unwrap();

    match web.read_script(&JsonScriptName::from(S1_NAME)).await {
        Ok(JsonScriptResponse {
            name,
            trigger,
            code,
            error,
        }) => {
            assert_eq!(name.as_str(), S1_NAME);
            assert!(matches!(
                Trigger::from_str(trigger.as_str()).unwrap(),
                Trigger::Init
            ));
            assert_eq!(code.as_str(), S1_CODE);
            assert!(error.is_none());
        }
        Err(_) => panic!("expected script response"),
    }

    const S2_NAME: &'static str = "ts_2";
    const S2_CODE: &'static str = "kj asldjkf kjlasdfj sdf ;;;;;;;;";

    // test create w/compile error and read
    web.create_script(&JsonScript::new(S2_NAME, Trigger::Init, S2_CODE))
        .await
        .unwrap();

    match web.read_script(&JsonScriptName::from(S2_NAME)).await {
        Ok(JsonScriptResponse {
            name,
            trigger,
            code,
            error,
        }) => {
            assert_eq!(name.as_str(), S2_NAME);
            assert!(matches!(
                Trigger::from_str(trigger.as_str()).unwrap(),
                Trigger::Init
            ));
            assert_eq!(code.as_str(), S2_CODE);
            assert!(error.is_some());
        }
        Err(_) => panic!("expected script response"),
    }
}

#[tokio::test]
async fn test_web_script_update() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const S1_NAME: &'static str = "ts_1";
    const S1_CODE: &'static str = "let x = 1;";

    // test update nonexistent
    match web
        .update_script(&JsonScript::new(S1_NAME, Trigger::Init, S1_CODE))
        .await
    {
        Err(StatusCode::NOT_FOUND) => (),
        _ => panic!("expected not found"),
    }

    // create script
    match web
        .create_script(&JsonScript::new(S1_NAME, Trigger::Init, S1_CODE))
        .await
    {
        Ok(None) => (),
        _ => panic!("expected no errors"),
    }

    const BAD_CODE: &'static str = "kj asldjkf kjlasdfj sdf ;;;;;;;;";

    // update script with code that doesn't compile
    match web
        .update_script(&JsonScript::new(S1_NAME, Trigger::Init, BAD_CODE))
        .await
    {
        Ok(response) => assert!(response.error.is_some()),
        _ => panic!("expected no errors"),
    }

    // confirm script reads as expected
    match web.read_script(&JsonScriptName::from(S1_NAME)).await {
        Ok(JsonScriptResponse {
            name,
            trigger,
            code,
            error,
        }) => {
            assert_eq!(name.as_str(), S1_NAME);
            assert!(matches!(
                Trigger::from_str(trigger.as_str()).unwrap(),
                Trigger::Init
            ));
            assert_eq!(code.as_str(), BAD_CODE);
            assert!(error.is_some());
        }
        Err(_) => panic!("expected script response"),
    }

    const GOOD_CODE: &'static str = "let z = 2;";

    // update script with code that compiles again
    match web
        .update_script(&JsonScript::new(S1_NAME, Trigger::Init, GOOD_CODE))
        .await
    {
        Ok(response) => assert!(response.error.is_none()),
        _ => panic!("expected no errors"),
    }

    // confirm script reads as expected
    match web.read_script(&JsonScriptName::from(S1_NAME)).await {
        Ok(JsonScriptResponse {
            name,
            trigger,
            code,
            error,
        }) => {
            assert_eq!(name.as_str(), S1_NAME);
            assert!(matches!(
                Trigger::from_str(trigger.as_str()).unwrap(),
                Trigger::Init
            ));
            assert_eq!(code.as_str(), GOOD_CODE);
            assert!(error.is_none());
        }
        Err(_) => panic!("expected script response"),
    }
}

#[tokio::test]
async fn test_web_script_delete() {
    let (server, t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const S1_NAME: &'static str = "ts_1";
    const S1_CODE: &'static str = "let x = 1;";

    match web.delete_script(&JsonScriptName::from(S1_NAME)).await {
        Err(StatusCode::NOT_FOUND) => (),
        e => panic!("expected not found, got: {:?}", e),
    }

    web.create_script(&JsonScript::new(S1_NAME, Trigger::Init, S1_CODE))
        .await
        .unwrap();

    match web.delete_script(&JsonScriptName::from(S1_NAME)).await {
        Ok(_) => (),
        Err(e) => panic!("expected ok, got: {:?}", e),
    }

    web.create_script(&JsonScript::new(S1_NAME, Trigger::Init, S1_CODE))
        .await
        .unwrap();
}
