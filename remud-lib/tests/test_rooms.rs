use remud_test::{start_server, TelnetClient};

/// Validate a room connection
fn assert_there_and_back_again(t: &mut TelnetClient, from: (u32, &str), to: (u32, &str)) {
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

/// Tests the room immortal commands
#[tokio::test(flavor = "multi_thread")]
async fn test_room_new() {
    let (telnet_port, _web_port) = start_server().await;

    let mut t = TelnetClient::new(telnet_port);

    t.create_user("krixi", "password");
    t.recv_contains("Welcome to City Six.");
    t.recv_contains("The Void");
    t.recv_prompt();

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

// TODO:
async fn test_room_name() {}
async fn test_room_desc() {}
async fn test_room_link_and_unlink() {}
async fn test_room_region() {}
async fn test_room_remove() {}
