# ReMUd

Remud is a new MUD server written in Rust. My intention is to begin experimenting with the MUD format once the basics are nailed down, and to build an interesting cyberpunk-themed world. Only the server itself will be open source.

While I am open to pull requests they are not a priority for this project and I can't promise they will be merged. I do appreciate architecture and Rust language tips, as I am always looking to improve.

I occasionally stream development of this project on [Twitch](https://www.twitch.tv/binchomittens). Feel free to drop by and give me a follow.

Technologies:

- [Rust](https://www.rust-lang.org/)
- [Tokio](https://tokio.rs/)
- [sqlx](https://github.com/launchbadge/sqlx) (currently backed by [SQLite](https://www.sqlite.org/))
- [bevy_ecs](https://bevyengine.org/)
- [rhai](https://rhai.rs/)
- [warp](https://github.com/seanmonstar/warp)

Docs for this project are built using [Hugo](https://gohugo.io/) and are available on [GitHub Pages](https://siler.github.io/remud). To view them locally, install Hugo and run:

```shell
cd docs
npm install
hugo serve -D
```

The web client for this project allows for management of the in-game scripts. To run it:

```shell
cd web-client
npm install
npm start
```


To build and run this project with docker (be warned, the build step takes at least 5 minutes each time). 
Note that the `remud-build` tag is important, it's referenced from the runtime docker file.

```shell
# to build it:
docker build -f Dockerfile.build -t remud-build .
docker build -t remud .
# to run it:
docker run -it --rm -v $PWD:/game/world/ -p 2004:2004 -p 2080:2080 --name citysix remud
# to stop it:
docker stop citysix
```
