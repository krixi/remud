use crate::support::{Matcher, Server};

#[tokio::test]
async fn test_communicate_emote() {
    let (mut server, mut t) = Server::new_create_player("krixi", "(*&%(*#&%*&").await;
    let mut t2 = server.create_player("Shane", "lkja;jf89 f").await;

    t.consume_prompt().await;
    t.line_contains("Shane arrives.").await;
    t.assert_prompt().await;

    t.test(
        "krixi emotes with normal command",
        "me dances around all silly.",
        vec!["krixi dances around all silly."],
    )
    .await;

    t2.consume_prompt().await;
    t2.line_contains("krixi dances around all silly.").await;
    t2.assert_prompt().await;

    t2.test(
        "shane also emotes with shortcut command",
        "/does the sprinkler.",
        vec!["Shane does the sprinkler."],
    )
    .await;

    t.consume_prompt().await;
    t.line_contains("Shane does the sprinkler.").await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_communicate_say() {
    let (mut server, mut t) = Server::new_create_player("krixi", "(*&%(*#&%*&").await;
    let mut t2 = server.create_player("Shane", "lkja;jf89 f").await;

    t.consume_prompt().await;
    t.line_contains("Shane arrives.").await;
    t.assert_prompt().await;

    t.test(
        "krixi says hello with normal command",
        "say Hello, Shane.",
        vec![r#"You say "Hello, Shane.""#],
    )
    .await;

    t2.consume_prompt().await;
    t2.line_contains(r#"krixi says "Hello, Shane.""#).await;
    t2.assert_prompt().await;

    t2.test(
        "shane also says hello with shortcut command",
        "'Hi hi.",
        vec![r#"You say "Hi hi."#],
    )
    .await;

    t.consume_prompt().await;
    t.line_contains(r#"Shane says "Hi hi.""#).await;
    t.assert_prompt().await;
}

#[tokio::test]
async fn test_communicate_send() {
    let (mut server, mut t) = Server::new_create_player("krixi", "(*&%(*#&%*&").await;
    let mut t2 = server.create_player("Shane", "lkja;jf89 f").await;

    t.consume_prompt().await;
    t.line_contains("Shane arrives.").await;
    t.assert_prompt().await;

    t.test(
        "send Shane a message",
        r#"send Shane Hi Shane!"#,
        vec!["Your term chirps happily"],
    )
    .await;

    t2.consume_prompt().await;
    t2.line_contains(r#"krixi sends "Hi Shane!""#).await;
    t2.assert_prompt().await;
}

#[tokio::test]
async fn test_communicate_who() {
    let (mut server, mut t) = Server::new_create_player("krixi", "(*&%(*#&%*&").await;

    t.test_matches(
        "who with just krixi",
        "who",
        Matcher::exact_includes(vec!["Online players:", "krixi"]),
    )
    .await;

    let t2 = server.create_player("Shane", "lkja;jf89 f").await;

    t.consume_prompt().await;
    t.line_contains("Shane arrives.").await;
    t.assert_prompt().await;

    t.test_matches(
        "who with krixi and Shane",
        "who",
        Matcher::exact_includes(vec!["Online players:", "Shane", "krixi"]),
    )
    .await;

    drop(t2);

    t.consume_prompt().await;
    t.line_contains("Shane leaves.").await;
    t.assert_prompt().await;

    t.test_matches(
        "who with just krixi again",
        "who",
        Matcher::exact_includes(vec!["Online players:", "krixi"]),
    )
    .await;
}
