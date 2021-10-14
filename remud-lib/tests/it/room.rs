use crate::support::{Server, TelnetPlayer};

// Validate a room connection
async fn assert_there_and_back_again(t: &mut TelnetPlayer, from: (u32, &str), to: (u32, &str)) {
    let (from_id, there) = from;
    let (to_id, back) = to;

    t.test(
        format!("make a room {}, then move there", there),
        format!("room new {}", there),
        vec![format!("Created room {}", to_id)],
    )
    .await;

    t.test("move to the new room", there, vec!["An empty room"])
        .await;

    t.test(
        format!("should be in room {} now", to_id),
        "room info",
        vec![format!("Room {}", to_id).as_str(), "krixi", back],
    )
    .await;

    t.test("make sure exits command works", "exits", vec![back])
        .await;

    t.test(
        format!("go {}, back to room {}", back, from_id),
        back,
        vec!["An empty room"],
    )
    .await;

    t.test(
        format!(
            "check the room again, should be back in room {} with an exit {}",
            from_id, there
        ),
        "room info",
        vec![
            format!("Room {}", from_id).as_str(),
            "players:",
            "krixi",
            there,
        ],
    )
    .await;

    t.test("", "exits", vec![there]).await;
}

// Tests the room immortal commands
#[tokio::test]
async fn test_room_new() {
    let (_server, mut t) = Server::new_create_player("krixi", "password").await;

    t.test("create a new room", "room new", vec!["Created room 1"])
        .await;

    t.test("teleport to it", "teleport 1", vec!["An empty room"])
        .await;

    t.test(
        "check info - should be room 1 and should contain the player",
        "room info",
        vec!["Room 1", "players:", "krixi"],
    )
    .await;

    // make a room in each direction, then move in those directions to confirm exits are set up correctly.
    assert_there_and_back_again(&mut t, (1, "north"), (2, "south")).await; // to the north
    assert_there_and_back_again(&mut t, (1, "south"), (3, "north")).await; // to the south
    assert_there_and_back_again(&mut t, (1, "east"), (4, "west")).await; // to the east
    assert_there_and_back_again(&mut t, (1, "west"), (5, "east")).await; // to the west
    assert_there_and_back_again(&mut t, (1, "up"), (6, "down")).await; // to the up
    assert_there_and_back_again(&mut t, (1, "down"), (7, "up")).await; // to the down
}

#[tokio::test]
async fn test_room_name() {
    let (_server, mut t) = Server::new_create_player("krixi", "password").await;

    t.test(
        "Check that we are in The Void room before changing",
        "room info",
        vec!["The Void"],
    )
    .await;

    t.test(
        "rename the void room",
        "room name Super Happy Fun Palace",
        vec!["Updated current room name."],
    )
    .await;

    t.test(
        "Room now has updated name via room info",
        "room info",
        vec!["name: Super Happy Fun Palace"],
    )
    .await;

    t.test(
        "Room now has updated name via look",
        "look",
        vec!["Super Happy Fun Palace"],
    )
    .await;
}

#[tokio::test]
async fn test_room_desc() {
    let (_server, mut t) = Server::new_create_player("krixi", "password").await;

    t.test(
        "Check that we are in The Void room before changing",
        "room info",
        vec!["description: A dark void extends infinitely in all directions"],
    )
    .await;

    t.test(
        "change the description of the void room",
        "room desc A waterslide spirals infinitely in every direction.",
        vec!["Updated current room description."],
    )
    .await;

    t.test(
        "Room now has updated desc via room info",
        "room info",
        vec!["description: A waterslide spirals infinitely in every direction."],
    )
    .await;

    t.test(
        "Room now has updated desc via look",
        "look",
        vec!["A waterslide spirals infinitely in every direction."],
    )
    .await;
}

async fn assert_link_and_unlink(t: &mut TelnetPlayer, there: &str, back: &str) {
    // assume room 0 (void room) and room 1 (new room).

    // Test links and movement thru them
    t.test("ensure we are in void room", "teleport 0", vec!["The Void"])
        .await;

    t.test(
        "no exits to begin with",
        "exits",
        vec!["This room has no obvious exits."],
    )
    .await;

    t.test(
        "link from void -> new in a direction",
        format!("room link {} 1", there),
        vec!["Linked", there, "room 1"],
    )
    .await;

    t.test(format!("exits now has {}", there), "exits", vec![there])
        .await;

    t.test(format!("move {}", there), there, vec!["An empty room"])
        .await;

    t.test("should be in new room", "room info", vec!["Room 1"])
        .await;

    t.test(
        "new room has no exits",
        "exits",
        vec!["This room has no obvious exits."],
    )
    .await;

    t.test(
        format!("link from new room -> void via {}", back),
        format!("room link {} 0", back),
        vec!["Linked", back, "room 0"],
    )
    .await;

    t.test(format!("new room has exit {}", back), "exits", vec![back])
        .await;

    t.test(format!("move {}", back), back, vec!["The Void"])
        .await;

    // Test unlinks
    t.test(
        "unlink exit in void room",
        format!("room unlink {}", there),
        vec!["Removed exit", there],
    )
    .await;

    t.test(
        "void room has 0 exits again",
        "exits",
        vec!["This room has no obvious exits."],
    )
    .await;

    t.test("teleport to new room", "teleport 1", vec!["An empty room"])
        .await;

    t.test("verify it still has an exit", "exits", vec![back])
        .await;

    t.test(
        "unlink exit in new room",
        format!("room unlink {}", back),
        vec!["Removed exit", back],
    )
    .await;

    t.test(
        "verify no exits",
        "exits",
        vec!["This room has no obvious exits."],
    )
    .await;
}

#[tokio::test]
async fn test_room_link_and_unlink() {
    let (_server, mut t) = Server::new_create_player("krixi", "password").await;

    t.test("create a new room", "room new", vec!["Created room 1"])
        .await;
    assert_link_and_unlink(&mut t, "north", "south").await;
    assert_link_and_unlink(&mut t, "east", "west").await;
    assert_link_and_unlink(&mut t, "up", "down").await;
}

#[tokio::test]
async fn test_room_region() {
    let (mut server, mut t) = Server::new_create_player("krixi", "password").await;

    t.test(
        "adding one region",
        "room regions add space",
        vec!["Updated room 0 regions."],
    )
    .await;

    t.test(
        "new region appears in room info",
        "room info",
        vec!["regions: space"],
    )
    .await;

    // restart to verify persistence
    t = server.restart(t).await;

    t.test(
        "new region appears in room info after restart",
        "room info",
        vec!["regions: space"],
    )
    .await;

    t.test(
        "adding multiple regions",
        "room regions add space void",
        vec!["Updated room 0 regions."],
    )
    .await;

    t.test(
        "two regions appear in room info",
        "room info",
        vec!["regions: space and void"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "two regions appear in room info after restart",
        "room info",
        vec!["regions: space and void"],
    )
    .await;

    t.test(
        "setting the region list",
        "room regions set one two three",
        vec!["Updated room 0 regions."],
    )
    .await;

    t.test(
        "multiple regions appear in room info",
        "room info",
        vec!["regions: one, three, and two"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "multiple regions appear in room info after restart",
        "room info",
        vec!["regions: one, three, and two"],
    )
    .await;

    t.test(
        "removing regions works",
        "room regions remove two",
        vec!["Updated room 0 regions."],
    )
    .await;

    t.test(
        "room info shows remaining regions",
        "room info",
        vec!["regions: one and three"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "room info shows remaining regions after restart",
        "room info",
        vec!["regions: one and three"],
    )
    .await;

    t.test(
        "removing all regions works",
        "room regions remove one three",
        vec!["Updated room 0 regions."],
    )
    .await;

    t.test(
        "room info shows no regions",
        "room info",
        vec!["regions: none"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "room info shows no regions after restart",
        "room info",
        vec!["regions: none"],
    )
    .await;

    t.test(
        "setting regions requires at least one",
        "room regions set",
        vec!["Enter one or more space separated regions"],
    )
    .await;
}

#[tokio::test]
async fn test_room_remove() {
    let (mut server, mut t) = Server::new_create_player("krixi", "password").await;

    t.test(
        "create a new room",
        "room new north",
        vec!["Created room 1"],
    )
    .await;

    t.test("move to it", "north", vec!["An empty room"]).await;

    t.test(
        "make a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    )
    .await;

    t.test("make an items", "object new 1", vec!["Created object 1."])
        .await;

    t.test("check it", "room info", vec!["Room 1", "krixi", "object 1"])
        .await;

    t = server.restart(t).await;

    t.test(
        "check it after restart",
        "room info",
        vec!["Room 1", "krixi", "object 1"],
    )
    .await;

    t.test("inspect object", "object 1 info", vec!["location: room 1"])
        .await;

    t.test(
        "removing the room transports you and all items in it to the void room",
        "room remove",
        // sent to all players when the room is destroyed
        vec![
            "The world begins to disintegrate around you.", // confirmation
            "Room 1 removed.",
        ],
    )
    .await;

    // a prompt shows up after processing room remove. another one will appear after
    // the look action executes.
    t.consume_prompt().await;
    // you automatically look when entering the void
    t.line_contains("The Void").await;
    t.line_contains("A dark void extends infinitely in all directions")
        .await;
    t.line_contains("You see object").await;
    t.assert_prompt().await;

    t.test(
        "check void room",
        "room info",
        vec!["Room 0", "krixi", "object 1"],
    )
    .await;

    t = server.restart(t).await;

    t.test(
        "check void room after restart",
        "room info",
        vec!["Room 0", "krixi", "object 1"],
    )
    .await;

    t.test(
        "teleporting to removed room doesn't work",
        "teleport 1",
        vec!["Room 1 doesn't exist"],
    )
    .await;
}
