use crate::support::{Match, Matcher, Server};
// all other object commands are tested as part of prototype testing.

// test inventory, get and drop
#[tokio::test]
async fn test_object_keywords() {
    let (mut server, mut t) = Server::new_create_player("krixi", "(*&%(*#&%*&").await;
    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;
    t.test(
        "with a description",
        "prototype 1 desc A cat snoozing noisily in her basket.",
        vec!["Updated prototype 1 description."],
    )
    .await;
    t.test(
        "with a name",
        "prototype 1 name |Gold3|Chonky|-| Cat",
        vec!["Updated prototype 1 name."],
    )
    .await;
    t.test(
        "including keywords",
        "prototype 1 keywords set chonky cat",
        vec!["Updated prototype 1 keywords."],
    )
    .await;
    t.test("spawn an object", "object new 1", vec!["Created object 1."])
        .await;

    t.test(
        "look at it using keywords",
        "look at cat",
        vec!["Chonky Cat", "A cat snoozing noisily in her basket"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "pick it up",
        "get chonky",
        vec!["You pick up", "Chonky Cat"],
    )
    .await;

    t.test(
        "check it's in inventory",
        "inventory",
        vec!["You have", "Chonky Cat"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "check it's still in inventory",
        "inventory",
        vec!["You have", "Chonky Cat"],
    )
    .await;
    t.test("drop it", "drop cat", vec!["You drop", "Chonky Cat"])
        .await;
    t.test("check it's gone", "inventory", vec!["You have nothing"])
        .await;

    t = server.restart(t).await;

    // look at it in the room
    t.test("it's in the room when you look", "look", vec!["Chonky Cat"])
        .await;
    t.test(
        "look at it in the room",
        "look at cat",
        vec!["Chonky Cat", "A cat snoozing noisily in her basket"],
    )
    .await;
}

// test removing an object
#[tokio::test]
async fn test_object_remove() {
    let (mut server, mut t) = Server::new_create_player("krixi", "(*&%(*#&%*&").await;
    t.test(
        "create a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;
    t.test("spawn an object", "object new 1", vec!["Created object 1."])
        .await;
    t.test(
        "spawn another object",
        "object new 1",
        vec!["Created object 2."],
    )
    .await;
    t.test(
        "check they are in the room",
        "room info",
        vec!["object 1", "object 2"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "check they are still in the room",
        "room info",
        vec!["object 1", "object 2"],
    )
    .await;

    t.test(
        "delete one of them",
        "object 1 remove",
        vec!["Removed object 1."],
    )
    .await;

    t.test_matches(
        "check one is deleted",
        "room info",
        Matcher::unordered(vec![Match::include("object 2"), Match::exclude("object 1")]),
    )
    .await;

    t.test(
        "check removed objects info",
        "object 1 info",
        vec!["Object 1 not found"],
    )
    .await;

    t = server.restart(t).await;

    // check one is still deleted
    t.test(
        "check one is still deleted",
        "object 1 info",
        vec!["Object 1 not found"],
    )
    .await;

    t.test(
        "delete other one",
        "object 2 remove",
        vec!["Removed object 2."],
    )
    .await;

    t.test_exclude(
        "both are deleted",
        "room info",
        vec!["object 1", "object 2"],
    )
    .await;

    t = server.restart(t).await;

    t.test_exclude(
        "both are deleted after restart",
        "room info",
        vec!["object 1", "object 2"],
    )
    .await;
    t.test(
        "prototype still exists",
        "prototype 1 info",
        vec!["Prototype 1", "name: object"],
    )
    .await;
}
