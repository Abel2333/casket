# Casket

Casket is a terminal-first personal knowledge and secrets vault written in Rust.

The project is still in early implementation. The current codebase focuses on the storage and vault foundation rather than the TUI.

## Current Status

Implemented:

- domain models for entries, tags, and secret fields
- SQLite schema initialization
- `VaultMetadataRepository` backed by SQLite
- vault lifecycle service for:
  - checking initialization
  - first-time initialization
  - unlock by re-deriving the master key
- Argon2-based key derivation driven by stored vault metadata
- XChaCha20Poly1305-based encryption and decryption
- `EntryService` support for:
  - draft creation
  - draft update
  - detail loading with secret-field decryption
- unit tests for repository, key derivation, and vault service

Not implemented yet:

- SQLite implementations for:
  - `EntryRepository`
  - `TagRepository`
  - `SecretRepository`
- entry list loading through real repositories
- app state and TUI screens
- full application wiring in `main.rs`

## Project Structure

- [src/domain](src/domain): core data types
- [src/storage](src/storage): repository traits
- [src/infra/sqlite](src/infra/sqlite): SQLite schema and repository implementations
- [src/infra/crypto](src/infra/crypto): vault metadata and key derivation
- [src/service](src/service): service-layer orchestration

## Vault Model

The vault is the encryption context for sensitive data.

- `VaultMetadata` stores the salt and KDF configuration needed to derive a `MasterKey`
- the master password is never stored
- `VaultService::initialize()` creates metadata and returns a derived key
- `VaultService::unlock()` reads metadata and re-derives the key

Current limitation:

- `unlock()` re-derives a key, but does not yet prove the password is correct by itself
- wrong-password detection will either happen during decryption or through a future key-check mechanism

## Development

Requirements:

- Rust toolchain
- SQLite support through `sqlx`

Run tests:

```bash
cargo test
```

Run the current binary:

```bash
cargo run
```

At the moment the binary only contains placeholder startup code in [main.rs](/src/main.rs).

## Next Steps

- implement SQLite repositories for entries, tags, and secret fields
- add real repository validation without the TUI
- add entry list loading through `EntryService`
- handle vault locking in `AppState` by dropping the in-memory key
- introduce app state and the first TUI screens
