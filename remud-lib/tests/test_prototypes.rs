use remud_test::Server;

#[test]
fn test_prototype_new() {
    let (mut server, mut t) = Server::new_create_player("shane", "password");

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );

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
    );

    t.test(
        "create object from prototype",
        "object new 1",
        vec!["Created object 1."],
    );

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
    );

    t = server.restart(t);

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
    );

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
    );
}

#[test]
fn test_prototype_list() {
    let (mut server, mut t) = Server::new_create_player("krixi", "password");
    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );
    t.test(
        "give it a name",
        "prototype 1 name fancy thinger",
        vec!["Updated prototype 1 name."],
    );
    t.test(
        "check it exists in the list",
        "prototype list",
        vec!["ID 1", "fancy thinger"],
    );
    t = server.restart(t);
    t.test(
        "check it still exists in the list",
        "prototype list",
        vec!["ID 1", "fancy thinger"],
    );
    t.test(
        "create a second prototype",
        "prototype new",
        vec!["Created prototype 2"],
    );
    t.test(
        "give it a name",
        "prototype 2 name apple",
        vec!["Updated prototype 2 name."],
    );
    t.test_many(
        "list should be sorted by id",
        "prototype list",
        vec![vec!["ID 1", "fancy thinger"], vec!["ID 2", "apple"]],
    );
}

#[test]
fn test_prototype_flags() {
    let (mut server, mut t) = Server::new_create_player("shane", "some^gibberish$password");

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );

    t.test("create a object", "object new 1", vec!["Created object 1."]);

    t.test(
        "set prototype flags",
        "prototype 1 set subtle",
        vec!["Updated prototype 1 flags."],
    );

    t.test(
        "prototype has correct flags",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    );

    t.test(
        "object has updated flags",
        "object 1 info",
        vec!["Object 1", "flags: SUBTLE"],
    );

    t = server.restart(t);

    t.test(
        "prototype has correct flags",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    );

    t.test(
        "object has updated flags",
        "object 1 info",
        vec!["Object 1", "flags: SUBTLE"],
    );

    t.test(
        "set object flags to prevent inheritence",
        "object 1 set fixed",
        vec!["Updated object 1 flags."],
    );

    t.test(
        "prototype flags haven't changed",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    );

    t.test(
        "object has updated flags",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    );

    t = server.restart(t);

    t.test(
        "prototype flags haven't changed after restart",
        "prototype 1 info",
        vec!["Prototype 1", "flags: SUBTLE"],
    );

    t.test(
        "object flags haven't changed after restart",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    );

    t.test(
        "update prototype flags to check object inheritence",
        "prototype 1 unset SUBTLE",
        vec!["Updated prototype 1 flags."],
    );

    t.test(
        "prototype flags are updated",
        "prototype 1 info",
        vec!["Prototype 1", "flags: (empty)"],
    );

    t.test(
        "object flags didn't change when prototype was updated",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    );

    t = server.restart(t);

    t.test(
        "prototype flags didn't change after restart",
        "prototype 1 info",
        vec!["Prototype 1", "flags: (empty)"],
    );

    t.test(
        "object flags didn't change after restart",
        "object 1 info",
        vec!["Object 1", "flags: FIXED | SUBTLE"],
    );
}

#[test]
fn test_prototype_name() {
    let (mut server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&");

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );

    t.test("create a object", "object new 1", vec!["Created object 1."]);

    t.test(
        "set prototype name",
        "prototype 1 name jar of peanut butter",
        vec!["Updated prototype 1 name."],
    );

    t.test(
        "prototype has correct name",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    );

    t.test(
        "object has updated name",
        "object 1 info",
        vec!["Object 1", "name: jar of peanut butter"],
    );

    t = server.restart(t);

    t.test(
        "prototype has correct name",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    );

    t.test(
        "object has updated name",
        "object 1 info",
        vec!["Object 1", "name: jar of peanut butter"],
    );

    t.test(
        "set object name to prevent inheritence",
        "object 1 name jar of jelly",
        vec!["Updated object 1 name."],
    );

    t.test(
        "prototype name hasn't changed",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    );

    t.test(
        "object has updated name",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    );

    t = server.restart(t);

    t.test(
        "prototype name hasn't changed after restart",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of peanut butter"],
    );

    t.test(
        "object name hasn't changed after restart",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    );

    t.test(
        "update prototype flags to check object inheritence",
        "prototype 1 name jar of vegemite",
        vec!["Updated prototype 1 name."],
    );

    t.test(
        "prototype name is updated",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of vegemite"],
    );

    t.test(
        "object name didn't change when prototype was updated",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    );

    t = server.restart(t);

    t.test(
        "prototype name didn't change after restart",
        "prototype 1 info",
        vec!["Prototype 1", "name: jar of vegemite"],
    );

    t.test(
        "object name didn't change after restart",
        "object 1 info",
        vec!["Object 1", "name: jar of jelly"],
    );
}

#[test]
fn test_prototype_desc() {
    let (mut server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&");

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );

    t.test("create a object", "object new 1", vec!["Created object 1."]);

    t.test(
        "set prototype desc",
        "prototype 1 desc A prototype rests here.",
        vec!["Updated prototype 1 description."],
    );

    t.test(
        "prototype has correct desc",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    );

    t.test(
        "object has updated desc",
        "object 1 info",
        vec!["Object 1", "description: A prototype rests here."],
    );

    t = server.restart(t);

    t.test(
        "prototype has correct desc",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    );

    t.test(
        "object has updated desc",
        "object 1 info",
        vec!["Object 1", "description: A prototype rests here."],
    );

    t.test(
        "set object desc to prevent inheritence",
        "object 1 desc An object rests here.",
        vec!["Updated object 1 description."],
    );

    t.test(
        "prototype description hasn't changed",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    );

    t.test(
        "object has updated desc",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    );

    t = server.restart(t);

    t.test(
        "prototype desc hasn't changed after restart",
        "prototype 1 info",
        vec!["Prototype 1", "description: A prototype rests here."],
    );

    t.test(
        "object desc hasn't changed after restart",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    );

    t.test(
        "update prototype name to check object inheritence",
        "prototype 1 desc A fancy prototype.",
        vec!["Updated prototype 1 description."],
    );

    t.test(
        "prototype desc is updated",
        "prototype 1 info",
        vec!["Prototype 1", "description: A fancy prototype."],
    );

    t.test(
        "object desc didn't change when prototype was updated",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    );

    t = server.restart(t);

    t.test(
        "prototype desc didn't change after restart",
        "prototype 1 info",
        vec!["Prototype 1", "description: A fancy prototype."],
    );

    t.test(
        "object desc didn't change after restart",
        "object 1 info",
        vec!["Object 1", "description: An object rests here."],
    );
}

#[test]
fn test_prototype_keywords() {
    let (mut server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&");

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );

    t.test(
        "create an object",
        "object new 1",
        vec!["Created object 1."],
    );

    t.test(
        "set prototype keywords",
        "prototype 1 keywords set dirty boots",
        vec!["Updated prototype 1 keywords."],
    );

    t.test(
        "prototype keywords updated",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    );

    t.test(
        "object keywords updated",
        "object 1 info",
        vec!["keywords: boots and dirty"],
    );

    t = server.restart(t);

    t.test(
        "prototype keywords haven't changed",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    );

    t.test(
        "object keywords haven't changed",
        "object 1 info",
        vec!["keywords: boots and dirty"],
    );

    t.test(
        "update object keywords",
        "object 1 keywords remove dirty",
        vec!["Updated object 1 keywords."],
    );

    t.test(
        "prototype keywords haven't changed",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    );

    t.test(
        "object keywords are updated",
        "object 1 info",
        vec!["keywords: boots"],
    );

    t = server.restart(t);

    t.test(
        "prototype keywords haven't changed after restart",
        "prototype 1 info",
        vec!["keywords: boots and dirty"],
    );

    t.test(
        "object keywords haven't changed after restart",
        "object 1 info",
        vec!["keywords: boots"],
    );

    t.test(
        "change prototype keywords - check object inheritence",
        "prototype 1 keywords add black",
        vec!["Updated prototype 1 keywords."],
    );

    t.test(
        "prototype keywords are updated",
        "prototype 1 info",
        vec!["keywords: black, boots, and dirty"],
    );

    t.test(
        "object keywords haven't changed",
        "object 1 info",
        vec!["keywords: boots"],
    );

    t = server.restart(t);

    t.test(
        "prototype keywords haven't changed after restart",
        "prototype 1 info",
        vec!["keywords: black, boots, and dirty"],
    );

    t.test(
        "object keywords haven't changed after restart",
        "object 1 info",
        vec!["keywords: boots"],
    );
}

#[test]
fn test_prototype_remove() {
    let (_server, mut t) = Server::new_create_player("shane", "(*&%(*#&%*&");

    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );

    t.test(
        "cannot remove prototype",
        "prototype 1 remove",
        vec!["Enter a valid prototype subcommand"],
    );
}
