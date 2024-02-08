# Mini-Redis

- Inspired by [code-crafters](https://app.codecrafters.io/courses/redis/overview) and [mini-redis](https://github.com/tokio-rs/mini-redis)

- This is redis server implementation for a subset of it's functionality.

# Supported Commands

- PING
- GET
- SET - With expiry in secs too😊

Redis serialization protocol([RESP](https://redis.io/docs/reference/protocol-spec/)) has been used for communication with clients.

Persistence is available through [RDB](https://redis.io/docs/management/persistence/) following [this](https://rdb.fnordig.de/file_format.html#length-encoding) specification.

## Tech stack

- Rust
- Async Rust powered by Tokio

## Architecture

### TCP Server and client

the server and client bins use tcp and RESP to communicate with each other.
TCP functionality is provided by tokio.
Server gracefully handles runtime errors.

### DATABASE

A hashmap is used as the KV database.
key - string representing the key.
value - a struct with two fields -> value represented in bytes to minimize serialization and deserialization, and expires_at a u32 representing when the KV should be evicted.
A u32 is used as it is the range that Redis uses for their ttls.
It uses a background worker to evict expired KV.
It uses a BTreeSet to sort the ttls for deletion by the background worker.
The database uses a mutex to prevent race conditions across threads and Arc pointer for safe sharing across threads.

### RDB

This crate/lib handles flushing and loading the db to and fro an rdb file.
It uses a background worker to do the job.

### Connection - crate

This crate handles reading and writing into the tcp stream following RESP and is reusable for both server and client.

### Frame - crate

This is responsible for reading meaningful data/ frames from raw bytes sent from a tcp stream.
Example: get key is read into

```
Frame::Array([Frame::String("get"), Frame::string("key")])
```

It works in conjuction with connection lib.

### Runner - crate

This crate is responsible of executing successful frames to get responses.

## Install AND RUN

```
git clone https://github.com/kariuki-george/mini-redis
cd mini-redis
cp .env.example .env
cargo run --bin server
cargo run --bin client
```

Note: More client examples coming soon⌛

## Timeline

1. BIND TO A PORT ✅
2. SETUP A SERVER ✅
3. SETUP A CLIENT ✅
4. RESPOND TO A COMMAND -> PING -> ACCORDING TO REDIS SPEC -> https://redis.io/docs/reference/protocol-spec/ ✅
5. READ MULTIPLE STRING IN THE SAME REQUEST -> SET KEY VALUE ✅
6. SERVER TO HANDLE CONCURRENT CLIENTS ✅
7. IMPLEMENT SET, GET ✅
8. ADD DATA PERSISTENCE -> USE RDB ✅ OR AOF -> https://redis.io/docs/management/persistence/ 
9. SUPPORT CONFIG GET FOR THE RDB STUFF ⚒️
10. EXPIRY -> TTL ✅
