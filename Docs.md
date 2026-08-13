# Kiwi.Character — Codebase Guide

## Purpose

Kiwi.Character is a Tauri desktop application for conversations with AI
characters. It is designed around OpenAI-compatible LLM endpoints, with LM
Studio as the primary local-server target. It can also work with compatible
servers such as Ollama's `/v1` interface or llama.cpp server. The interface is
implemented in React and TypeScript; the desktop/backend layer is Rust.

The product currently provides:

- A character library with search, All/Recent/Favourites filtering, favourite
  and delete actions, and case-insensitive unique character names.
- Per-character persisted chats, including a seeded initial greeting.
- Streaming model responses, visible error reporting, editing/deleting/rewinding
  messages, copying message text, and an empty-send "continue" action.
- Character and user-persona creation/editing, including locally stored avatars.
- Per-conversation persona selection; the selected persona is incorporated into
  the model system prompt.
- Group creation and sidebar display. A group conversation itself is not yet
  implemented, so group history rows are deliberately inert.
- Model endpoint testing, model discovery/loading/unloading, and persisted
  generation settings.

## Stack and repository layout

| Area | Technology / location | Responsibility |
| --- | --- | --- |
| Desktop shell | Tauri v2, `src-tauri/` | App lifecycle, native filesystem/database access, command registration |
| Frontend | React 19 + TypeScript, `src/` | Application state, pages, modal UI, streaming presentation |
| Build | Vite 7, `vite.config.ts` | Serves `src/` on port 1420 and builds to `dist/` |
| Persistence | SQLite through `rusqlite`, `src-tauri/src/db/` | Schema, migrations, repositories, seed data |
| LLM client | `reqwest`, `src-tauri/src/openai.rs` | OpenAI-compatible REST and SSE streaming client |
| Styling | Plain CSS, `src/styles.css` | Global theme, layout, components and responsive presentation |

Important frontend files:

- `src/main.tsx` mounts React into `#root`.
- `src/App.tsx` is the frontend coordinator. It owns loaded data, selected page,
  modal stack, selected character/persona, and optimistic UI updates.
- `src/types.ts` defines the shared TypeScript DTOs and defaults.
- `src/api.ts` is the only frontend Tauri bridge. All backend calls are typed
  wrappers around `invoke` here.
- `src/events.ts` centralizes the chat streaming event names.
- `src/components/` contains page and modal components. `Avatar.tsx` provides
  the shared character-avatar/fallback monogram; `GroupAvatar.tsx` renders a
  custom group image or a member-avatar collage.

Important backend files:

- `src-tauri/src/lib.rs` creates the app, initializes logging and the database,
  manages `AppState`, and registers every command.
- `src-tauri/src/commands.rs` implements the Tauri command surface and the
  system-prompt/chat orchestration.
- `src-tauri/src/models.rs` contains Rust DTOs that mirror `src/types.ts`.
- `src-tauri/src/state.rs` holds the mutex-protected database state plus UUID
  and millisecond-epoch helpers.
- `src-tauri/src/openai.rs` is the small purpose-built endpoint client.
- `src-tauri/src/db/` contains schema startup/migration code and one repository
  module per persisted area.

## Frontend behavior

`App.tsx` has two page states: `main` (character selection) and `chat`. It
loads characters, conversation history, personas, groups, and model settings
once at startup. It also pings the configured endpoint immediately and every
45 seconds so the Settings modal can start in an already-verified state when
the server is reachable.

The sidebar is shared between both pages. It contains the app navigation,
character/group creation actions, search, settings, and history. Character
history is time-bucketed as Today, Yesterday, This week, This month, This year,
or A while ago. It can delete a conversation while retaining its character.
Groups are merged into this visual list by creation time but cannot be opened
until group chat exists.

The main page displays character cards and filters them as follows:

- **All** shows every character.
- **Recent** includes characters created or messaged within the last three days.
- **Favourites** includes characters whose `isFavorite` value is true.

Creating a character immediately opens its chat. Editing a character updates
the active header, cards, and corresponding history labels/avatar. Deleting a
character also removes its cascaded chats from the interface. These UI updates
are optimistic in several places; API errors are logged to the browser console.

The chat page loads visible messages for its conversation and uses Tauri events
to append streamed assistant content. The composer grows up to 160 px: Enter
sends and Shift+Enter inserts a newline. Message actions offer Copy and Remove;
user messages additionally offer inline Edit and Rewind to here. An empty send
asks the model to continue if the latest turn was from the assistant; otherwise
it simply generates the pending reply to the latest user message.

Modals are held in a stack so opening, for example, a persona form over the
persona list supports returning via Back. Escape closes all currently stacked
modals. Character, persona, and group avatar selection uses
`AvatarCropModal.tsx`, which exports a 512×512 PNG data URL after drag/zoom
cropping.

## Backend/frontend contract

The bridge boundary is deliberately narrow:

1. React calls a wrapper in `src/api.ts`.
2. The wrapper calls `invoke('<command>', payload)`.
3. A command in `commands.rs` calls a database repository or an LLM helper.
4. Rust returns a serde DTO, then `api.ts` changes absolute avatar file paths
   into Tauri `asset:` URLs with `convertFileSrc`.

Rust DTOs use `#[serde(rename_all = "camelCase")]`, so Rust names such as
`initial_message` and `conversation_id` become `initialMessage` and
`conversationId` on the JavaScript side. `src/types.ts` and `src/api.ts` are
the frontend contract surface and must be updated alongside command/DTO changes.

### Command inventory

| Domain | Commands |
| --- | --- |
| Characters | `list_characters`, `get_character`, `create_character`, `update_character`, `character_name_available`, `set_favorite`, `delete_character` |
| Personas | `list_personas`, `create_persona`, `update_persona`, `delete_persona`, `get_active_persona`, `set_active_persona` |
| Groups | `list_groups`, `create_group` |
| Conversations/messages | `list_history`, `list_messages`, `send_message`, `stream_message`, `stream_continue`, `update_message`, `delete_message`, `rewind_to_message`, `delete_conversation` |
| Settings/models | `get_settings`, `save_settings`, `test_endpoint`, `loaded_models`, `load_model`, `unload_model` |

Streaming uses three window events declared in `src/events.ts`:

- `chat://token` carries each content chunk.
- `chat://done` signals a persisted successful reply.
- `chat://error` carries a user-displayable failure reason.

## Database and files

At startup Tauri resolves its OS application-data directory. The app stores:

```text
<app_data_dir>/kiwi.db
<app_data_dir>/avatars/<generated-uuid>.<png|jpg|webp>
```

SQLite runs with foreign keys and WAL mode. A single `rusqlite::Connection` is
kept inside `AppState.db: Mutex<Db>`. Async commands snapshot settings, character,
persona, and message data while holding the mutex, release it before HTTP awaits,
then re-lock only to persist the reply. A mutex guard must never live across an
`.await`.

The schema is in `src-tauri/src/db/schema.sql`; database startup and migrations
are in `db/mod.rs`. The current schema version is 3. Existing-database migrations
added `messages.hidden` in v2 and `conversations.active_persona_id` in v3.

| Table | Role |
| --- | --- |
| `characters` | Character details, relative avatar path, favourite flag and timestamps |
| `conversations` | Character chat identity, newest activity, selected persona |
| `messages` | Ordered persisted turns, with a `hidden` flag for technical turns |
| `personas` | User identity profiles and their relative avatar paths |
| `groups` / `group_members` | Group metadata and ordered character membership |
| `settings` | Singleton model configuration row (`id = 1`) |

Opening `conv-<character-id>` lazily creates the conversation and, when present,
stores its character's initial greeting as the first assistant message. Deleting
a conversation cascades its messages; opening it later recreates a fresh greeting.
Deleting a character cascades conversations/messages. Deleting a persona clears
that persona from any selected conversations first.

Avatar bytes arrive from the UI as data URLs. The repositories decode these to
disk and store only an `avatars/<filename>` relative path in SQLite. Replacing an
avatar writes a new UUID filename, then best-effort removes the old file; this
avoids webview cache collisions. Tauri's asset protocol is enabled and scoped to
the app-data `avatars` directory.

On an empty database, three sample characters are seeded: Aria, Sherlock Holmes,
and Luna. Character names are enforced unique case-insensitively by a SQLite
unique index and are pre-checked by the frontend form.

## LLM integration and prompt construction

`openai.rs` deliberately uses a small direct HTTP client rather than an SDK.
All requests target the configured OpenAI-style endpoint:

- `GET <endpoint>/models` tests connectivity and lists advertised models.
- `POST <endpoint>/chat/completions` sends regular or `stream: true` chat
  requests.
- `GET <host>/api/v0/models` lists loaded LLM/VLM models through LM Studio's
  native API; unsupported compatible servers can fail this best-effort call.
- Model loading is a 1-token chat request, relying on LM Studio's JIT loading.
- Model unloading invokes the LM Studio CLI: `lms unload <model>`.

Endpoint testing uses a 10-second timeout, normal completion 300 seconds,
streaming 600 seconds, and model loading 900 seconds. Failures include HTTP
status and, where available, the server response body so UI errors are useful.

Before sending a turn, `commands.rs` creates a system message from character
name, short info, description, appearance, the selected persona, and the saved
global system prompt. The persona is escaped and embedded as a `<persona>` block.
Leading assistant greetings are deliberately omitted from the model request until
the first real user turn: some chat templates, including Qwen3, reject a prompt
whose first non-system message is assistant-role. The greeting remains stored and
visible in the UI. Empty assistant replies are not persisted; reasoning-model
`reasoning_content` is used as a fallback when normal `content` is empty.

The "continue" operation adds a hidden user instruction only when the latest
stored role is assistant. Hidden messages are excluded by `list_messages` (and
therefore never displayed) but included in the LLM history.

## Settings

The default persisted configuration is:

```text
endpoint       http://localhost:1234/v1
model          llama-3.1-8b-instruct
contextLength  100
gpuOffload     60
temperature    0.8
maxTokens      2048
systemPrompt   empty
```

The Settings modal requires a successful endpoint test before its model
configuration controls can be used. It can show currently loaded LM Studio
models, select one already loaded (saving the choice), load a selected model,
and unload a loaded model. The `contextLength` and `gpuOffload` values are
persisted settings; the current OpenAI-compatible chat request sends model,
messages, temperature, max tokens, and stream mode. `max_tokens` is omitted
when non-positive to avoid requesting more than a server/model context permits.

## Runtime configuration, logs, and verification

The Tauri app identifier is `com.kiwi.character`. Its default desktop window is
1180×760 px with a minimum of 900×600 px. Vite serves development builds at
`http://localhost:1420`, and Tauri runs `npm run dev` before development or
`npm run build` before packaging.

The log plugin writes informational logs to stdout and to Tauri's OS log
directory. Chat prompt logging uses the `kiwi::prompt` target and records the
complete request message array; this is useful for debugging model behavior but
may contain private conversation content.

Useful checks:

```powershell
# Frontend type check (from the project root)
npx tsc --noEmit

# Development application
npm run tauri dev
```

On this development machine, Smart App Control blocks Rust build-script binaries
with `os error 4551`; `cargo check`, Cargo builds, and Tauri runtime verification
cannot currently be completed until it is disabled. `rusqlite`'s bundled SQLite
also requires an MSVC C toolchain when Rust builds are attempted. Frontend
TypeScript checks remain usable independently.

## Current limitations and extension points

- Group records can be created and shown, but group conversations, group prompt
  composition, and group deletion/editing are not implemented.
- The UI is optimized for local LM Studio behavior. Loading/unloading and the
  native loaded-model list are LM Studio-specific; ordinary OpenAI-compatible
  servers may still support basic chat and `/models` without those features.
- API calls in several optimistic UI paths only log failures rather than rolling
  back state. New work in these paths should decide whether user-visible errors
  and rollback are needed.
- The older `README.md` describes an earlier frontend-only integration state;
  this document reflects the implemented Tauri/SQLite application.

When adding a feature that crosses the desktop boundary, update the Rust DTO and
command, `src/types.ts`, and `src/api.ts` together. For data changes, update the
schema and migration path before repository code. Preserve millisecond timestamps
and avoid retaining a database mutex guard across asynchronous work.
