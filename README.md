# Rust Poker

A multiplayer Texas Hold'em poker game with a graphical client and a TCP game server.

## Background

This project started as an unfinished school group project. I revived and completed it independently (fixing the networking and gameplay issues that were left broken, and simplifying the project's scope to land on something reliable).

## Features

- **Live multiplayer Texas Hold'em** — join a room, wait for players, and play a full hand.
- **Persistent accounts** — register and log in against a local database.
- **Graphical client** — built with [macroquad](https://github.com/not-fl3/macroquad); login screen, main menu, waiting room, and a live table view with cards, chip counts, pot, and turn indicators.
- **Resilient networking** — a single background thread owns all reads from the server per client and routes messages by shape.

## Architecture

The project is split into two halves that talk over a single JSON protocol on a TCP socket:

- **`client/`** — the macroquad GUI app. Handles login/registration, room joining, rendering the table, and capturing player actions.
- **`server/`** — a multithreaded TCP server (one thread per connection). Manages game rooms, player accounts, and runs the Texas Hold'em game logic.

Each message is a single line of JSON terminated by `\n`. The server frames incoming bytes itself, and the client's connection manager does the same so partial reads are handled correctly.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)

### Running the server

```bash
cd server
cargo run
```

The server listens on `127.0.0.1:8080` by default.

### Running the client

In a separate terminal:

```bash
cd client
cargo run
```


- Currently supports Texas Hold'em only.
- The server keeps a single fixed room per game variant rather than supporting multiple concurrent tables.
- No reconnect/resume support — a disconnected player is folded out of the current hand rather than able to rejoin it.

## Acknowledgments

Originally started as a group project for a university course. All networking and gameplay logic in its current, working form was independently rewritten and debugged after the fact.
