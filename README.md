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

### Docker Images

Public ReMUD ECR repository: `public.ecr.aws/s1x5o0q1/remud`

This project can be built and deployed with Docker (be warned, the build step takes at least 5
minutes each time). Note that the `remud-build` tag is important, it's referenced from the
runtime docker file.

```shell
docker build -f Dockerfile.build -t remud-build .
docker build -t remud .

```

Publishing a new image to ECR:

```
aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws
docker tag remud:latest public.ecr.aws/s1x5o0q1/remud
docker push public.ecr.aws/s1x5o0q1/remud:latest
```

### Infrastructure

(Optional) Set up an [S3 backend](https://www.terraform.io/docs/language/settings/backends/s3.html) for Terraform state.

Set up a .tfvars file based on `variables.tf`.

To configure AWS resources with Terraform:

```shell
cd terraform
terraform init
terraform plan -var-file <vars_file>.tfvars
terraform apply -var-file <vars_file>.tfvars
```

#### Initial EBS setup

The EBS provided for game data storage is unformatted and unmounted to start. To configure it in an EC2 instance:

```
sudo mkfs -t xfs /dev/xvdg
sudo mkdir /game
sudo mount /dev/sdg /game
```

### Running ReMUD in Docker

This section assumes ReMUD will be run on an EC2 instance provisioned by the above Terraform commands. If not, adjust the volumes appropriately.

To pull and run ReMUD:

```
docker pull public.ecr.aws/s1x5o0q1/remud:latest

# when running without TLS for the API
docker run -it --rm \
  -v /game/world:/game/world \
  -v /game/keys:/game/keys \
  -p 23:2004 -p 80:2080 \
  --name citysix \
  --entrypoint ./remud \
  public.ecr.aws/s1x5o0q1/remud:latest \
    --db /game/world/world.db \
    --keys /game/keys \
    --cors <cors_host>,...

# when provisioning a new TLS certificate
docker run -it --rm \
  -v /game/world:/game/world \
  -v /game/keys:/game/keys \
  -p 23:2004 -p 443:2080 -p 80:80 \
  --name citysix \
  --entrypoint ./remud \
  public.ecr.aws/s1x5o0q1/remud:latest \
    --db /game/world/world.db \
    --keys /game/keys \
    --cors <cors_host>,... \
    --tls <domain> \
    --email <contact_email>

# when using an already provisioned TLS certificate
docker run -it --rm \
  -v /game/world:/game/world \
  -v /game/keys:/game/keys \
  -p 23:2004 -p 443:2080 \
  --name citysix \
  --entrypoint ./remud \
  public.ecr.aws/s1x5o0q1/remud:latest \
    --db /game/world/world.db \
    --keys /game/keys \
    --cors <cors_host>,... \
    --tls <domain> \
    --email <contact_email>
```
