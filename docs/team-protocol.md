# Team Messaging Protocol

This protocol is mandatory for all ATM team communications used with this repo.

## Required Flow

1. Immediately acknowledge every ATM message received.
2. Execute the requested task.
3. Send a completion message with a concise summary of what was done.
4. Receiver immediately acknowledges completion.
5. No silent processing.

## Good Pattern

- `ack, working on <task>`
- `task complete: <summary>`
- `received. QA starting now.`
