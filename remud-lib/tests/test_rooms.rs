use remud_test::{start_server, TelnetClient};

/// Tests the room immortal commands
#[tokio::test(flavor = "multi_thread")]
async fn test_room_new() {
    let (telnet_port, _web_port) = start_server().await;

    let mut client = TelnetClient::new(telnet_port);

    client.create_user("krixi", "password");
    client.recv_contains("Welcome to City Six.");
    client.recv_contains("The Void");
    client.recv_prompt();

    client.info("create a new room");
    client.send("room new");
    client.recv_contains("Created room 1");
    client.recv_prompt();

    client.info("teleport to it");
    client.send("teleport 1");
    client.recv_contains("An empty room");
    client.recv_prompt();

    client.info("check info - should be room 1 and should contain the player");
    client.send("room info");
    client.recv_contains_all(vec!["Room 1", "players:", "krixi"]);
    client.recv_prompt();

    // make a room in each direction, then move in those directions to confirm exits are set up correctly.
    client.info("make a room north, then move there");
    client.send("room new north");
    client.recv_contains("Created room 2");
    client.recv_prompt();

    client.info("move to the new room");
    client.send("north");
    client.recv_contains("An empty room");
    client.recv_prompt();

    client.info("should be in room 2 now");
    client.send("room info");
    client.recv_contains_all(vec!["Room 2", "krixi", "south"]);
    client.recv_prompt();

    client.info("make sure exits command works");
    client.send("exits");
    client.recv_contains("south");
    client.recv_prompt();

    client.info("go south, back to room 1");
    client.send("south");
    client.recv_contains("An empty room");
    client.recv_prompt();

    client.info("check the room again, should be back in room 1 with an exit north");
    client.send("room info");
    client.recv_contains_all(vec!["Room 1", "players:", "krixi", "north"]);
    client.recv_prompt();
    client.send("exits");
    client.recv_contains("north");
    client.recv_prompt();
}

// TODO:
async fn test_room_name() {}
async fn test_room_desc() {}
async fn test_room_link_and_unlink() {}
async fn test_room_region() {}
async fn test_room_remove() {}
