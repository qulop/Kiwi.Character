# Kiwi.Character — React + TypeScript frontend

Faithful React + TS port of the two hi-fi mockups (`Main Page.dc.html`,
`Chat Page.dc.html`), structured to drop into a **Tauri** app and talk to your
**Rust** backend via `invoke`.

> **Stack note.** You mentioned a *pure HTML/CSS/TS* Tauri frontend, then asked
> for **React + TS** — this package is the React version (most common Tauri
> setup: Vite + React + TS). If you'd rather have framework-free TS, say so and
> I'll re-emit the same UI as vanilla TS modules.

## Layout

```
src/
  styles.css                 All visual styling (CSS vars, fonts, components, states)
  types.ts                   Domain types — mirror these in your Rust structs
  api.ts                     The ONLY file that imports @tauri-apps/api — typed invoke wrappers
  main.tsx                   React entry (createRoot)
  App.tsx                    Page routing + shared state (characters, history, modals)
  components/
    Sidebar.tsx              Left rail (brand, Create, search, history, Settings)
    MainPage.tsx             Character pickup grid
    ChatPage.tsx             Message thread + composer + character panel
    SettingsModal.tsx        Endpoint test → model config (gated)
    NewCharacterModal.tsx    Create-character form
    icons.tsx                Inline SVG icons
preview.html                 DEV-ONLY browser smoke test (mocked backend, no build)
```

## Run it standalone (no Tauri) to preview

Open `preview.html` in a browser — it transpiles the real `.tsx` files with
Babel and stubs `invoke` with sample data. Use it for visual checks only; it is
**not** part of the shipped app.

## Wire into a Tauri + Vite project

1. Scaffold (or reuse) a Tauri React-TS app, e.g.
   `npm create tauri-app@latest` → React → TypeScript.
2. Copy `src/` over your generated `src/`. Install the API package:
   `npm i @tauri-apps/api`.
3. Ensure your `index.html` has `<div id="root"></div>` and loads
   `/src/main.tsx`. Done — `npm run tauri dev`.

## The contract your Rust side must implement

`api.ts` calls these commands. Argument keys are **camelCase** on the JS side;
with Tauri's default the Rust parameters are snake_case and it maps them for you.
Return types must serde-serialize to the shapes in `types.ts`.

```rust
#[tauri::command]
fn list_characters() -> Vec<Character> { /* ... */ }

#[tauri::command]
fn get_character(id: String) -> Character { /* ... */ }

#[tauri::command]
fn create_character(input: NewCharacterInput) -> Character { /* ... */ }

#[tauri::command]
fn list_history() -> Vec<HistoryItem> { /* ... */ }

#[tauri::command]
fn list_messages(conversation_id: String) -> Vec<ChatMessage> { /* ... */ }

#[tauri::command]
async fn send_message(conversation_id: String, content: String) -> ChatMessage { /* ... */ }

#[tauri::command]
fn get_settings() -> ModelSettings { /* ... */ }

#[tauri::command]
fn save_settings(settings: ModelSettings) { /* ... */ }

#[tauri::command]
async fn test_endpoint(endpoint: String) -> EndpointTestResult { /* ping LM Studio / Ollama */ }

#[tauri::command]
async fn load_model(settings: ModelSettings) { /* ... */ }
```

Register them in `main.rs`:

```rust
tauri::Builder::default()
  .invoke_handler(tauri::generate_handler![
    list_characters, get_character, create_character,
    list_history, list_messages, send_message,
    get_settings, save_settings, test_endpoint, load_model,
  ])
  .run(tauri::generate_context!())
  .expect("error while running tauri application");
```

Use `#[serde(rename_all = "camelCase")]` on the structs so fields like
`initialMessage` / `contextLength` match `types.ts`.

## Token streaming (recommended for local LLMs)

`send_message` is the simple request/response form. For live typing, have a
`stream_message` command emit events and listen on the JS side:

```rust
// Rust: for each token
window.emit("chat://token", token)?;
window.emit("chat://done", ())?;
```

```ts
// JS: see api.ts `streamMessage` + the @tauri-apps/api/event listener
import { listen } from '@tauri-apps/api/event';
const stop = await listen<string>('chat://token', e => appendToLastBubble(e.payload));
```

`ChatPage.tsx` currently appends the whole reply from `send_message`; swap that
for the event listener when you move to streaming. The typing indicator and
optimistic user bubble are already in place.

## Notes / intentional gaps (matching the mockups)

- **Personas** panel is disabled ("coming soon") per the design.
- The endpoint **must be tested** before model settings unlock — that gate lives
  in `SettingsModal.tsx` (`verified` state), not the backend.
- Avatar upload reads the file as a data URL client-side; decide server-side
  whether to persist bytes or a path in `create_character`.
- Styling is plain CSS (not CSS-in-JS) so it's easy to theme — the purple accent
  and hand-drawn button radius are CSS variables at the top of `styles.css`.
