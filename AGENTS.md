# AGENTS.md — Implementation Playbook for Kiwi.Character

This file tells an implementing agent **how** to execute the roadmap. Read it
fully before touching code. The roadmap is split into numbered **steps**; each
step has its own detailed doc (see the index below).

---

## 0. Project facts the agent must know

- **App root:** `client/` (Tauri v2 + Vite + React-TS). Frontend in `client/src/`,
  Rust backend in `client/src-tauri/src/`.
- **Backend ↔ frontend contract:** the frontend calls Rust via `invoke` — all
  wrappers live in `client/src/api.ts`; shared shapes in `client/src/types.ts`
  (`serde(rename_all = "camelCase")` on the Rust side keeps them aligned).
- **Database:** SQLite via `rusqlite` (bundled). **The complete, authoritative
  schema is in [`database-scheme.md`](database-scheme.md).** Do not invent tables
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
- Frontend TypeScript **can** still be verified with `npx tsc --noEmit` in `client/`.
- When a step needs runtime verification, state clearly that it is *pending a
  build with SAC off*, and ask the user to verify (or to disable SAC so the agent
  can).
- `rusqlite` with `bundled` also needs an **MSVC C toolchain**; note this when the
  build is first attempted.

---

## 1. Execution algorithm (follow exactly)

For each step, in order (Step 0 → Step 6):

1. **Open the step doc** (`agent-docs/<n>_<name>.md`) and re-read
   `database-scheme.md` if the step touches data.
2. **Implement every substep** in the doc: create/modify the listed files, apply
   the described changes, keep the `api.ts`/`types.ts` contract in sync on both
   sides.
3. **Self-check**: run `npx tsc --noEmit` in `client/` for any frontend change.
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
- End every commit message with:
  ```
  Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
  ```
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

- [~] **Step 0 — Chat reliability & streaming** → [0_chat-reliability-and-streaming.md](0_chat-reliability-and-streaming.md) — ⚠️ **PARTIAL / DEFERRED**
      _Streaming infra + error surfacing implemented (committed with the baseline).
      The LM Studio "Channel Error" is **still unresolved**; per the user's
      decision this is revisited **after Step 6**. Do not mark `[STEP DONE]` until
      chat works end-to-end._
- [ ] **Step 1 — Database interlayer** → [1_database-interlayer.md](1_database-interlayer.md)
      _Introduce SQLite (`rusqlite`), the schema/migrations, and repository
      functions; back all existing commands with the DB instead of in-memory state._
- [ ] **Step 2 — Jump to chat after character creation** → [2_navigate-to-chat-after-create.md](2_navigate-to-chat-after-create.md)
      _After creating a character, open the chat page with that new character._
- [ ] **Step 3 — Character avatar on the chat page** → [3_character-avatar-on-chat-page.md](3_character-avatar-on-chat-page.md)
      _Render the character's uploaded avatar in the chat header, message rows, and
      right panel (not just on the selection page)._
- [ ] **Step 4 — Card three-dots menu: favourite & delete** → [4_character-card-actions-favorite-delete.md](4_character-card-actions-favorite-delete.md)
      _Add a three-dots button to each card expanding to "Mark as favourite" (heart)
      and "Delete" (cross); wire delete + favourite to the DB; show a red heart
      overlay on favourites._
- [ ] **Step 5 — Category filters: All / Recent / Favourite** → [5_category-filters-all-recent-favourite.md](5_category-filters-all-recent-favourite.md)
      _Implement the three category buttons. Recent = created or spoken-with within
      3 days; Favourite = explicitly favourited only._
- [ ] **Step 6 — Character name-collision check** → [6_character-name-collision-check.md](6_character-name-collision-check.md)
      _Reject creating two characters with the same name, case-insensitively, at
      both the DB and UX layers._

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
- After any frontend edit, run `npx tsc --noEmit` in `client/` and fix errors
  (the strict config has `noUnusedLocals`/`noUnusedParameters` on).
