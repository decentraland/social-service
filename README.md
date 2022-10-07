# Social service

The social service is an enabler of social interactions for decentraland, it stores the friendships between users, has the logic for managing friend requests, and has logic on top of the chat to make sure it's being used correctly (sets a max amount of friends or channels a user can have).

## Collaboration
### Configuration
There's a configuration file (`collaboration.toml`) that allows configuring the following variables:
```
host: Host address where the server will run
port: Port where the server will be exposed
```

### Running the server

```
cargo run
```
Running this command will run the server on the port specified in the configuration file

