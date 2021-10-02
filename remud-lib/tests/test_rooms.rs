use remud_test::{Server, TelnetPlayer};

// Validate a room connection
fn assert_there_and_back_again(t: &mut TelnetPlayer, from: (u32, &str), to: (u32, &str)) {
    let (from_id, there) = from;
    let (to_id, back) = to;

    t.test(
        format!("make a room {}, then move there", there),
        format!("room new {}", there),
        vec![format!("Created room {}", to_id)],
    );
    t.test("move to the new room", there, vec!["An empty room"]);
    t.test(
        format!("should be in room {} now", to_id),
        "room info",
        vec![format!("Room {}", to_id).as_str(), "krixi", back],
    );
    t.test("make sure exits command works", "exits", vec![back]);
    t.test(
        format!("go {}, back to room {}", back, from_id),
        back,
        vec!["An empty room"],
    );
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
    );
    t.test("", "exits", vec![there]);
}

// Tests the room immortal commands
#[test]
fn test_room_new() {
    let (_server, mut t) = Server::new_connect("krixi", "password");

    t.test("create a new room", "room new", vec!["Created room 1"]);
    t.test("teleport to it", "teleport 1", vec!["An empty room"]);
    t.test(
        "check info - should be room 1 and should contain the player",
        "room info",
        vec!["Room 1", "players:", "krixi"],
    );

    // make a room in each direction, then move in those directions to confirm exits are set up correctly.
    assert_there_and_back_again(&mut t, (1, "north"), (2, "south")); // to the north
    assert_there_and_back_again(&mut t, (1, "south"), (3, "north")); // to the south
    assert_there_and_back_again(&mut t, (1, "east"), (4, "west")); // to the east
    assert_there_and_back_again(&mut t, (1, "west"), (5, "east")); // to the west
    assert_there_and_back_again(&mut t, (1, "up"), (6, "down")); // to the up
    assert_there_and_back_again(&mut t, (1, "down"), (7, "up")); // to the down
}

#[test]
fn test_room_name() {
    let (_server, mut t) = Server::new_connect("krixi", "password");

    t.test(
        "Check that we are in The Void room before changing",
        "room info",
        vec!["The Void"],
    );
    t.test(
        "rename the void room",
        "room name Super Happy Fun Palace",
        vec!["Updated current room name."],
    );
    t.test(
        "Room now has updated name via room info",
        "room info",
        vec!["name: Super Happy Fun Palace"],
    );
    t.test(
        "Room now has updated name via look",
        "look",
        vec!["Super Happy Fun Palace"],
    );
}

#[test]
fn test_room_desc() {
    let (_server, mut t) = Server::new_connect("krixi", "password");

    t.test(
        "Check that we are in The Void room before changing",
        "room info",
        vec!["description: A dark void extends infinitely in all directions"],
    );
    t.test(
        "change the description of the void room",
        "room desc A waterslide spirals infinitely in every direction.",
        vec!["Updated current room description."],
    );
    t.test(
        "Room now has updated desc via room info",
        "room info",
        vec!["description: A waterslide spirals infinitely in every direction."],
    );
    t.test(
        "Room now has updated desc via look",
        "look",
        vec!["A waterslide spirals infinitely in every direction."],
    );
}

fn assert_link_and_unlink(t: &mut TelnetPlayer, there: &str, back: &str) {
    // assume room 0 (void room) and room 1 (new room).

    // Test links and movement thru them
    t.test("ensure we are in void room", "teleport 0", vec!["The Void"]);

    t.test(
        "no exits to begin with",
        "exits",
        vec!["This room has no obvious exits."],
    );
    t.test(
        "link from void -> new in a direction",
        format!("room link {} 1", there),
        vec!["Linked", there, "room 1"],
    );
    t.test(format!("exits now has {}", there), "exits", vec![there]);
    t.test(format!("move {}", there), there, vec!["An empty room"]);
    t.test("should be in new room", "room info", vec!["Room 1"]);
    t.test(
        "new room has no exits",
        "exits",
        vec!["This room has no obvious exits."],
    );
    t.test(
        format!("link from new room -> void via {}", back),
        format!("room link {} 0", back),
        vec!["Linked", back, "room 0"],
    );
    t.test(format!("new room has exit {}", back), "exits", vec![back]);
    t.test(format!("move {}", back), back, vec!["The Void"]);

    // Test unlinks
    t.test(
        "unlink exit in void room",
        format!("room unlink {}", there),
        vec!["Removed exit", there],
    );
    t.test(
        "void room has 0 exits again",
        "exits",
        vec!["This room has no obvious exits."],
    );
    t.test("teleport to new room", "teleport 1", vec!["An empty room"]);
    t.test("verify it still has an exit", "exits", vec![back]);
    t.test(
        "unlink exit in new room",
        format!("room unlink {}", back),
        vec!["Removed exit", back],
    );
    t.test(
        "verify no exits",
        "exits",
        vec!["This room has no obvious exits."],
    );
}

#[test]
fn test_room_link_and_unlink() {
    let (_server, mut t) = Server::new_connect("krixi", "password");

    t.test("create a new room", "room new", vec!["Created room 1"]);
    assert_link_and_unlink(&mut t, "north", "south");
    assert_link_and_unlink(&mut t, "east", "west");
    assert_link_and_unlink(&mut t, "up", "down");
}

#[test]
fn test_room_region() {
    let (mut server, mut t) = Server::new_connect("krixi", "password");

    t.test(
        "adding one region",
        "room regions add space",
        vec!["Updated room 0 regions."],
    );
    t.test(
        "new region appears in room info",
        "room info",
        vec!["regions: space"],
    );

    // restart to verify persistence
    t = server.restart(t);

    t.test(
        "new region appears in room info after restart",
        "room info",
        vec!["regions: space"],
    );

    t.test(
        "adding multiple regions",
        "room regions add space void",
        vec!["Updated room 0 regions."],
    );
    t.test(
        "two regions appear in room info",
        "room info",
        vec!["regions: space and void"],
    );

    t = server.restart(t);

    t.test(
        "two regions appear in room info after restart",
        "room info",
        vec!["regions: space and void"],
    );

    t.test(
        "setting the region list",
        "room regions set one two three",
        vec!["Updated room 0 regions."],
    );
    t.test(
        "multiple regions appear in room info",
        "room info",
        vec!["regions: one, three, and two"],
    );

    t = server.restart(t);

    t.test(
        "multiple regions appear in room info after restart",
        "room info",
        vec!["regions: one, three, and two"],
    );

    t.test(
        "removing regions works",
        "room regions remove two",
        vec!["Updated room 0 regions."],
    );
    t.test(
        "room info shows remaining regions",
        "room info",
        vec!["regions: one and three"],
    );

    t = server.restart(t);

    t.test(
        "room info shows remaining regions after restart",
        "room info",
        vec!["regions: one and three"],
    );

    t.test(
        "removing all regions works",
        "room regions remove one three",
        vec!["Updated room 0 regions."],
    );
    t.test(
        "room info shows no regions",
        "room info",
        vec!["regions: none"],
    );

    t = server.restart(t);

    t.test(
        "room info shows no regions after restart",
        "room info",
        vec!["regions: none"],
    );

    t.test(
        "setting regions requires at least one",
        "room regions set",
        vec!["Enter one or more space separated regions"],
    )
}

#[test]
fn test_room_remove() {
    let (mut server, mut t) = Server::new_connect("krixi", "password");

    t.test(
        "create a new room",
        "room new north",
        vec!["Created room 1"],
    );
    t.test("move to it", "north", vec!["An empty room"]);
    t.test(
        "make a prototype",
        "prototype new",
        vec!["Created prototype 1."],
    );
    t.test("make an items", "object new 1", vec!["Created object 1."]);
    t.test("check it", "room info", vec!["Room 1", "krixi", "object 1"]);

    t = server.restart(t);

    t.test(
        "check it after restart",
        "room info",
        vec!["Room 1", "krixi", "object 1"],
    );

    t.test("inspect object", "object 1 info", vec!["container: room 1"]);

    t.test_many(
        "removing the room transports you and all items in it to the void room",
        "room remove",
        vec![
            // sent to all players when the room is destroyed
            vec!["The world begins to disintegrate around you."],
            // confirmation
            vec!["Room 1 removed."],
        ],
    );

    // you automatically look when entering the void
    t.recv(); // ignore the random \r\n for now // TODO
    t.recv_contains_all(vec![
        "A dark void extends infinitely in all directions",
        "You see object",
    ]);
    t.recv_prompt();

    t.test(
        "check void room",
        "room info",
        vec!["Room 0", "krixi", "object 1"],
    );

    t = server.restart(t);

    t.test(
        "check void room after restart",
        "room info",
        vec!["Room 0", "krixi", "object 1"],
    );

    t.test(
        "teleporting to removed room doesn't work",
        "teleport 1",
        vec!["Room 1 doesn't exist"],
    );
}
