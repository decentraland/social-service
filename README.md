<p align="center">
  <a href="https://decentraland.org">
    <img alt="Decentraland" src="https://decentraland.org/images/logo.png" width="60" />
  </a>
</p>
<h1 align="center">
  Decentraland Social Service
</h1>

The social service is an enabler of social interactions for decentraland, it stores the friendships between users, has the logic for managing friend requests, and has logic on top of the chat to make sure it's being used correctly (sets a max amount of friends or channels a user can have).

## Collaboration

### Setting up Rust

#### Rust Installation

The preferred way to install Rust [is by using the rustup command](https://www.rust-lang.org/tools/install):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

This will by default install the stable toolchain, but will also enable you to install beta and nightly toolchains as well as new platforms (e.g. WASM) and update all toolchains to their latest versions.

#### Rust update

Run

```bash
rustup update
```

#### Editor & Tooling

There are two recommended editors/IDEs at the moment:

- [IntelliJ Rust](https://www.jetbrains.com/rust/)
- VS Code with [Rust Analyzer](https://www.google.com/search?q=rust+analyzer&oq=rust+analyzer&aqs=chrome..69i57j0i512l9.2107j0j7&sourceid=chrome&ie=UTF-8#:~:text=rust%2Dlang/rust,lang%20%E2%80%BA%20rust%2Danalyzer) support

#### Debug Rust in VS Code

- Follow the instructions [in the following post](https://www.forrestthewoods.com/blog/how-to-debug-rust-with-visual-studio-code/) to add the extension for VS Code for debugging LLVM programs in VS Code

### Building the server

This project will run an HTTP Server and a WebSocket Server.

The WebSocket server implements the protocol definition defined in https://github.com/decentraland/protocol/blob/main/proto/decentraland/social/friendships/friendships.proto which is automatically downloaded from GitHub during the build time. If a build fails, it could be related to that.

### Requirements

You need to have protoc installed

```
brew install protobuf
```

### Running the server

```
make run
```

Running this command will run a dockerized Postgres DB and run the server on port `8080`. You should have Docker installed on your computer and running.

For development, you can use this command:

```
make dev
```

Running this command will run a dockerized Postgres DB and run the server on port `8080` but in watch mode, so every change you make will be watched and the server will restart.

### Running Tests locally

```bash
make test
```

### Database & Migrations

Migrations or pending migrations run when the server starts programmatically using the [sqlx](https://github.com/launchbadge/sqlx) API.

In order to create a new migration, you have to run:

```
make migration name={YOUR_MIGRATION_NAME}
```

This command will create the migration SQL files (up and down) with the given name

#### Enter in the DB

```
docker exec -ti social_service_db psql -U postgres -d social_service
```

### Configuration

There's a configuration file (`configuration.toml`) that allows configuring the following variables but you can ignore this file, if you use the above `make` commands:

```
host: Host address where the server will run
port: Port where the server will be exposed
```
