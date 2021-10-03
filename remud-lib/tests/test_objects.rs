use remud_test::Server;
// all other object commands are tested as part of prototype testing.

// test inventory, get and drop
#[test]
fn test_object_keywords() {
    let (mut server, mut t) = Server::new_connect("krixi", "(*&%(*#&%*&");
    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );
    t.test(
        "with a description",
        "prototype 1 desc A cat snoozing noisily in her basket.",
        vec!["Updated prototype 1 description."],
    );
    t.test(
        "with a name",
        "prototype 1 name |Gold3|Chonky|-| Cat",
        vec!["Updated prototype 1 name."],
    );
    t.test(
        "with a description",
        "prototype 1 desc A cat snoozing noisily in her basket.",
        vec!["Updated prototype 1 description."],
    );
    t.test(
        "including keywords",
        "prototype 1 keywords set chonky cat",
        vec!["Updated prototype 1 keywords."],
    );
    t.test("spawn an object", "object new 1", vec!["Created object 1."]);

    t.test_many(
        "look at it using keywords",
        "look at cat",
        vec![
            vec!["Chonky Cat"],
            vec!["A cat snoozing noisily in her basket"],
        ],
    );

    t = server.restart(t);

    t.test(
        "pick it up",
        "get chonky",
        vec!["You pick up", "Chonky Cat"],
    );

    t.test(
        "check it's in inventory",
        "inventory",
        vec!["You have", "Chonky Cat"],
    );

    t = server.restart(t);

    t.test(
        "check it's still in inventory",
        "inventory",
        vec!["You have", "Chonky Cat"],
    );
    t.test("drop it", "drop cat", vec!["You drop", "Chonky Cat"]);
    t.test("check it's gone", "inventory", vec!["You have nothing"]);

    t = server.restart(t);

    // look at it in the room
    t.test("it's in the room when you look", "look", vec!["Chonky Cat"]);
    t.test_many(
        "look at it in the room",
        "look at cat",
        vec![
            vec!["Chonky Cat"],
            vec!["A cat snoozing noisily in her basket"],
        ],
    );
}

// test removing an object
#[test]
fn test_object_remove() {
    let (mut server, mut t) = Server::new_connect("krixi", "(*&%(*#&%*&");
    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );
    t.test("spawn an object", "object new 1", vec!["Created object 1."]);
    t.test(
        "spawn another object",
        "object new 1",
        vec!["Created object 2."],
    );
    t.test(
        "check they are in the room",
        "room info",
        vec!["object 1", "object 2"],
    );

    t = server.restart(t);

    t.test(
        "check they are still in the room",
        "room info",
        vec!["object 1", "object 2"],
    );

    t.test(
        "delete one of them",
        "object 1 remove",
        vec!["Removed object 1."],
    );

    t.test("check one is deleted", "room info", vec!["object 2"]);
    t.test_exclude("is it tho", "room info", vec!["object 1"]);
    t.test(
        "check removed objects info",
        "object 1 info",
        vec!["Object 1 not found"],
    );

    t = server.restart(t);

    // check one is still deleted
    t.test(
        "check one is still deleted",
        "object 1 info",
        vec!["Object 1 not found"],
    );

    t.test(
        "delete other one",
        "object 2 remove",
        vec!["Removed object 2."],
    );

    t.test_exclude(
        "both are deleted",
        "room info",
        vec!["object 1", "object 2"],
    );

    t = server.restart(t);

    t.test_exclude(
        "both are deleted after restart",
        "room info",
        vec!["object 1", "object 2"],
    );
    t.test(
        "prototype still exists",
        "prototype 1 info",
        vec!["Prototype 1", "name: object"],
    );
}
