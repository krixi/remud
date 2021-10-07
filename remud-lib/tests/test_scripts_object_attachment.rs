use std::time::Duration;

use remud_test::{JsonScript, Match, Matcher, Server, Trigger};

/// test scripts object interface
#[tokio::test]
async fn test_script_object_attachment_attach_init() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Init,
            r#"SELF.say("hello");"#,
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    )
    .await;

    t.test(
        "attach init script to prototype",
        format!("scripts {} attach-init prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec!["Prototype 1", format!("Init -> {}", SCRIPT).as_str()],
    )
    .await;

    t.test("create object", "object new 1", vec!["Created object 1"])
        .await;

    // objects created with init scripts immediately run them
    t.consume_prompt().await;
    t.line_contains(r#"object says "hello""#).await;
    t.assert_prompt().await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("Init -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t.test(
        "run init scripts",
        "object 1 init",
        vec!["Initializing object 1 with 1 script."],
    )
    .await;

    t.consume_prompt().await;
    t.line_contains(r#"object says "hello""#).await;
    t.assert_prompt().await;

    t.test(
        "detach init script from prototype",
        format!("scripts {} detach prototype 1", SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SCRIPT)],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    )
    .await;

    t = server.restart(t).await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    )
    .await;
}

#[tokio::test]
async fn test_script_object_attachment_attach_pre() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello"); }"#,
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    )
    .await;

    t.test(
        "attach pre-action script to prototype",
        format!("scripts {} attach-pre prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    )
    .await;

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to prototype post-restart",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t.test("create object", "object new 1", vec!["Created object 1"])
        .await;

    t.test("greet object", "say hello", vec![""]).await;

    t.consume_prompt().await;
    t.line_contains(r#"object says "hello""#).await;
    t.assert_prompt().await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t.test("greet object", "say hello", vec![""]).await;

    t.consume_prompt().await;
    t.line_contains(r#"object says "hello""#).await;
    t.assert_prompt().await;

    t.test(
        "detach pre-event script from prototype",
        format!("scripts {} detach prototype 1", SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SCRIPT)],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;

    t = server.restart(t).await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;
}

#[tokio::test]
async fn test_script_object_attachment_attach_pre_disallow_action() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { allow_action = false; SELF.say("shh..."); }"#,
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    )
    .await;

    t.test(
        "attach pre-action script to prototype",
        format!("scripts {} attach-pre prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    )
    .await;

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to prototype post-restart",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t.test("create object", "object new 1", vec!["Created object 1"])
        .await;

    t.test(
        "object contains pre-event script",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    // the 'say hello' command is prevented by the script
    t.send("say hello").await;
    t.line_contains(r#"object says "shh...""#).await;
    t.assert_prompt().await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t.send("say hello").await;
    t.line_contains(r#"object says "shh...""#).await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_script_object_attachment_attach_post() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello"); }"#,
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    )
    .await;

    t.test(
        "attach post-action script to prototype",
        format!("scripts {} attach-post prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t.test("create object", "object new 1", vec!["Created object 1"])
        .await;

    t.test("greet object", "say hello", vec!["You say"]).await;

    t.consume_prompt().await;
    t.line_contains(r#"object says "hello""#).await;
    t.assert_prompt().await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    )
    .await;

    t.test("greet object", "say hello", vec!["You say"]).await;

    t.consume_prompt().await;
    t.line_contains(r#"object says "hello""#).await;
    t.assert_prompt().await;

    t.test(
        "detach post-event script from prototype",
        format!("scripts {} detach prototype 1", SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SCRIPT)],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;

    t = server.restart(t).await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    )
    .await;
}

#[tokio::test]
async fn test_script_object_attachment_attach_timer() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd").await;
    let web = server.login_web(&t).await;

    const TIMER_NAME: &'static str = "react";
    const SAY_SCRIPT: &'static str = "say_script";
    const TIMER_SCRIPT: &'static str = "timer_script";

    let error = web
        .create_script(&JsonScript::new(
            SAY_SCRIPT,
            Trigger::Say,
            format!(
                r#"if WORLD.is_player(EVENT.actor) {{ SELF.timer("{}", ms(100)); }}"#,
                TIMER_NAME
            )
            .as_str(),
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    let error = web
        .create_script(&JsonScript::new(
            TIMER_SCRIPT,
            Trigger::Timer,
            r#"SELF.say("What's all this?");"#,
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    )
    .await;

    t.test(
        "attach say script to prototype",
        format!("scripts {} attach-post prototype 1", SAY_SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SAY_SCRIPT)],
    )
    .await;

    t.test(
        "attach timer script to prototype",
        format!(
            "scripts {} attach-timer {} prototype 1",
            TIMER_SCRIPT, TIMER_NAME
        ),
        vec![format!("Script {} attached to prototype 1.", TIMER_SCRIPT)],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "ensure scripts are attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("react") -> {}"#, TIMER_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test("create object", "object new 1", vec!["Created object 1"])
        .await;

    t.test("greet object", "say hello", vec![r#"You say "hello""#])
        .await;

    tokio::time::sleep(Duration::from_millis(150)).await;

    t.consume_prompt().await;
    t.line_contains(r#"object says "What's all this?""#).await;
    t.assert_prompt().await;

    t = server.restart(t).await;

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("react") -> {}"#, TIMER_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test("greet object", "say hello", vec![""]).await;

    t.consume_prompt().await;
    t.line_contains(r#"object says "What's all this?""#).await;
    t.assert_prompt().await;

    t.test(
        "detach post-event script from prototype",
        format!("scripts {} detach prototype 1", SAY_SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SAY_SCRIPT)],
    )
    .await;

    t.test(
        "detach timer script from prototype",
        format!("scripts {} detach prototype 1", TIMER_SCRIPT),
        vec![format!(
            "Detached script {} from prototype 1.",
            TIMER_SCRIPT
        )],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    )
    .await;

    t = server.restart(t).await;

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    )
    .await;
}

// removal from prototype reflected in object is tested above
#[tokio::test]
async fn test_script_object_attachment_inherit_scripts() {
    let (mut server, mut t) = Server::new_create_player("Shane", "_)(@${P :@L KL J").await;
    let web = server.login_web(&t).await;

    const SCRIPT: &'static str = "the_script";
    const OTHER_SCRIPT: &'static str = "the_other_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello"); }"#,
        ))
        .await
        .unwrap();
    assert!(error.is_none());
    let error = web
        .create_script(&JsonScript::new(
            OTHER_SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello there"); }"#,
        ))
        .await
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    )
    .await;

    t.test_exclude(
        "ensure no scripts are attached to prototype",
        "prototype 1 info",
        vec!["->"],
    )
    .await;

    t.test("create object", "object new 1", vec!["Created object 1"])
        .await;

    t.test_exclude(
        "ensure no scripts are attached to object",
        "object 1 info",
        vec!["->"],
    )
    .await;

    t.test(
        "attach post-action script to prototype",
        format!("scripts {} attach-post prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    )
    .await;

    t.test(
        "attach other post-action script to prototype",
        format!("scripts {} attach-post prototype 1", OTHER_SCRIPT),
        vec![format!("Script {} attached to prototype 1.", OTHER_SCRIPT)],
    )
    .await;

    t.test(
        "ensure scripts are attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test(
        "ensure scripts are attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "ensure scripts are attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test(
        "ensure scripts are attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test(
        "remove other script from prototype",
        format!("scripts {} detach prototype 1", OTHER_SCRIPT),
        vec![format!(
            "Detached script {} from prototype 1.",
            OTHER_SCRIPT
        )],
    )
    .await;

    t.test_matches(
        "ensure script is attached to prototype",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test_matches(
        "ensure script is attached to object",
        "object 1 info",
        Matcher::unordered(vec![
            Match::include("Object 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t = server.restart(t).await;

    t.test_matches(
        "ensure one script is still attached to prototype",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test_matches(
        "ensure one script is still attached to object",
        "object 1 info",
        Matcher::unordered(vec![
            Match::include("Object 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test(
        "add other script to object",
        format!("scripts {} attach-post object 1", OTHER_SCRIPT),
        vec![format!("Script {} attached to object 1.", OTHER_SCRIPT)],
    )
    .await;

    t.test_matches(
        "prototype has only one script",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test(
        "object still has both scripts",
        "object 1 info",
        vec![
            "Object 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    )
    .await;

    t = server.restart(t).await;

    t.test_matches(
        "prototype still has only one script",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test(
        "object still has both scripts",
        "object 1 info",
        vec![
            "Object 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    )
    .await;

    t.test(
        "remove script from object",
        format!("scripts {} detach object 1", SCRIPT),
        vec![format!("Detached script {} from object 1.", SCRIPT)],
    )
    .await;

    t.test_matches(
        "prototype still has only one script",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test_matches(
        "object has only other script",
        "object 1 info",
        Matcher::unordered(vec![
            Match::include("Object 1"),
            Match::include(format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str()),
            Match::exclude(SCRIPT),
        ]),
    )
    .await;

    t = server.restart(t).await;

    t.test_matches(
        "prototype still has only one script",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test_matches(
        "object still has only other script",
        "object 1 info",
        Matcher::unordered(vec![
            Match::include("Object 1"),
            Match::include(format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str()),
            Match::exclude(SCRIPT),
        ]),
    )
    .await;

    t.test(
        "object inherits from prototype",
        "object 1 inherit scripts",
        vec!["Object 1 fields set to inherit."],
    )
    .await;

    t.test_matches(
        "prototype still has only one script",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test_matches(
        "object now has only script",
        "object 1 info",
        Matcher::unordered(vec![
            Match::include("Object 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t = server.restart(t).await;

    t.test_matches(
        "prototype still has only one script",
        "prototype 1 info",
        Matcher::unordered(vec![
            Match::include("Prototype 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;

    t.test_matches(
        "object still has only script",
        "object 1 info",
        Matcher::unordered(vec![
            Match::include("Object 1"),
            Match::include(format!("PostEvent(Say) -> {}", SCRIPT).as_str()),
            Match::exclude(OTHER_SCRIPT),
        ]),
    )
    .await;
}
