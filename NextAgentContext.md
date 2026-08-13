# Kiwi.Character — Handoff Context

Updated: 2026-07-14

## Project

Kiwi.Character is a Windows-oriented Tauri v2 desktop app for chatting with AI
characters. The frontend is React + TypeScript in `src/`; the Rust backend is
in `src-tauri/src/`. The app uses SQLite (`rusqlite`, bundled) and sends
OpenAI-compatible requests to a configured LLM server, primarily LM Studio.

Important contract files:

- `src/types.ts`: frontend DTOs.
- `src/api.ts`: the only frontend `invoke` bridge.
- `src-tauri/src/models.rs`: matching Rust DTOs with camelCase serde names.
- `src-tauri/src/commands.rs`: Tauri commands plus prompt construction.

The database lives at `<app_data_dir>/kiwi.db`; avatar files live under
`<app_data_dir>/avatars/`. SQLite has repositories under `src-tauri/src/db/`.
Never retain the DB mutex guard across an `.await`.

## Verification constraint

Smart App Control blocks local Rust compilation/builds with `os error 4551`.
Do not claim Cargo checks/tests or Tauri runtime testing passed until SAC is
disabled. `rusqlite` with bundled SQLite also needs an MSVC C toolchain.

Frontend changes should be checked with `npx tsc --noEmit`, but the current
work is Rust/prompt-only. `git diff --check` has passed for the current work.
`cargo fmt --check` reports pre-existing formatting differences in unrelated
Rust files; do not reformat those broadly without a separate request.

## Existing documentation

`Docs.md` was created as a broad codebase guide. It is currently untracked.
It describes architecture, UI, DB, LLM integration, commands, and limitations.

## Character-behavior problem

The user reported that characters follow their profiles too literally. Example:
Minah's profile mentions sneakers, basketball, athletic appearance, and a fiery
temper. When the user says "Oh, sorry so much..." after watching her practise,
the model repeatedly invents sneaker-related incidents (scuffing shoes, a phone
with a sneaker-drop countdown, tripping her) even though none occurred.

Diagnosis:

1. Strong profile facts were previously injected as plain prose, encouraging
   the model to demonstrate them as a checklist.
2. More importantly, `build_request` intentionally skips leading assistant
   messages before sending chat history because models such as Qwen3 reject a
   first non-system assistant turn. The character's initial greeting is an
   assistant message, so on the first user reply the model receives the system
   prompt plus an ambiguous apology, but not the opening basketball/staring
   context. It fabricates a cause and tends to choose the salient sneaker trait.

The user does **not** want a permanent/stale `<scene>` field. Recommended next
behavioral fix (not implemented yet): include the initial greeting as a private
`<opening_context>` block only while composing the first real user response,
then rely on normal history after that. Add an explicit grounding rule:

> Treat opening context and chat history as established facts. Do not invent
> actions, objects, physical contact, conflicts, or causes that were not
> established. Do not reinterpret an ambiguous apology through the character's
> strongest interest; respond only to known context or clarify naturally.

Do not implement this unless the user asks to proceed.

## Implemented, uncommitted prompt changes

The old `build_system_prompt` plain-text prompt in `src-tauri/src/commands.rs`
was replaced with a structured XML roleplay prompt. It now:

- states that profile data is private reference material, not dialogue;
- tells the model to show personality through choices/tone/reactions;
- treats traits as tendencies rather than obligations;
- discourages repeated hobbies, appearance, clothing, biography, and signature
  traits unless relevant to the current exchange;
- has no `<scene>` block;
- XML-escapes all character, persona, and optional user system-prompt content;
- gives application roleplay rules precedence over user-configured additional
  instructions.

Tests were added in `commands.rs` for XML structure, persona presence/absence,
escaping, absence of `<scene>`, and containment of user instructions. They have
not been compiled/run because SAC blocks Cargo.

## Prompt assets embedded in the binary

At the user's request, static prompt instructions and XML layouts were moved out
of Rust command code into these files:

```text
src-tauri/prompts/system_prompt.xml
src-tauri/prompts/xml_interpretation.txt
src-tauri/prompts/roleplay_rules.txt
src-tauri/prompts/character.xml
src-tauri/prompts/user_persona.xml
src-tauri/prompts/additional_user_instructions.xml
```

`src-tauri/src/prompts.rs` loads all of them with `include_str!`, so their
content is embedded into the Rust binary at compile time. It contains a strict
one-pass `{{placeholder}}` renderer, avoiding accidental replacement when user
text itself contains placeholder-looking text. `src-tauri/src/lib.rs` declares
`mod prompts;`, and `commands.rs` calls the rendering helpers.

Changing a prompt asset requires rebuilding/relaunching the app; it is not a
runtime file lookup. This is intentional per the user's request to embed them.

## Latest LM Studio debugging discussion

The user compiled a release build and still saw a fabricated sneaker/phone/trip
scenario. This supports the missing-opening-context diagnosis; naturalness rules
alone do not prevent event invention.

The user suspects LM Studio may retain old messages. Current facts:

- Kiwi uses the OpenAI-compatible `/v1/chat/completions` endpoint.
- LM Studio documents this endpoint as stateless; its native `/api/v1/chat` and
  OpenAI `/v1/responses` are the stateful alternatives. Thus the server should
  not be retaining an application chat as server-managed state.
- The app already logs the final logical `messages` array through the
  `kiwi::prompt` log target in `commands.rs` (`log_prompt`). This is close to
  what the app sends but not an explicitly labelled full JSON request body.
- Recent LM Studio has `lms log stream --source model --filter input,output` to
  stream formatted model input/output. `--source server` is useful for endpoint
  and status activity but model-input logging is the useful prompt comparison.
- A standard OpenAI-compatible API does not generally provide an independent
  "echo the payload received" endpoint.

Recommended debug feature, discussed but **not implemented**:

1. In debug builds only (`#[cfg(debug_assertions)]`), log the exact serialized
   outgoing JSON request: URL, model, messages, temperature, max_tokens, and
   stream flag. Do not enable this in release because prompts/personas/chat text
   are sensitive.
2. Run `lms log stream --source model --filter input,output` separately and
   compare the app's outgoing log with LM Studio's model input.
3. Matching inputs mean the response is model/prompt behavior; a mismatch
   demonstrates a server/template transformation.

The user asked whether this is possible; they have not yet explicitly requested
implementation of the debug-only JSON logging.

## Git / workflow status

The root `AGENTS.md` says: do not commit until user reviews and approves each
step. The user explicitly reiterated: do not commit until review. No commits
have been made for this work.

At the time this handoff was written, expected working-tree changes are:

```text
M  src-tauri/src/commands.rs
M  src-tauri/src/lib.rs
?? src-tauri/src/prompts.rs
?? src-tauri/prompts/
?? Docs.md
?? Prompt.md
?? NextAgentContext.md
```

`Prompt.md` is user-authored scratch context and should not be changed unless
asked. `Docs.md` and this handoff are also untracked. Preserve unrelated user
changes and do not use destructive git commands.
