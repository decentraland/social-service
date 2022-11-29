# Social service

The social service is an enabler of social interactions for decentraland, it stores the friendships between users, has the logic for managing friend requests, and has logic on top of the chat to make sure it's being used correctly (sets a max amount of friends or channels a user can have).

## Collaboration

### Running the server

```
make run
```

Running this command will run a dockerized Postgres DB and run the server on port `8080`. You should have Docker installed on your computer.

For development, you can use this command:

```
make dev
```

Running this command will run a dockerized Postgres DB and run the server on port `8080` but in watch mode, so every change you make will be watched and the server will restart.

### Database & Migrations

Migrations or pending migrations run when the server starts up programatically with the [sea-orm](https://github.com/SeaQL/sea-orm) API.

In order to create a new migration, you have to run:

```
make migration name={YOUR_MIGRATION_NAME}
```

The above command will create a new migration file with a specific name and basic code, you should define your schema programatically based on [sea-orm](https://github.com/SeaQL/sea-orm)

After your migrations run, you may want to run the following command:

```
make models
```

This command will create the entities (or models) so that you can make queries and interact with the DB via these models easily.

### Configuration

There's a configuration file (`configuration.toml`) that allows configuring the following variables but you can ignore this file, if you use the above `make` commands:

```
host: Host address where the server will run
port: Port where the server will be exposed
```
