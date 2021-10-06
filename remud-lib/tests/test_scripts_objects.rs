#![allow(dead_code)]

use remud_test::{JsonScript, Server, Trigger};

/// test scripts object interface
#[test]
fn test_script_object_attach_init() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd");
    let web = server.login_web(&t);

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Init,
            r#"SELF.say("hello");"#,
        ))
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    );

    t.test(
        "attach init script to prototype",
        format!("scripts {} attach-init prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    );

    t = server.restart(t);

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec!["Prototype 1", format!("Init -> {}", SCRIPT).as_str()],
    );

    t.test("create object", "object new 1", vec!["Created object 1"]);

    // objects created with init scripts immediately run them
    t.recv();
    t.recv_contains(r#"object says "hello""#);
    t.recv_prompt();

    t = server.restart(t);

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("Init -> {}", SCRIPT).as_str(),
        ],
    );

    t.test(
        "run init scripts",
        "object 1 init",
        vec!["Initializing object 1 with 1 script."],
    );

    t.recv();
    t.recv_contains(r#"object says "hello""#);
    t.recv_prompt();

    t.test(
        "detach init script from prototype",
        format!("scripts {} detach prototype 1", SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SCRIPT)],
    );

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    );

    t = server.restart(t);

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("Init -> {}", SCRIPT).as_str()],
    );
}

#[test]
fn test_script_object_attach_pre() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd");
    let web = server.login_web(&t);

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello"); }"#,
        ))
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    );

    t.test(
        "attach pre-action script to prototype",
        format!("scripts {} attach-pre prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    );

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t = server.restart(t);

    t.test(
        "ensure script is attached to prototype post-restart",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t.test("create object", "object new 1", vec!["Created object 1"]);

    t.test("greet object", "say hello", vec![""]);

    t.recv();
    t.recv_contains(r#"object says "hello""#);
    t.recv_prompt();

    t = server.restart(t);

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t.test("greet object", "say hello", vec![""]);

    t.recv();
    t.recv_contains(r#"object says "hello""#);
    t.recv_prompt();

    t.test(
        "detach pre-event script from prototype",
        format!("scripts {} detach prototype 1", SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SCRIPT)],
    );

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    );

    t = server.restart(t);

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PreEvent(Say) -> {}", SCRIPT).as_str()],
    );
}

#[test]
fn test_script_object_attach_pre_disallow_action() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd");
    let web = server.login_web(&t);

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { allow_action = false; SELF.say("shh..."); }"#,
        ))
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    );

    t.test(
        "attach pre-action script to prototype",
        format!("scripts {} attach-pre prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    );

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t = server.restart(t);

    t.test(
        "ensure script is attached to prototype post-restart",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t.test("create object", "object new 1", vec!["Created object 1"]);

    t.test(
        "object contains pre-event script",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    // the 'say hello' command is prevented by the script
    t.send("say hello");
    t.recv_contains(r#"object says "shh...""#);
    t.recv_prompt();

    t = server.restart(t);

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PreEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t.send("say hello");
    t.recv_contains(r#"object says "shh...""#);
    t.recv_prompt();
}

#[test]
fn test_script_object_attach_post() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd");
    let web = server.login_web(&t);

    const SCRIPT: &'static str = "the_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello"); }"#,
        ))
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    );

    t.test(
        "attach post-action script to prototype",
        format!("scripts {} attach-post prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    );

    t = server.restart(t);

    t.test(
        "ensure script is attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t.test("create object", "object new 1", vec!["Created object 1"]);

    t.test("greet object", "say hello", vec![""]);

    t.recv();
    t.recv_contains(r#"object says "hello""#);
    t.recv_prompt();

    t = server.restart(t);

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
        ],
    );

    t.test("greet object", "say hello", vec![""]);

    t.recv();
    t.recv_contains(r#"object says "hello""#);
    t.recv_prompt();

    t.test(
        "detach post-event script from prototype",
        format!("scripts {} detach prototype 1", SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SCRIPT)],
    );

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    );

    t = server.restart(t);

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec!["->", format!("PostEvent(Say) -> {}", SCRIPT).as_str()],
    );
}

#[test]
fn test_script_object_attach_timer() {
    let (mut server, mut t) = Server::new_create_player("Shane", "p@55w0rd");
    let web = server.login_web(&t);

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
        .unwrap();
    assert!(error.is_none());

    let error = web
        .create_script(&JsonScript::new(
            TIMER_SCRIPT,
            Trigger::Timer,
            r#"SELF.say("What's all this?");"#,
        ))
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    );

    t.test(
        "attach say script to prototype",
        format!("scripts {} attach-post prototype 1", SAY_SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SAY_SCRIPT)],
    );

    t.test(
        "attach timer script to prototype",
        format!(
            "scripts {} attach-timer {} prototype 1",
            TIMER_SCRIPT, TIMER_NAME
        ),
        vec![format!("Script {} attached to prototype 1.", TIMER_SCRIPT)],
    );

    t = server.restart(t);

    t.test(
        "ensure scripts are attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("react") -> {}"#, TIMER_SCRIPT).as_str(),
        ],
    );

    t.test("create object", "object new 1", vec!["Created object 1"]);

    t.test("greet object", "say hello", vec![""]);

    t.recv();
    t.recv_contains(r#"object says "What's all this?""#);
    t.recv_prompt();

    t = server.restart(t);

    t.test(
        "ensure script is attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("react") -> {}"#, TIMER_SCRIPT).as_str(),
        ],
    );

    t.test("greet object", "say hello", vec![""]);

    t.recv();
    t.recv_contains(r#"object says "What's all this?""#);
    t.recv_prompt();

    t.test(
        "detach post-event script from prototype",
        format!("scripts {} detach prototype 1", SAY_SCRIPT),
        vec![format!("Detached script {} from prototype 1.", SAY_SCRIPT)],
    );

    t.test(
        "detach timer script from prototype",
        format!("scripts {} detach prototype 1", TIMER_SCRIPT),
        vec![format!(
            "Detached script {} from prototype 1.",
            TIMER_SCRIPT
        )],
    );

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    );

    t = server.restart(t);

    t.test_exclude(
        "ensure script is removed from prototype",
        "prototype 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    );

    t.test_exclude(
        "ensure script is removed from object",
        "object 1 info",
        vec![
            "->",
            format!("PostEvent(Say) -> {}", SAY_SCRIPT).as_str(),
            format!(r#"Timer("{}") -> {}"#, TIMER_NAME, SAY_SCRIPT).as_str(),
        ],
    );
}

// removal from prototype reflected in object is tested above
#[test]
fn test_script_object_inherit_scripts() {
    let (mut server, mut t) = Server::new_create_player("Shane", "_)(@${P :@L KL J");
    let web = server.login_web(&t);

    const SCRIPT: &'static str = "the_script";
    const OTHER_SCRIPT: &'static str = "the_other_script";
    let error = web
        .create_script(&JsonScript::new(
            SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello"); }"#,
        ))
        .unwrap();
    assert!(error.is_none());
    let error = web
        .create_script(&JsonScript::new(
            OTHER_SCRIPT,
            Trigger::Say,
            r#"if WORLD.is_player(EVENT.actor) { SELF.say("hello there"); }"#,
        ))
        .unwrap();
    assert!(error.is_none());

    t.test(
        "create prototype",
        "prototype new",
        vec!["Created prototype 1"],
    );

    t.test_exclude(
        "ensure no scripts are attached to prototype",
        "prototype 1 info",
        vec!["->"],
    );

    t.test("create object", "object new 1", vec!["Created object 1"]);

    t.test_exclude(
        "ensure no scripts are attached to object",
        "object 1 info",
        vec!["->"],
    );

    t.test(
        "attach post-action script to prototype",
        format!("scripts {} attach-post prototype 1", SCRIPT),
        vec![format!("Script {} attached to prototype 1.", SCRIPT)],
    );

    t.test(
        "attach other post-action script to prototype",
        format!("scripts {} attach-post prototype 1", OTHER_SCRIPT),
        vec![format!("Script {} attached to prototype 1.", OTHER_SCRIPT)],
    );

    t.test(
        "ensure scripts are attached to prototype",
        "prototype 1 info",
        vec![
            "Prototype 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    );

    t.test(
        "ensure scripts are attached to object",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            format!("PostEvent(Say) -> {}", SCRIPT).as_str(),
            format!("PostEvent(Say) -> {}", OTHER_SCRIPT).as_str(),
        ],
    );
}

/// Test triggers with this type
fn test_script_object_trigger_drop() {}
fn test_script_object_trigger_emote() {}
fn test_script_object_trigger_exits() {}
fn test_script_object_trigger_get() {}
fn test_script_object_trigger_init() {}
fn test_script_object_trigger_inventory() {}
fn test_script_object_trigger_look() {}
fn test_script_object_trigger_look_at() {}
fn test_script_object_trigger_move() {}
fn test_script_object_trigger_say() {}
fn test_script_object_trigger_send() {}
fn test_script_object_trigger_timer() {}
