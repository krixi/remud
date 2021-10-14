use crate::support::{AuthenticatedWebClient, JsonScript, Server, TelnetPlayer, Trigger};

async fn configure_test_object(
    web: &AuthenticatedWebClient,
    t: &mut TelnetPlayer,
    trigger: Trigger,
    message: &str,
) {
    const SCRIPT: &'static str = "say_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            trigger,
            format!(
                r#"if WORLD.is_player(EVENT.actor) {{ SELF.say("{}");}}"#,
                message
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
async fn test_script_object_trigger_drop() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Drop, "not on me!").await;

    t.command("create prototype", "prototype new").await;
    t.command("create object", "object new 2").await;
    t.command("pick up object", "get object").await;
    t.command(
        "dropping an object causes talking rock to speak",
        "drop object",
    )
    .await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "not on me!""#).await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_emote() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Emote, "nice moves").await;

    t.command("emote to trigger object", "me dances around.")
        .await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "nice moves""#).await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_exits() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Exits, "looking for a way out?").await;

    t.command("check for a quick exit", "exits").await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "looking for a way out?""#)
        .await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_get() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Get, "me next!").await;

    t.command("create prototype", "prototype new").await;
    t.command("create object", "object new 2").await;
    t.command(
        "picking up the object causes the rock to speak",
        "get object",
    )
    .await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "me next!""#).await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_inventory() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Inventory, "put me in there!").await;

    t.command("check our inventory to trigger rock", "inventory")
        .await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "put me in there!""#)
        .await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_look() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Look, "looking for something? *wink*").await;

    t.command("look around the room", "look").await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "looking for something? *wink*""#)
        .await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_look_at() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::LookAt, "look at me instead.").await;

    t.command("look at myself", "look at Shane").await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "look at me instead.""#)
        .await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_move() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    t.command("create a room to move to", "room new east").await;
    t.command("move there - it is the spawn room", "east").await;

    configure_test_object(&web, &mut t, Trigger::Move, "where are you going?").await;

    let mut t2 = server.create_player("krixi", "IAO&*)*(&*").await;

    t.consume_prompt().await;
    t.line_contains("krixi arrives.").await;
    t.assert_prompt().await;

    t2.command("move elsewhere", "west").await;

    t.consume_prompt().await;
    t.line_contains("krixi leaves to the west").await;
    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "where are you going?""#)
        .await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_say() {
    let (server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    configure_test_object(&web, &mut t, Trigger::Say, "hello!").await;

    t.command("say hello to rock", "say hello").await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "hello!""#).await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_trigger_send() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    let _krixi = server.create_player("krixi", "kljsadf 9*(90").await;

    t.consume_prompt().await;
    t.line_contains("krixi arrives.").await;
    t.assert_prompt().await;

    configure_test_object(
        &web,
        &mut t,
        Trigger::Send,
        "sending secret messages, are we?",
    )
    .await;

    t.command(
        "send krixi a secret message",
        "send krixi this rock is weird",
    )
    .await;

    t.consume_prompt().await;
    t.line_contains(r#"talking rock says "sending secret messages, are we?""#)
        .await;
    t.assert_prompt().await;
}
