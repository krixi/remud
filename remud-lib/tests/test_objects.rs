use remud_test::Server;
// all other object commands are tested as part of prototype testing.

// test inventory, get and drop
#[test]
fn test_object_keywords() {
    let (server, mut t) = Server::new_connect("krixi", "(*&%(*#&%*&");
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

    // restart
    // pick it up
    // check it's in inventory
    // restart
    // check it's still in inventory
    // drop it
    // restart
    // look at it in the room
}

// test removing an object
#[test]
fn test_object_remove() {
    todo!()
    // create a prototype
    // create two instances of it as objects
    // check they are in the room
    // restart
    // check they are sitll in the room
    // delete one of them
    // check one is deleted
    // restart
    // check one is still deleted
    // delete other one
    // check prototype still exists
    // restart
    // check prototype still exists
    // check 0 objects in the room
}
