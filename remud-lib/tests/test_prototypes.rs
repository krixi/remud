use remud_test::Server;

#[tokio::test]
async fn test_prototype_new() {
    let (mut server, mut t) = Server::new_create_player("shane", "password").await;

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;

    t.test(
        "created prototype has expected attributes",
        "prototype 1 info",
        vec![
            "Prototype 1",
            "name: object",
            "description: A nondescript object.",
            "flags: (empty)",
            "keywords: object",
            "script hooks: none",
        ],
    )
    .await;

    t.test(
        "create object from prototype",
        "object new 1",
        vec!["Created object 1."],
    )
    .await;

    t.test(
        "created object has expected attributes",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            "inherit scripts: true",
            "name: object",
            "description: A nondescript object.",
            "flags: (empty)",
            "keywords: object",
            "location: room 0",
            "script hooks: none",
            "script data: none",
            "timers: none",
            "fsm: none",
        ],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "created prototype has expected attributes after restart",
        "prototype 1 info",
        vec![
            "Prototype 1",
            "name: object",
            "description: A nondescript object.",
            "flags: (empty)",
            "keywords: object",
            "script hooks: none",
        ],
    )
    .await;

    t.test(
        "created object has expected attributes after restart",
        "object 1 info",
        vec![
            "Object 1",
            "prototype: 1",
            "inherit scripts: true",
            "name: object",
            "description: A nondescript object.",
            "flags: (empty)",
            "keywords: object",
            "location: room 0",
            "script hooks: none",
            "script data: none",
            "timers: none",
            "fsm: none",
        ],
    )
    .await;
}

#[tokio::test]
async fn test_prototype_list() {
    let (mut server, mut t) = Server::new_create_player("krixi", "password").await;
    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;
    t.test(
        "give it a name",
        "prototype 1 name fancy thinger",
        vec!["Updated prototype 1 name."],
    )
    .await;
    t.test(
        "check it exists in the list",
        "prototype list",
        vec!["ID 1", "fancy thinger"],
    )
    .await;
    t = server.restart(t).await;
    t.test(
        "check it still exists in the list",
        "prototype list",
        vec!["ID 1", "fancy thinger"],
    )
    .await;
    t.test(
        "create a second prototype",
        "prototype new",
        vec!["Created prototype 2"],
    )
    .await;
    t.test(
        "give it a name",
        "prototype 2 name apple",
        vec!["Updated prototype 2 name."],
    )
    .await;
    t.test(
        "list should be sorted by id",
        "prototype list",
        vec!["ID 1", "fancy thinger", "ID 2", "apple"],
    )
    .await;
}

#[tokio::test]
async fn test_prototype_flags() {
    let (mut server, mut t) = Server::new_create_player("shane", "some^gibberish$password").await;

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;

    t.test("create a object", "object new 1", vec!["Created object 1."])
        .await;

    t.test(
        "set prototype flags",
        "prototype 1 set subtle",
        vec!["Updated prototype 1 flags."],
    )
    .await;

    t.test(
        "prototype has correct flags",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    )
    .await;

    t.test(
        "object has updated flags",
        "object 1 info",
        vec!["Object 1", "flags: SUBTLE"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype has correct flags",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    )
    .await;

    t.test(
        "object has updated flags",
        "object 1 info",
        vec!["Object 1", "flags: SUBTLE"],
    )
    .await;

    t.test(
        "set object flags to prevent inheritence",
        "object 1 set fixed",
        vec!["Updated object 1 flags."],
    )
    .await;

    t.test(
        "prototype flags haven't changed",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    )
    .await;

    t.test(
        "object has updated flags",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype flags haven't changed after restart",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    )
    .await;

    t.test(
        "object flags haven't changed after restart",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    )
    .await;

    t.test(
        "update prototype flags to check object inheritence",
        "prototype 1 unset SUBTLE",
        vec!["Updated prototype 1 flags."],
    )
    .await;

    t.test(
        "prototype flags are updated",
        "prototype 1 info",
        vec!["Prototype 1", "flags: (empty)"],
    )
    .await;

    t.test(
        "object flags didn't change when prototype was updated",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype flags didn't change after restart",
        "prototype 1 info",
        vec!["Prototype 1", "flags: (empty)"],
    )
    .await;

    t.test(
        "object flags didn't change after restart",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    )
    .await;
}

#[tokio::test]
async fn test_prototype_name() {
    let (mut server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&").await;

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;

    t.test("create a object", "object new 1", vec!["Created object 1."])
        .await;

    t.test(
        "set prototype name",
        "prototype 1 name jar of peanut butter",
        vec!["Updated prototype 1 name."],
    )
    .await;

    t.test(
        "prototype has correct name",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    )
    .await;

    t.test(
        "object has updated name",
        "object 1 info",
        vec!["Object 1", "name: jar of peanut butter"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype has correct name",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    )
    .await;

    t.test(
        "object has updated name",
        "object 1 info",
        vec!["Object 1", "name: jar of peanut butter"],
    )
    .await;

    t.test(
        "set object name to prevent inheritence",
        "object 1 name jar of jelly",
        vec!["Updated object 1 name."],
    )
    .await;

    t.test(
        "prototype name hasn't changed",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    )
    .await;

    t.test(
        "object has updated name",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype name hasn't changed after restart",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    )
    .await;

    t.test(
        "object name hasn't changed after restart",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    )
    .await;

    t.test(
        "update prototype flags to check object inheritence",
        "prototype 1 name jar of vegemite",
        vec!["Updated prototype 1 name."],
    )
    .await;

    t.test(
        "prototype name is updated",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of vegemite"],
    )
    .await;

    t.test(
        "object name didn't change when prototype was updated",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype name didn't change after restart",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of vegemite"],
    )
    .await;

    t.test(
        "object name didn't change after restart",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    )
    .await;
}

#[tokio::test]
async fn test_prototype_desc() {
    let (mut server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&").await;

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;

    t.test("create a object", "object new 1", vec!["Created object 1."])
        .await;

    t.test(
        "set prototype desc",
        "prototype 1 desc A prototype rests here.",
        vec!["Updated prototype 1 description."],
    )
    .await;

    t.test(
        "prototype has correct desc",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    )
    .await;

    t.test(
        "object has updated desc",
        "object 1 info",
        vec!["Object 1", "description: A prototype rests here."],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype has correct desc",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    )
    .await;

    t.test(
        "object has updated desc",
        "object 1 info",
        vec!["Object 1", "description: A prototype rests here."],
    )
    .await;

    t.test(
        "set object desc to prevent inheritence",
        "object 1 desc An object rests here.",
        vec!["Updated object 1 description."],
    )
    .await;

    t.test(
        "prototype description hasn't changed",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    )
    .await;

    t.test(
        "object has updated desc",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype desc hasn't changed after restart",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    )
    .await;

    t.test(
        "object desc hasn't changed after restart",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    )
    .await;

    t.test(
        "update prototype name to check object inheritence",
        "prototype 1 desc A fancy prototype.",
        vec!["Updated prototype 1 description."],
    )
    .await;

    t.test(
        "prototype desc is updated",
        "prototype 1 info",
        vec!["Prototype 1", "description: A fancy prototype."],
    )
    .await;

    t.test(
        "object desc didn't change when prototype was updated",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype desc didn't change after restart",
        "prototype 1 info",
        vec!["Prototype 1", "description: A fancy prototype."],
    )
    .await;

    t.test(
        "object desc didn't change after restart",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    )
    .await;
}

#[tokio::test]
async fn test_prototype_keywords() {
    let (mut server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&").await;

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;

    t.test(
        "create an object",
        "object new 1",
        vec!["Created object 1."],
    )
    .await;

    t.test(
        "set prototype keywords",
        "prototype 1 keywords set dirty boots",
        vec!["Updated prototype 1 keywords."],
    )
    .await;

    t.test(
        "prototype keywords updated",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    )
    .await;

    t.test(
        "object keywords updated",
        "object 1 info",
        vec!["keywords: boots and dirty"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype keywords haven't changed",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    )
    .await;

    t.test(
        "object keywords haven't changed",
        "object 1 info",
        vec!["keywords: boots and dirty"],
    )
    .await;

    t.test(
        "update object keywords",
        "object 1 keywords remove dirty",
        vec!["Updated object 1 keywords."],
    )
    .await;

    t.test(
        "prototype keywords haven't changed",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    )
    .await;

    t.test(
        "object keywords are updated",
        "object 1 info",
        vec!["keywords: boots"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype keywords haven't changed after restart",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    )
    .await;

    t.test(
        "object keywords haven't changed after restart",
        "object 1 info",
        vec!["keywords: boots"],
    )
    .await;

    t.test(
        "change prototype keywords - check object inheritence",
        "prototype 1 keywords add black",
        vec!["Updated prototype 1 keywords."],
    )
    .await;

    t.test(
        "prototype keywords are updated",
        "prototype 1 info",
        vec!["keywords: black, boots, and dirty"],
    )
    .await;

    t.test(
        "object keywords haven't changed",
        "object 1 info",
        vec!["keywords: boots"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "prototype keywords haven't changed after restart",
        "prototype 1 info",
        vec!["keywords: black, boots, and dirty"],
    )
    .await;

    t.test(
        "object keywords haven't changed after restart",
        "object 1 info",
        vec!["keywords: boots"],
    )
    .await;
}

#[tokio::test]
async fn test_prototype_remove() {
    let (_server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&").await;

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;

    t.test(
        "cannot remove prototype",
        "prototype 1 remove",
        vec!["Enter a valid prototype subcommand"],
    )
    .await;
}
