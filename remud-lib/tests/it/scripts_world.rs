use crate::support::{AuthenticatedWebClient, JsonScript, Server, TelnetPlayer, Trigger};

async fn configure_test_object(
    web: &AuthenticatedWebClient,
    t: &mut TelnetPlayer,
    trigger: Trigger,
) {
    const SCRIPT: &'static str = "say_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            trigger,
            // testing WORLD.name() works correctly
            format!(
                r#"let name = WORLD.name(EVENT.actor); 
                   SELF.say(`i see you ${{name}}`);
               "#
            ),
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    t.command("create prototype", "prototype new").await;
    t.command("prototype name", "prototype 1 name talking rock")
        .await;
    t.command(
        "prototype keywords",
        "prototype 1 keywords set talking rock",
    )
    .await;
    t.command(
        "attach script",
        format!("script {} attach-pre prototype 1", SCRIPT),
    )
    .await;
    t.command("create object", "object new 1").await;
}

#[tokio::test]
async fn test_world_get_entity_name() {
    const PLAYER_NAME: &'static str = "krixi";
    let (server, mut t) = Server::new_create_player(PLAYER_NAME, "let me in").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Look).await;

    t.command(
        "script attached to look can read player's name correctly",
        "look",
    )
    .await;
    t.consume_prompt().await;
    t.line_contains(format!("talking rock says \"i see you {}\"", PLAYER_NAME))
        .await;
    t.assert_prompt().await;
}
