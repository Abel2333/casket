# Casket Architecture and Development Plan

## 1. Project Scope

The first version of `casket` should not aim to be a full general-purpose note application. It should be a:

- local-first personal knowledge store
- terminal-first TUI application
- note system for both regular content and sensitive data
- SQLite-backed application first, with PostgreSQL support added later

The intended content includes:

- journal entries
- general notes
- bookmarked pages
- API keys, tokens, cookies, credentials, and similar sensitive material

Core principles:

1. Separate regular data from sensitive data
2. Perform encryption at the application layer, not the database layer
3. Complete the SQLite implementation before adding PostgreSQL
4. Keep UI, business logic, and persistence separated

## 2. First-Phase Goals

The first phase should only target an MVP with a complete core loop.

### 2.1 MVP Features

- start the application
- enter a master password to unlock sensitive content
- create a regular note
- create an entry with encrypted fields
- browse an entry list
- view entry details
- edit entries
- persist data with SQLite

### 2.2 Out of Scope for Phase One

- cloud sync
- multi-user support
- file attachments
- rich text editing
- full-text search engine
- PostgreSQL implementation
- import/export tools

These can be added in later phases.

## 3. Recommended Directory Layout

```text
src/
  main.rs
  app/
    mod.rs
    state.rs
    action.rs
    event.rs
  ui/
    mod.rs
    root.rs
    screens/
      list.rs
      detail.rs
      editor.rs
      unlock.rs
    widgets/
  domain/
    mod.rs
    entry.rs
    tag.rs
    secret.rs
  service/
    mod.rs
    entry_service.rs
    vault_service.rs
  storage/
    mod.rs
    traits.rs
  infra/
    mod.rs
    config.rs
    crypto/
      mod.rs
    sqlite/
      mod.rs
      schema.rs
      entry_repo.rs
      secret_repo.rs
      tag_repo.rs
docs/
  architecture.md
```

## 4. Layer Responsibilities

### 4.1 `domain`

Contains the core domain models and business concepts.

This layer should not depend on:

- `ratatui`
- `crossterm`
- SQLite
- PostgreSQL

It should define concepts such as:

- `Entry`
- `EntryKind`
- `Tag`
- `SecretField`

### 4.2 `app`

Contains application state and interaction flow. This is the central state layer of the program.

It is responsible for:

- current screen
- current focus
- selected entry
- lock state
- active draft
- state transitions driven by actions

### 4.3 `ui`

Only handles rendering and input mapping.

Responsibilities:

- render `AppState`
- translate keyboard input into `Action`

It should not:

- query the database directly
- perform encryption directly
- implement business logic directly

### 4.4 `service`

Contains use cases and orchestration logic.

Examples:

- create entry
- save entry
- load entry details
- save encrypted fields
- unlock the vault

### 4.5 `storage`

Contains persistence interfaces, expressed as traits.

Examples:

- `EntryRepository`
- `SecretRepository`
- `TagRepository`

### 4.6 `infra`

Contains infrastructure implementations.

Examples:

- SQLite repository implementations
- future PostgreSQL repository implementations
- crypto implementation
- config loading

## 5. Domain Model Recommendations

### 5.1 `Entry`

Use one unified entry model early on instead of splitting by feature into many tables.

```rust
pub struct Entry {
    pub id: EntryId,
    pub kind: EntryKind,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_favorite: bool,
    pub is_archived: bool,
}
```

### 5.2 `EntryKind`

```rust
pub enum EntryKind {
    Journal,
    Note,
    Bookmark,
    Credential,
}
```

Keep the first version limited to a small stable set of types.

### 5.3 `Tag`

```rust
pub struct Tag {
    pub id: TagId,
    pub name: String,
}
```

### 5.4 `SecretField`

Sensitive fields should not be embedded directly in `body`. Store them separately.

```rust
pub struct SecretField {
    pub id: SecretFieldId,
    pub entry_id: EntryId,
    pub name: String,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_version: i32,
}
```

Common first-version field names:

- `token`
- `password`
- `cookie`
- `api_key`
- `client_secret`

### 5.5 Draft Model

The application state should manage an editing draft separately instead of mutating database entities directly.

```rust
pub struct EntryDraft {
    pub id: Option<EntryId>,
    pub kind: EntryKind,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub secret_fields: Vec<SecretDraftField>,
}
```

This makes it easier to support:

- creating new entries
- canceling edits
- unsaved changes prompts

## 6. Persistence Design

Phase one should only implement SQLite, but the schema and repository design should leave room for PostgreSQL later.

### 6.1 SQLite Tables

#### `entries`

```sql
CREATE TABLE entries (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  is_favorite INTEGER NOT NULL DEFAULT 0,
  is_archived INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

#### `tags`

```sql
CREATE TABLE tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);
```

#### `entry_tags`

```sql
CREATE TABLE entry_tags (
  entry_id TEXT NOT NULL,
  tag_id TEXT NOT NULL,
  PRIMARY KEY (entry_id, tag_id),
  FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE,
  FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);
```

#### `secret_fields`

```sql
CREATE TABLE secret_fields (
  id TEXT PRIMARY KEY,
  entry_id TEXT NOT NULL,
  field_name TEXT NOT NULL,
  ciphertext BLOB NOT NULL,
  nonce BLOB NOT NULL,
  key_version INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
);
```

#### `vault_metadata`

Used to store key derivation parameters, not the plaintext password.

```sql
CREATE TABLE vault_metadata (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  salt BLOB NOT NULL,
  key_version INTEGER NOT NULL,
  kdf_algorithm TEXT NOT NULL,
  kdf_params_json TEXT NOT NULL
);
```

### 6.2 Why Sensitive Data Lives in a Separate Table

Benefits:

- regular content can be listed in plaintext
- regular notes can be filtered and searched
- most UI flows stay simple
- display policy can be controlled per field
- the design remains portable to PostgreSQL

### 6.3 Full-Text Search

Do not implement database-backed full-text search in phase one.

Start with:

- title filtering
- kind filtering
- tag filtering
- updated-at sorting

Later options:

- SQLite FTS5
- PostgreSQL `tsvector`

## 7. Encryption Design

This is the part that needs to be correct from the beginning.

### 7.1 Goals

- the master password is never stored
- sensitive fields are encrypted before persistence
- the application starts locked
- unlocking allows sensitive fields to be read

### 7.2 Recommended Approach

- KDF: `Argon2id`
- AEAD: `XChaCha20Poly1305`
- random salt when initializing the vault
- a fresh nonce for each encrypted field

### 7.3 Flow

First startup:

1. the user sets a master password
2. generate a random `salt`
3. derive a key from the password with `Argon2id`
4. write `vault_metadata`
5. enter the unlocked state

Later startups:

1. read `vault_metadata`
2. prompt for the master password
3. derive the key using the same parameters
4. use that key to decrypt sensitive fields

### 7.4 Suggested Module Interface

```rust
pub trait KeyDeriver {
    fn derive_key(&self, password: &str, salt: &[u8]) -> Result<MasterKey>;
}

pub trait Encryptor {
    fn encrypt(&self, plaintext: &[u8], key: &MasterKey) -> Result<EncryptedBlob>;
    fn decrypt(&self, blob: &EncryptedBlob, key: &MasterKey) -> Result<Vec<u8>>;
}
```

### 7.5 Important Constraints

- do not invent a custom cryptographic scheme
- do not reuse nonces
- do not persist the plaintext master password
- use `zeroize` for in-memory key material if possible

## 8. Storage Interface Design

The traits should use business language, not SQL language.

### 8.1 `EntryRepository`

```rust
pub trait EntryRepository {
    fn create(&self, entry: &Entry) -> Result<()>;
    fn update(&self, entry: &Entry) -> Result<()>;
    fn get(&self, id: EntryId) -> Result<Option<Entry>>;
    fn list(&self, filter: EntryFilter) -> Result<Vec<Entry>>;
    fn delete(&self, id: EntryId) -> Result<()>;
}
```

### 8.2 `SecretRepository`

```rust
pub trait SecretRepository {
    fn replace_for_entry(&self, entry_id: EntryId, fields: &[SecretField]) -> Result<()>;
    fn list_for_entry(&self, entry_id: EntryId) -> Result<Vec<SecretField>>;
}
```

### 8.3 `TagRepository`

```rust
pub trait TagRepository {
    fn attach_tags(&self, entry_id: EntryId, tag_names: &[String]) -> Result<()>;
    fn list_tags_for_entry(&self, entry_id: EntryId) -> Result<Vec<Tag>>;
    fn list_all_tags(&self) -> Result<Vec<Tag>>;
}
```

## 9. Application State Design

In `ratatui` projects, UI state and business state often get mixed together too early.

Define `AppState` clearly from the start.

### 9.1 Screen Enum

```rust
pub enum Screen {
    Unlock,
    List,
    Detail,
    Editor,
}
```

### 9.2 Vault State

```rust
pub enum VaultState {
    Locked,
    Unlocked,
}
```

### 9.3 Core State

```rust
pub struct AppState {
    pub screen: Screen,
    pub vault_state: VaultState,
    pub list: EntryListState,
    pub selected_entry_id: Option<EntryId>,
    pub current_entry: Option<EntryDetailView>,
    pub editor: Option<EntryDraft>,
    pub status_message: Option<String>,
    pub should_quit: bool,
}
```

### 9.4 State That Should Be Split Out

- `EntryListState`
- `EditorState`
- `UnlockState`

Do not place every field directly on `AppState`.

## 10. Event and Action Flow

Recommended flow:

```text
input event -> Action -> AppState/Service -> state change -> redraw
```

### 10.1 `Event`

Represents low-level input such as:

- keyboard input
- tick
- resize

### 10.2 `Action`

Represents application-level intent such as:

- `Quit`
- `OpenSelectedEntry`
- `StartCreateEntry`
- `StartEditEntry`
- `SaveDraft`
- `DeleteSelectedEntry`
- `UnlockVault`
- `BackToList`

### 10.3 Why Split Them

Benefits:

- cleaner input mapping
- easier tests
- screen transitions and business logic stay out of render code

## 11. Screen Recommendations

### 11.1 `UnlockScreen`

Purpose:

- prompt for the master password
- set the master password on first startup

First version requirements:

- password input
- status message
- submit on Enter

### 11.2 `ListScreen`

Suggested layout:

- left: kinds, tags, or filters
- center: entry list
- bottom: key hints

Do not over-design the first version.

### 11.3 `DetailScreen`

Display:

- title
- kind
- updated time
- body
- tags
- sensitive field names

Only display sensitive values when unlocked.

### 11.4 `EditorScreen`

Editable content:

- title
- kind
- body
- tags
- secret fields

The first version does not need sophisticated form widgets.

## 12. Config Design

Keep config centralized in `infra/config.rs`.

The first version can support:

- data directory location
- database backend type
- SQLite file path

For example:

```rust
pub enum DatabaseBackend {
    Sqlite,
    Postgres,
}

pub struct AppConfig {
    pub data_dir: PathBuf,
    pub backend: DatabaseBackend,
    pub sqlite_path: PathBuf,
}
```

Notes:

- define the backend enum even if only SQLite is implemented initially
- do not implement PostgreSQL behavior yet

## 13. How to Leave Room for PostgreSQL

Do not implement full support in phase one. Only reserve the shape.

### 13.1 What to Reserve

- stable `storage` traits
- an `infra/postgres/` place in the layout
- config support for backend selection

### 13.2 What Not to Do Yet

- do not write and maintain two SQL implementations now
- do not make the SQLite design more complex just for PostgreSQL
- do not introduce over-abstract generic repositories

The correct order is:

1. define traits
2. complete the SQLite implementation
3. add PostgreSQL after the business model stabilizes

## 14. Recommended Development Order

This order is worth following strictly.

### Step 1: Create the Base Modules

Create the directory layout first, even if many modules are still mostly empty.

Done when:

- `app`
- `ui`
- `domain`
- `service`
- `storage`
- `infra`

exist and compile

### Step 2: Define Domain Models

Start with:

- `Entry`
- `EntryKind`
- `Tag`
- `SecretField`
- `EntryDraft`

Done when:

- they do not depend on the database
- they do not depend on the TUI
- they compile cleanly

### Step 3: Design the SQLite Schema

Lock down the table definitions.

Done when:

- `entries`
- `tags`
- `entry_tags`
- `secret_fields`
- `vault_metadata`

are defined and initialized

### Step 4: Define Storage Traits

Define persistence capabilities before implementing PostgreSQL.

Done when:

- `EntryRepository`
- `SecretRepository`
- `TagRepository`

are stable enough to build on

### Step 5: Implement SQLite Repositories

Get the core reads and writes working.

Done when:

- create entry
- update entry
- list entries
- load by id
- replace secret fields
- bind tags

all work correctly

### Step 6: Implement the Crypto Module

Done when:

- key derivation works
- encryption works
- decryption works
- first-time vault initialization works
- vault unlock works

### Step 7: Implement Services

Start with these use cases:

- create a regular entry
- create an entry with encrypted fields
- load entry detail
- save a draft

### Step 8: Implement `AppState` and `Action`

Done when:

- screen transitions work
- selection state works
- draft state works
- lock state works

### Step 9: Implement the Minimal TUI

Build:

- unlock screen
- list screen
- detail screen
- editor screen

Do not optimize visuals too early.

### Step 10: Wire Up `main.rs`

`main.rs` should only handle:

- config initialization
- database initialization
- service initialization
- TUI startup
- event loop startup

## 15. Suggested First-Version Keybindings

Keep the first keymap simple and stable:

- `q`: quit
- `j` / `k`: move selection
- `Enter`: open details
- `n`: create entry
- `e`: edit current entry
- `d`: delete current entry
- `Esc`: go back
- `s`: save
- `/`: search
- `l`: lock the vault

Do not introduce too many modes in the first version.

## 16. Suggested Dependencies

Potential additions:

- `anyhow`
- `thiserror`
- `chrono`
- `uuid`
- `serde`
- `serde_json`
- `ratatui`
- `crossterm`
- `sqlx` or `rusqlite`
- `argon2`
- `chacha20poly1305`
- `rand`
- `zeroize`
- `directories`

### `sqlx` vs `rusqlite`

If PostgreSQL support is a real future requirement, prefer `sqlx`.

Reasons:

- the PostgreSQL migration path is smoother
- repository implementation style stays more consistent

If you want the lowest possible initial complexity, `rusqlite` is acceptable, but the later PostgreSQL refactor will be larger.

## 17. Suggested Milestones

### Milestone A: Base Skeleton

Goals:

- directory structure complete
- domain models complete
- storage traits complete

### Milestone B: SQLite Integration

Goals:

- schema complete
- SQLite repositories complete
- command-line insertion and retrieval works

### Milestone C: Encryption Loop

Goals:

- initialize vault
- unlock vault
- encrypt and decrypt sensitive fields

### Milestone D: TUI MVP

Goals:

- unlock screen
- list screen
- detail screen
- editor screen

### Milestone E: UX Improvements

Goals:

- tag filtering
- search
- favorites
- recently updated sorting

## 18. Best Place to Start Now

If you want the most stable starting point, implement these next:

1. entities and enums under `domain`
2. `storage/traits.rs`
3. `infra/sqlite/schema.rs`

Reasons:

- these define the boundaries of the entire project
- once these are stable, service and UI work gets much easier
- if this layer changes late, the TUI will need more rework

## 19. Good Follow-Up Docs

Later, it would make sense to add:

- `docs/schema.md`
- `docs/crypto.md`
- `docs/ui-flow.md`
- `docs/roadmap.md`

If you keep only one architecture document, this one is already enough to support MVP development.
