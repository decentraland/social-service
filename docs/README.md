### Friendship Lifecycle

```mermaid
stateDiagram
    [*] --> REQUEST
    REQUEST --> CANCEL
    REQUEST --> ACCEPT
    REQUEST --> REJECT
    ACCEPT --> DELETE
    CANCEL --> REQUEST
    REJECT --> REQUEST
    DELETE --> REQUEST
```
