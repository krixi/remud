use remud_test::{AuthenticatedWebClient, JsonScript, Server, TelnetPlayer, Trigger};
use std::time::Duration;

async fn configure_test_object(
    web: &AuthenticatedWebClient,
    t: &mut TelnetPlayer,
    trigger: Trigger,
    script_name: &str,
    code: &str,
) {
    let error = web
        .create_script(&JsonScript::new(script_name, trigger, code))
        .await
        .unwrap();
    assert!(error.is_none(), "{:?}", error.unwrap());

    t.command("create prototype", "prototype new").await;
    t.command("prototype name", "prototype 1 name widget").await;
    t.command("prototype keywords", "prototype 1 keywords set widget")
        .await;
    t.command(
        "attach script",
        format!("script {} attach-pre prototype 1", script_name),
    )
    .await;
    t.command("create the widget", "object new 1").await;
}

#[tokio::test]
async fn test_self_object_new() {
    const PLAYER_NAME: &'static str = "krixi";
    let (server, mut t) = Server::new_create_player(PLAYER_NAME, "let me in").await;
    let web = server.login_web(&t).await;

    configure_test_object(
        &web,
        &mut t,
        Trigger::Use,
        "test_self_script",
        r#"SELF.object_new(2); 
SELF.whisper(EVENT.actor, "created one more robot");"#,
    )
    .await;

    t.command("create other prototype", "prototype new").await;
    t.command("set its name", "prototype 2 name flying robot")
        .await;
    t.command("set its keywords", "prototype 2 keywords set flying robot")
        .await;

    t.test(
        "script can instantiate new objects",
        "use widget",
        vec!["You use widget."],
    )
    .await;
    t.consume_prompt().await;
    t.line_contains(r#"created one more robot"#).await;
    t.assert_prompt().await;

    t.test(
        "there should be a flying robot in the room now",
        "look",
        vec!["flying robot"],
    )
    .await;
}

#[tokio::test]
async fn test_self_object_remove() {
    const PLAYER_NAME: &'static str = "krixi";
    let (server, mut t) = Server::new_create_player(PLAYER_NAME, "let me in").await;
    let web = server.login_web(&t).await;

    configure_test_object(
        &web,
        &mut t,
        Trigger::Use,
        "test_self_script",
        r#"SELF.object_remove(SELF.entity);"#,
    )
    .await;

    t.test(
        "there should be a widget on the ground",
        "look",
        vec!["widget"],
    )
    .await;
    t.test("if you use it", "use widget", vec!["You use widget."])
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    t.test_exclude("it disappears!", "look", vec!["widget"])
        .await;
}
