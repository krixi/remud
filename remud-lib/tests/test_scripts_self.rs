use remud_test::{AuthenticatedWebClient, JsonScript, Server, TelnetPlayer, Trigger};

async fn configure_test_object(
    web: &AuthenticatedWebClient,
    t: &mut TelnetPlayer,
    trigger: Trigger,
) {
    const SCRIPT: &'static str = "test_self_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            trigger,
            // testing SELF.object_new(id) works correctly
            format!(
                r#"SELF.object_new(2); 
SELF.whisper(EVENT.actor, "created one more robot");
            "#
            ),
        ))
        .await
        .unwrap();
    assert!(error.is_none(), "{:?}", error.unwrap());

    t.command("create prototype", "prototype new").await;
    t.command(
        "prototype name",
        "prototype 1 name infinite robot generator",
    )
    .await;
    t.command(
        "prototype keywords",
        "prototype 1 keywords set infinite robot generator",
    )
    .await;
    t.command(
        "attach script",
        format!("script {} attach-pre prototype 1", SCRIPT),
    )
    .await;
    t.command("create the generator", "object new 1").await;
    t.command("create other prototype", "prototype new").await;
    t.command("set its name", "prototype 2 name flying robot")
        .await;
    t.command("set its keywords", "prototype 2 keywords set flying robot")
        .await;
}

#[tokio::test]
async fn test_self_spawn_object() {
    const PLAYER_NAME: &'static str = "krixi";
    let (server, mut t) = Server::new_create_player(PLAYER_NAME, "let me in").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Use).await;

    t.test(
        "script can instantiate new objects",
        "use infinite robot generator",
        vec!["You use infinite robot generator."],
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
