# remud

Remud is a new MUD server written in Rust. My intention is to begin experimenting with the MUD format once the basics are nailed down, and to build an interesting cyberpunk-themed world. Only the server itself will be open source.
 
While I am open to pull requests they are not a priority for this project and I can't promise they will be merged. I do appreciate architecture and Rust language tips, as I am always looking to improve.
 
I occasionally stream development of this project on [Twitch](https://www.twitch.tv/binchomittens). Feel free to drop by and give me a follow.
 
Technologies:
 - [Rust](https://www.rust-lang.org/)
 - [Tokio](https://tokio.rs/)
 - [sqlx](https://github.com/launchbadge/sqlx) (currently backed by [SQLite](https://www.sqlite.org/))
 - [bevy_ecs](https://bevyengine.org/)


Docs for this project are built using [Hugo](https://gohugo.io/) - to view them locally, install Hugo and run:
```shell
cd docs
hugo serve -D
```
