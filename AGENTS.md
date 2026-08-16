# AGENTS.md — Implementation Playbook for Kiwi.Character

This file tells an implementing agent **how** to execute the roadmap. Read it
fully before touching code. The roadmap is split into numbered **steps**; each
step has its own detailed doc (see the index below).

---

## 0. Project facts the agent must know

- **App root:** repository root (Tauri v2 + Vite + React-TS). Frontend in
  `src/`, Rust backend in `src-tauri/src/`.
- **Backend ↔ frontend contract:** the frontend calls Rust via `invoke` — all
  wrappers live in `src/api.ts`; shared shapes in `src/types.ts`
  (`serde(rename_all = "camelCase")` on the Rust side keeps them aligned).
- **Database:** SQLite via `rusqlite` (bundled). **The complete, authoritative
  schema is in [`agent-docs/database-scheme.md`](agent-docs/database-scheme.md).** Do not invent tables
  or columns — if a change is needed, update `database-scheme.md` first, then the
  migration, then code.
- **DB & assets location:** `<app_data_dir>/kiwi.db` and `<app_data_dir>/avatars/`
  where `app_data_dir` comes from `app.path().app_data_dir()`.
- **Avatars:** stored as **files on disk**, DB holds only the relative path.

### ⚠️ Build/verification constraint (read this)
Smart App Control is **enforced** on the current dev machine, so `cargo check`,
`cargo build`, and `tauri dev` fail with `os error 4551` (unsigned build-script
binaries are blocked). **The agent cannot compile or run the Rust locally until
the user disables SAC.** Consequences:
- Do **not** claim the Rust compiles/works based on a local build — it can't run.
- Frontend TypeScript **can** still be verified with `npx tsc --noEmit` at the
  repository root.
- When a step needs runtime verification, state clearly that it is *pending a
  build with SAC off*, and ask the user to verify (or to disable SAC so the agent
  can).
- `rusqlite` with `bundled` also needs an **MSVC C toolchain**; note this when the
  build is first attempted.

---

## 1. Execution algorithm (follow exactly)

For each step, in order (Step 0 onward):

1. **Open the step doc** (`agent-docs/<n>_<name>.md`) and re-read
   `database-scheme.md` if the step touches data.
2. **Implement every substep** in the doc: create/modify the listed files, apply
   the described changes, keep the `api.ts`/`types.ts` contract in sync on both
   sides.
3. **Self-check**: run `npx tsc --noEmit` at the repository root for any frontend change.
   For Rust, do a careful manual review (the build is SAC-blocked). Note anything
   that can only be confirmed once SAC is off.
4. **PAUSE.** Present a concise summary of what changed and what remains
   unverified, then **wait for the user's feedback.** Do not start the next step.
5. **On user approval:** create the step's commits (see §2), then mark the step
   done in this file (see §3), then move to the next step.
6. **If the user requests changes:** apply them, re-check, and pause again. Only
   commit once the user approves.

**Never** batch multiple steps before a pause. **Never** commit before approval.

---

## 2. Commit discipline

After the user approves a step, commit the work as **several small, logically
coherent commits** — not one giant commit. Guidelines:

- Group related changes: e.g. *"Add SQLite schema + migrations"*, then
  *"Back character commands with the DB"*, then *"Persist settings in DB"*.
- Each commit message is **short and meaningful** (imperative mood, ≤ ~70 chars
  subject), describing the logical unit, not the file list.
- A commit should build conceptually on the previous one and leave the tree in a
  coherent state.
- Do **not** mix unrelated steps in one commit.
- Commit on the current working branch unless the user asks for a feature branch.
  Do not push unless the user asks.
- `agent-docs/` is git-ignored (the detailed per-step specs are local working
  notes). **`AGENTS.md` itself lives at the repo root and IS tracked**, so its
  step-index updates (the `[STEP DONE]` marks) are committed — include this file
  in the step's final commit.

**Example commit sequence for Step 1:**
```
1. Add rusqlite dependency and SQLite schema/migrations
2. Introduce Db state and character repository functions
3. Route character/conversation/message commands through the DB
4. Persist model settings in the database
```

---

## 3. Marking a step complete

When a step is approved and committed, edit **this file's** step index below:

- Prefix the line with `[STEP DONE]`.
- Strike the text through using `<s>...</s>` (or `~~...~~`).

Example — before:
```
- [ ] **Step 1 — Database interlayer** → [1_database-interlayer.md](1_database-interlayer.md)
```
after:
```
- [x] [STEP DONE] <s>**Step 1 — Database interlayer** → [1_database-interlayer.md](1_database-interlayer.md)</s>
```

---

## 4. Step index

> Execute top to bottom. Each links to its detailed spec.

- [x] [STEP DONE] <s>**Step 0 — Chat reliability & streaming** → [0_chat-reliability-and-streaming.md](0_chat-reliability-and-streaming.md)</s>
      _Streaming + error surfacing + hardened request. Root cause of the LM Studio
      "Channel Error" found: chat templates (Qwen3) reject an assistant greeting
      before the first user turn. Fixed by dropping leading assistant messages
      from the model request. Confirmed working end-to-end._
- [x] [STEP DONE] <s>**Step 1 — Database interlayer** → [1_database-interlayer.md](1_database-interlayer.md)</s>
      _SQLite (`rusqlite` bundled) with schema/migrations, per-entity
      repositories, and all commands backed by the DB. Seed data kept.
      Not runtime-verified (SAC blocks the build)._
- [x] [STEP DONE] <s>**Step 2 — Jump to chat after character creation** → [2_navigate-to-chat-after-create.md](2_navigate-to-chat-after-create.md)</s>
      _Creating a character now navigates straight into its chat; modal stays open on error._
- [x] [STEP DONE] <s>**Step 3 — Character avatar on the chat page** → [3_character-avatar-on-chat-page.md](3_character-avatar-on-chat-page.md)</s>
      _Avatars served via the asset protocol (protocol-asset feature + scope),
      resolved to asset: URLs in api.ts, rendered by a shared Avatar component.
      Compile-verified; avatars confirmed working in the app._
- [x] [STEP DONE] <s>**Step 4 — Card three-dots menu: favourite & delete** → [4_character-card-actions-favorite-delete.md](4_character-card-actions-favorite-delete.md)</s>
      _is_favorite flag + set_favorite/delete_character commands; card menu with
      favourite/delete, red heart overlay. Compile-verified. (Favourites
      filtering itself is Step 5.)_
- [x] [STEP DONE] <s>**Step 5 — Category filters: All / Recent / Favourite** → [5_category-filters-all-recent-favourite.md](5_category-filters-all-recent-favourite.md)</s>
      _createdAt/lastMessageAt on the DTO (join); MainPage filters All / Recent
      (3-day window) / Favourites, with empty states. Compile-verified._
- [x] [STEP DONE] <s>**Step 6 — Character name-collision check** → [6_character-name-collision-check.md](6_character-name-collision-check.md)</s>
      _Case-insensitive uniqueness enforced in insert (+ UNIQUE mapping) and
      surfaced inline in the modal, with a live blur pre-check. Compile-verified,
      no warnings._
- [x] [STEP DONE] <s>**Step 7 — Message actions: copy / edit / remove / rewind** → [7_message-actions-delete-copy-rewind.md](7_message-actions-delete-copy-rewind.md)</s>
      _Top-right three-dots menu (flips up near the page bottom). All: Copy,
      Remove. User also: Edit (inline) + Rewind to here. Backend delete/rewind/
      update_message; thread reloads after send for real ids. Compile-verified._
- [x] [STEP DONE] <s>**Step 8 — Character Info modal (view & edit)** → [8_character-info-modal.md](8_character-info-modal.md)</s>
      _Generalized CharacterFormModal for create + edit; update_character command
      (self-excluding name check, data-URL-only avatar replace). Both chat headers
      open the pre-filled modal with a Save button. Compile-verified._
- [x] [STEP DONE] <s>**Step 9 — Like button in the chat** → [9_chat-like-button.md](9_chat-like-button.md)</s>
      _Favourite toggle in the right-panel header (right of the name): red heart
      when favourited, grey otherwise. Reuses set_favorite/toggleFavorite._
- [x] [STEP DONE] <s>**Step 10 — History avatars + time grouping** (implemented directly, no separate spec)</s>
      _Sidebar history shows character avatars and is grouped by time buckets
      (Today / Yesterday / This week / This month / This year / A while ago).
      list_history returns avatar + lastMessageAt; buckets computed client-side.
      Compile-verified._
- [x] [STEP DONE] <s>**Step 11 — "Delete chat" in history menu** (implemented directly, no separate spec)</s>
      _History three-dots now opens a working menu with "Delete chat" (trash
      icon). delete_conversation command removes the conversation (cascades
      messages); character kept. Menu flips up near the bottom. Compile-verified._
- [x] [STEP DONE] <s>**Step 12 — Auto-growing message input** (implemented directly, no separate spec)</s>
      _Composer is a textarea that grows with content (capped at 160px, then
      scrolls). Enter sends, Shift+Enter inserts a newline. Compile-verified._
- [x] [STEP DONE] <s>**Step 13 — Empty "continue" send** (implemented directly, no separate spec)</s>
      _Empty/whitespace send: if the last turn is the AI's, a hidden technical
      user message makes it continue (new bubble, message not shown); if the last
      turn is the user's, it just replies. Added a hidden flag on messages
      (schema v2 migration) + stream_continue command. Compile-verified._
- [x] [STEP DONE] <s>**Step 14 — Memory schema and migrations** → [14_memory-schema-and-migrations.md](14_memory-schema-and-migrations.md)</s>
      _Before code, extend `agent-docs/database-scheme.md` with singleton memory
      settings plus `memories` and `memory_sources`. Add schema version 4,
      migration code, and repositories. Vectors are normalized little-endian
      `f32` BLOBs; memories are scoped to a conversation/character and optional
      persona, and source links support later invalidation._
- [ ] **Step 15 — Persist Memory settings** → [15_memory-settings.md](15_memory-settings.md)
      _Back the existing Settings > Memory controls with the DB. Add Rust/TS
      DTOs, repository functions, commands, and `api.ts` wrappers for enabled,
      embedding endpoint/model/dimensions, recent-message limit, recall depth,
      and reserved ranking configuration. The embedding endpoint is configured
      separately from the character-model endpoint so CPU inference can be used._
- [ ] **Step 16 — Embedding provider and diagnostics** → [16_embedding-provider.md](16_embedding-provider.md)
      _Implement the OpenAI-compatible embedding client for
      `Qwen/Qwen3-Embedding-0.6B`: request/response validation, vector
      normalization/serialization, and a real Settings test action. Default to
      a separately configured CPU embedding runtime; document that embedding
      failure must never prevent a chat reply._
- [ ] **Step 17 — Semantic recall and prompt integration** → [17_semantic-recall.md](17_semantic-recall.md)
      _Load active memories in the current conversation/persona scope, embed the
      latest user message using a fixed retrieval instruction, score candidates
      with cosine similarity, and add the top six to a private
      `<recalled_memories>` system-prompt block. Define a reranker interface but
      use direct embedding ranking only in this step._
- [ ] **Step 18 — Short-term history window** → [18_short-term-history-window.md](18_short-term-history-window.md)
      _Replace full visible-history prompt construction with the configured most
      recent message window (default 20 messages / 10 turns), while preserving
      required hidden technical turns such as Continue. The final request is
      character prompt + recalled memories + short-term conversation context._
- [ ] **Step 19 — Manual memories and invalidation** → [19_manual-memories-and-invalidation.md](19_manual-memories-and-invalidation.md)
      _Add a message action to save a manual memory and UI to list, edit, pin,
      delete, or clear memories. Editing, deleting, or rewinding a source
      message must mark dependent generated memories stale or invalid so they
      can never be retrieved._
- [ ] **Step 20 — Automatic memory writer** → [20_automatic-memory-writer.md](20_automatic-memory-writer.md)
      _After a successful reply, asynchronously extract only durable facts,
      preferences, commitments, and meaningful events; deduplicate, embed, and
      save source-linked candidates. It must run after the visible reply and
      must not delay streaming or hold a DB mutex during model/embedding awaits._
- [ ] **Step 21 — Optional reranker quality mode** → [21_optional-memory-reranker.md](21_optional-memory-reranker.md)
      _Implement the reserved reranker interface with
      `Qwen/Qwen3-Reranker-0.6B` only after embedding-only retrieval is measured.
      In opt-in quality mode, rerank 20–30 embedding candidates down to six;
      record first-token latency and VRAM impact before considering it a default._

---

## 5. Cross-cutting rules

- Keep `api.ts` and `types.ts` as the **only** contract surface — when a command's
  shape changes, update both the Rust struct/signature and these TS files.
- Preserve the camelCase (JS) ↔ snake_case (Rust) mapping via
  `serde(rename_all = "camelCase")`.
- Never hold a `MutexGuard` (DB connection or state) across an `.await`. Snapshot
  the data you need, drop the guard, await, then re-lock. `send_message` already
  follows this pattern — keep it.
- Timestamps are ms-epoch `i64` everywhere (`now_ms()` in `state.rs`).
- After any frontend edit, run `npx tsc --noEmit` at the repository root and fix errors
  (the strict config has `noUnusedLocals`/`noUnusedParameters` on).
