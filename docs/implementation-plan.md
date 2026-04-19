# Casket Implementation Plan

This document complements `docs/architecture.md`.

If the architecture doc answers "why this design", this document answers:

- which files to write first
- what each file should contain initially
- what counts as done at each step
- what the first version of the interfaces and state can look like

The goal is to let you implement in a steady sequence instead of making repeated structural decisions while coding.

## 1. Recommended Implementation Order

Strongly recommended order:

1. create modules and empty files
2. define domain models
3. define storage traits
4. implement SQLite schema and initialization
5. implement SQLite repositories
6. implement the crypto module
7. implement services
8. implement `AppState` and `Action`
9. implement TUI screens
10. wire up `main.rs`

Do not start with complex UI.
Do not start with PostgreSQL.
Do not start with search.

## 2. File-by-File Guidance

## 2.1 `src/domain/entry.rs`

Start this file with:

- `EntryId`
- `EntryKind`
- `Entry`
- `EntryDraft`
- `EntryFilter`

Suggested sketch:

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub type EntryId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Journal,
    Note,
    Bookmark,
    Credential,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: EntryId,
    pub kind: EntryKind,
    pub title: String,
    pub body: String,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EntryDraft {
    pub id: Option<EntryId>,
    pub kind: EntryKind,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub secret_fields: Vec<SecretDraftField>,
}

#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    pub kind: Option<EntryKind>,
    pub include_archived: bool,
    pub search_term: Option<String>,
    pub tag: Option<String>,
}
```

The main point here:

- `Entry` is persisted data
- `EntryDraft` is editable state
- do not treat them as the same object

## 2.2 `src/domain/secret.rs`

Start with:

- `SecretFieldId`
- `SecretField`
- `SecretDraftField`
- `EncryptedBlob`

Suggested sketch:

```rust
use uuid::Uuid;

use crate::domain::entry::EntryId;

pub type SecretFieldId = Uuid;

#[derive(Debug, Clone)]
pub struct SecretField {
    pub id: SecretFieldId,
    pub entry_id: EntryId,
    pub name: String,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_version: i32,
}

#[derive(Debug, Clone)]
pub struct SecretDraftField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct EncryptedBlob {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}
```

## 2.3 `src/domain/tag.rs`

Keep this minimal at first:

```rust
use uuid::Uuid;

pub type TagId = Uuid;

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
}
```

## 2.4 `src/storage/traits.rs`

This file matters a lot. It determines how easy the later SQLite-to-PostgreSQL path will be.

Define:

- `EntryRepository`
- `SecretRepository`
- `TagRepository`
- `VaultMetadataRepository`

Suggested sketch:

```rust
use async_trait::async_trait;
use anyhow::Result;

use crate::domain::entry::{Entry, EntryFilter, EntryId};
use crate::domain::secret::SecretField;
use crate::domain::tag::Tag;
use crate::infra::crypto::VaultMetadata;

pub trait EntryRepository {
    fn create(&self, entry: &Entry) -> Result<()>;
    fn update(&self, entry: &Entry) -> Result<()>;
    fn get(&self, id: EntryId) -> Result<Option<Entry>>;
    fn list(&self, filter: &EntryFilter) -> Result<Vec<Entry>>;
    fn delete(&self, id: EntryId) -> Result<()>;
}

pub trait SecretRepository {
    fn replace_for_entry(&self, entry_id: EntryId, fields: &[SecretField]) -> Result<()>;
    fn list_for_entry(&self, entry_id: EntryId) -> Result<Vec<SecretField>>;
}

pub trait TagRepository {
    fn replace_for_entry(&self, entry_id: EntryId, tag_names: &[String]) -> Result<()>;
    fn list_for_entry(&self, entry_id: EntryId) -> Result<Vec<Tag>>;
    fn list_all(&self) -> Result<Vec<Tag>>;
}

#[async_trait]
pub trait VaultMetadataRepository {
    async fn get(&self) -> Result<Option<VaultMetadata>>;
    async fn save(&self, metadata: &VaultMetadata) -> Result<()>;
}
```

`sqlx` is async-first, so a SQLite-backed `VaultMetadataRepository` is simpler if you make it async immediately.
It is still acceptable to keep the entry, secret, and tag repositories synchronous for the first pass if they are not implemented yet.

## 2.5 `src/infra/sqlite/schema.rs`

This file should do only two things:

- hold schema SQL
- expose an `init_schema()` or `run_migrations()` entry point

Suggested form:

```rust
pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS entries (...);
CREATE TABLE IF NOT EXISTS tags (...);
CREATE TABLE IF NOT EXISTS entry_tags (...);
CREATE TABLE IF NOT EXISTS secret_fields (...);
CREATE TABLE IF NOT EXISTS vault_metadata (...);
"#;
```

Then execute it from a single initialization function.

For the first version, do not rush into a complex migration system. Stable schema initialization is enough.

## 2.6 `src/infra/crypto/mod.rs`

Start this module with these structures and traits:

```rust
pub struct MasterKey(pub [u8; 32]);

pub struct VaultMetadata {
    pub salt: Vec<u8>,
    pub key_version: i32,
    pub kdf_algorithm: String,
    pub kdf_params_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Argon2ParamsConfig {
    pub memory_cost: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

pub trait KeyDeriver {
    fn derive_key(&self, password: &str, metadata: &VaultMetadata) -> anyhow::Result<MasterKey>;
}

pub trait VaultMetadataFactory {
    fn new_metadata(&self) -> anyhow::Result<VaultMetadata>;
}

pub trait Encryptor {
    fn encrypt(
        &self,
        plaintext: &[u8],
        key: &MasterKey,
    ) -> anyhow::Result<EncryptedBlob>;

    fn decrypt(
        &self,
        blob: &EncryptedBlob,
        key: &MasterKey,
    ) -> anyhow::Result<Vec<u8>>;
}
```

Keep this module focused on the smallest complete encryption loop first. Do not start with key rotation.

Important boundary:

- `VaultMetadataRepository` reads and writes metadata
- `VaultService` decides when metadata is created or loaded
- `KeyDeriver` does not read the database by itself
- `KeyDeriver` should derive a key from the `VaultMetadata` it is given
- `VaultMetadataFactory` creates metadata for a newly initialized vault
- `VaultService` should not hard-code Argon2 defaults directly once the factory exists

In practice, `kdf_params_json` should be real serialized KDF parameters, not a placeholder string.

Recommended next refactor before moving deeper into encryption:

- introduce `VaultMetadataFactory`
- move salt generation and default Argon2 parameter selection out of `VaultService`
- make `VaultService::initialize()` depend on the factory instead of constructing `VaultMetadata` directly

Reason:

- this keeps vault lifecycle orchestration separate from KDF policy
- it prevents `VaultService` from being tied to `Argon2ParamsConfig`
- it makes tests easier because metadata creation can be faked independently

## 2.7 `src/service/entry_service.rs`

This file should convert:

- `EntryDraft`
- tags
- secret draft fields

into:

- `Entry`
- `SecretField`
- repository writes

Suggested methods:

```rust
pub struct EntryService<ER, SR, TR, ENC> {
    pub entries: ER,
    pub secrets: SR,
    pub tags: TR,
    pub encryptor: ENC,
}

impl<ER, SR, TR, ENC> EntryService<ER, SR, TR, ENC> {
    pub async fn create_from_draft(&self, draft: EntryDraft, key:
Option<&MasterKey>) -> Result<EntryId>;
    pub async fn update_from_draft(&self, draft: EntryDraft, key:
Option<&MasterKey>) -> Result<()>;
    pub async fn get_detail(&self, entry_id: EntryId, key: Option<&MasterKey>)
-> Result<EntryDetail>;
    pub async fn list(&self, filter: &EntryFilter) ->
Result<Vec<EntryListItem>>;
}
```

Do not make the UI assemble tags and secret fields by itself.

## 2.8 `src/service/vault_service.rs`

This file should handle only the vault lifecycle:

- initialize
- unlock
- check whether the vault is initialized

Clarification on `lock`:

- do not force a fake `lock()` method into `VaultService` just to match the
  outline
- in the current design, locking is primarily an application-state concern
- locking means:
  - drop the in-memory `MasterKey`
  - switch `VaultState` from `Unlocked` to `Locked`
- keep `VaultService` focused on initialization and unlock flows

Suggested methods:

```rust
pub struct VaultService<VR, KD, MF> {
    pub metadata_repo: VR,
    pub key_deriver: KD,
    pub metadata_factory: MF,
}

impl<VR, KD, MF> VaultService<VR, KD, MF> {
    pub async fn is_initialized(&self) -> Result<bool>;
    pub async fn initialize(&self, password: &str) -> Result<MasterKey>;
    pub async fn unlock(&self, password: &str) -> Result<MasterKey>;
}
```

The first implementation can stop here, but note one subtle point:

- `unlock()` can re-derive the key from stored metadata
- this alone does not prove the password is correct
- wrong-password detection only becomes possible when you either:
  - decrypt known ciphertext successfully, or
  - store an explicit key-check value in metadata

Do not hide this limitation in the plan. Call it out clearly while building phase one.

## 2.9 `src/app/action.rs`

Define actions clearly first. That keeps the UI from turning into a state mess.

At minimum:

```rust
pub enum Action {
    Quit,
    Tick,
    MoveUp,
    MoveDown,
    OpenSelected,
    Back,
    StartCreate,
    StartEdit,
    DeleteSelected,
    SaveDraft,
    UpdateDraftTitle(String),
    UpdateDraftBody(String),
    UnlockVault(String),
    LockVault,
    ShowMessage(String),
    ClearMessage,
}
```

The first version does not need every form input to be actionized immediately.

## 2.10 `src/app/state.rs`

Stabilize the top-level state early.

Suggested sketch:

```rust
pub enum Screen {
    Unlock,
    List,
    Detail,
    Editor,
}

pub enum VaultState {
    Locked,
    Unlocked,
}

pub struct EntryListState {
    pub items: Vec<EntryListItem>,
    pub selected: usize,
    pub filter: EntryFilter,
}

pub struct UnlockState {
    pub first_time_setup: bool,
    pub password_input: String,
    pub error_message: Option<String>,
}

pub struct AppState {
    pub screen: Screen,
    pub vault_state: VaultState,
    pub list_state: EntryListState,
    pub selected_entry: Option<EntryDetail>,
    pub editor_draft: Option<EntryDraft>,
    pub unlock_state: UnlockState,
    pub status_message: Option<String>,
    pub should_quit: bool,
}
```

The key distinction:

- `selected_entry` is detail-view state
- `editor_draft` is edit-view state
- they should not share one structure

## 2.11 `src/ui/root.rs`

This file should only dispatch rendering based on `AppState.screen`.

Suggested logic:

```rust
match app.screen {
    Screen::Unlock => render_unlock(...),
    Screen::List => render_list(...),
    Screen::Detail => render_detail(...),
    Screen::Editor => render_editor(...),
}
```

Do not put the entire layout in one oversized function.

## 2.12 `src/main.rs`

In the first version, `main.rs` should only:

- load config
- initialize the database
- initialize schema
- initialize repositories
- initialize services
- initialize `AppState`
- start the event loop

It should not contain:

- SQL statements
- crypto logic
- screen layout code

## 3. Concrete Development Steps

## 3.1 Step One: Create Modules Without Full Implementations

Create these files and make sure the module structure builds:

```text
src/app/mod.rs
src/app/action.rs
src/app/event.rs
src/app/state.rs
src/ui/mod.rs
src/ui/root.rs
src/domain/mod.rs
src/domain/entry.rs
src/domain/secret.rs
src/domain/tag.rs
src/service/mod.rs
src/service/entry_service.rs
src/service/vault_service.rs
src/storage/mod.rs
src/storage/traits.rs
src/infra/mod.rs
src/infra/config.rs
src/infra/crypto/mod.rs
src/infra/sqlite/mod.rs
src/infra/sqlite/schema.rs
```

Done when:

- every module is referenced correctly with `mod`
- the project compiles

## 3.2 Step Two: Fill in `domain`

Do not touch the database yet.

Done when:

- `Entry`, `Tag`, `SecretField`, and `EntryDraft` are defined
- these structures compile independently

## 3.3 Step Three: Define Traits

Fix the persistence capabilities first.

Done when:

- `EntryRepository`
- `SecretRepository`
- `TagRepository`
- `VaultMetadataRepository`

are stable enough to stop renaming

## 3.4 Step Four: Initialize SQLite

Only do:

- open the database
- execute schema initialization

Done when:

- a local SQLite file can be created
- tables are created successfully

## 3.5 Step Five: Validate Repositories Without UI

You can use temporary code in `main.rs` or unit tests.

Verify:

- insert one `Entry`
- query the list
- query details
- save tags
- save secret fields

Done when:

- the persistence layer works without the TUI

This step is important. Do not skip it.

Current planning note:

- after the vault and encryptor groundwork, finish the SQLite repository implementations before moving into `AppState`
- specifically:
  - `EntryRepository`
  - `TagRepository`
  - `SecretRepository`
- service tests with fakes are useful, but they do not replace real repository validation

## 3.6 Step Six: Implement the Vault

Start with:

- first-time initialization
- unlock
- persist `vault_metadata`
- derive keys from stored KDF parameters
- encrypt a string
- decrypt it back

Done when:

- the same password and stored metadata re-derive the same key
- `vault_metadata` round-trips through SQLite
- encrypt and decrypt round-trip with the derived key
- wrong-password behavior is defined explicitly:
  - either decryption fails later, or
  - `unlock()` validates against a stored key-check value

Recommended checkpoint split:

1. implement `VaultMetadataRepository` and test SQLite round-trips
2. implement `KeyDeriver` using `VaultMetadata`
3. implement `VaultService` with initialization and unlock
4. refactor `VaultService` to depend on `VaultMetadataFactory`
5. implement `Encryptor`
6. only then decide whether to add immediate password validation

Immediate next step:

- do the factory refactor before implementing `Encryptor`
- specifically:
  - add `VaultMetadataFactory`
  - add an Argon2-backed factory implementation
  - remove direct `Argon2ParamsConfig` construction from `VaultService`

## 3.7 Step Seven: Build Services

At this point, connect drafts, repositories, and encryption.

Done when:

- an `EntryDraft` can be turned into an `Entry`
- secret fields can be encrypted before persistence
- detail loading can decrypt for display

## 3.8 Step Eight: Build the Minimal `AppState`

Do not rush into complex input handling.

First support:

- unlock screen
- list screen
- detail screen
- editor screen

Done when:

- screens can switch cleanly
- state does not become inconsistent

## 3.9 Step Nine: Build the Minimal TUI

Suggested render order:

1. list screen
2. detail screen
3. unlock screen
4. editor screen

Reason:

- list and detail stabilize faster
- the editor usually takes the most time

## 3.10 Step Ten: Clean Up `main.rs`

After the rest works, go back and remove temporary wiring:

- remove throwaway debug logic
- consolidate initialization
- consolidate the event loop

Done when:

- `main.rs` stays thin
- business logic has been moved out

## 4. Minimum First-Version Screen State

## 4.1 Unlock Screen

At minimum it should manage:

- whether this is first startup
- password input
- status text

## 4.2 List Screen

At minimum it should manage:

- current list data
- selected index
- current filter

## 4.3 Detail Screen

At minimum it should manage:

- current entry detail
- whether sensitive fields are visible

## 4.4 Editor Screen

At minimum it should manage:

- title
- body
- kind
- tags
- secret fields
- unsaved changes state

## 5. First-Version Data Flow

Recommended structure:

### 5.1 Create an Entry

1. user presses `n`
2. create a blank `EntryDraft`
3. switch `screen` to `Editor`
4. user edits content
5. user presses `s`
6. `EntryService` saves
7. return to list and refresh

### 5.2 View Details

1. user selects an entry in the list
2. presses `Enter`
3. service loads entry, tags, and secret fields
4. update `selected_entry`
5. switch `screen` to `Detail`

### 5.3 Unlock the Vault

1. on startup, check `vault_metadata`
2. if missing, enter first-time setup flow
3. otherwise prompt for the password
4. on success, hold a `MasterKey`
5. update `vault_state` to `Unlocked`

## 6. Things the First Version Should Avoid

To reduce rework, avoid adding these too early:

- Markdown preview
- SQLite FTS
- PostgreSQL implementation
- custom theming system
- complex multi-pane editor
- attachment system
- automatic sync

These should wait until the MVP is stable.

## 7. Suggested First-Version Dependencies

Potential additions:

```toml
anyhow = "1"
thiserror = "2"
chrono = "0.4"
uuid = { version = "1", features = ["v4", "serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ratatui = "0.30"
crossterm = "0.29"
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-rustls"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
argon2 = "0.5"
chacha20poly1305 = "0.10"
rand = "0.8"
zeroize = "1"
directories = "5"
```

If you decide not to use async yet, you can replace `sqlx + tokio` with `rusqlite`.
If PostgreSQL is a real follow-up requirement, `sqlx` is usually the smoother path.

## 8. What to Check After Each Stage

### 8.1 After `domain`

Check:

- are the names stable
- are `Entry` and `EntryDraft` separated clearly

### 8.2 After repositories

Check:

- can CRUD work without the UI
- can tags and secret fields be handled independently

### 8.3 After crypto

Check:

- does the wrong password fail
- does ciphertext avoid obvious plaintext leakage
- does the nonce change every time

### 8.4 After `app/state`

Check:

- are screen transitions simple and predictable
- are there multiple conflicting sources of truth

### 8.5 After TUI

Check:

- are list, detail, and edit now a complete loop
- do keybindings conflict
- are sensitive fields hidden while locked

## 9. Smallest Useful Starting Task

If you want to start coding immediately, do these three things first:

1. create all module files and make the project compile
2. complete `domain/entry.rs`, `domain/secret.rs`, and `domain/tag.rs`
3. complete `storage/traits.rs` and `infra/sqlite/schema.rs`

At that point, the project boundaries are already much more stable.

## 10. Reasonable Next Docs

If you want more docs later, the most useful next ones are:

- `docs/schema.md`
- `docs/crypto.md`
- `docs/ui-flow.md`

At the current stage, this file plus `docs/architecture.md` is enough to begin implementation.
