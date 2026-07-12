/* ============================================================
   Tauri command bridge.

   Every function here is a thin typed wrapper around
   `invoke('<command>', ...)`. The matching Rust side lives in
   `src-tauri/src/` — see README.md for the `#[tauri::command]`
   signatures these expect.

   Keeping all `invoke` calls in ONE file means the UI never
   touches Tauri directly: easy to mock in tests / Storybook,
   and a single place to evolve the contract.
   ============================================================ */

import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import type {
  Character,
  NewCharacterInput,
  ChatMessage,
  HistoryItem,
  ModelSettings,
  EndpointTestResult,
  Persona,
  NewPersonaInput,
  Group,
  NewGroupInput,
} from './types';

/**
 * The backend returns `avatar` as an absolute filesystem path (or null). The
 * webview can't load a raw path, so convert it to an `asset:` URL. Anything
 * that already looks like a URL (data:/http:/asset:) is left untouched.
 */
// Turn a backend absolute path into a loadable asset: URL. Values that are
// already URLs (data:/blob:/http(s):/asset:/tauri:) are returned untouched — and
// a Windows path like "C:\\Users\\..." must NOT be mistaken for a scheme.
function assetUrl<T extends string | null | undefined>(p: T): T {
  if (!p || /^(data|blob|https?|asset|tauri):/i.test(p)) return p;
  return convertFileSrc(p) as T;
}

function withAvatarUrl(c: Character): Character {
  return c.avatar ? { ...c, avatar: assetUrl(c.avatar) } : c;
}

// ---- Characters ----------------------------------------------------------

export async function listCharacters(): Promise<Character[]> {
  const cs = await invoke<Character[]>('list_characters');
  return cs.map(withAvatarUrl);
}

export async function getCharacter(id: string): Promise<Character> {
  return withAvatarUrl(await invoke<Character>('get_character', { id }));
}

export async function createCharacter(input: NewCharacterInput): Promise<Character> {
  return withAvatarUrl(await invoke<Character>('create_character', { input }));
}

export async function updateCharacter(id: string, input: NewCharacterInput): Promise<Character> {
  return withAvatarUrl(await invoke<Character>('update_character', { id, input }));
}

/** `excludeId` lets the edit form keep the character's own current name. */
export function characterNameAvailable(name: string, excludeId?: string): Promise<boolean> {
  return invoke('character_name_available', { name, excludeId: excludeId ?? null });
}

export function setFavorite(characterId: string, favorite: boolean): Promise<void> {
  return invoke('set_favorite', { characterId, favorite });
}

export function deleteCharacter(characterId: string): Promise<void> {
  return invoke('delete_character', { characterId });
}

// ---- Personas --------------------------------------------------------------

function withPersonaAvatarUrl(p: Persona): Persona {
  return p.avatar ? { ...p, avatar: assetUrl(p.avatar) } : p;
}

export async function listPersonas(): Promise<Persona[]> {
  const ps = await invoke<Persona[]>('list_personas');
  return ps.map(withPersonaAvatarUrl);
}

export async function createPersona(input: NewPersonaInput): Promise<Persona> {
  return withPersonaAvatarUrl(await invoke<Persona>('create_persona', { input }));
}

export async function updatePersona(id: string, input: NewPersonaInput): Promise<Persona> {
  return withPersonaAvatarUrl(await invoke<Persona>('update_persona', { id, input }));
}

export function deletePersona(personaId: string): Promise<void> {
  return invoke('delete_persona', { personaId });
}

/** The persona currently selected for this chat, or null. Persists across launches. */
export async function getActivePersona(conversationId: string): Promise<Persona | null> {
  const p = await invoke<Persona | null>('get_active_persona', { conversationId });
  return p ? withPersonaAvatarUrl(p) : null;
}

/** Select (or, with `null`, clear) the persona for this chat. */
export function setActivePersona(conversationId: string, personaId: string | null): Promise<void> {
  return invoke('set_active_persona', { conversationId, personaId });
}

// ---- Groups ------------------------------------------------------------

function withGroupAvatarUrl(g: Group): Group {
  return {
    ...g,
    avatar: g.avatar ? assetUrl(g.avatar) : g.avatar,
    members: g.members.map((m) => (m.avatar ? { ...m, avatar: assetUrl(m.avatar) } : m)),
  };
}

export async function listGroups(): Promise<Group[]> {
  const gs = await invoke<Group[]>('list_groups');
  return gs.map(withGroupAvatarUrl);
}

export async function createGroup(input: NewGroupInput): Promise<Group> {
  return withGroupAvatarUrl(await invoke<Group>('create_group', { input }));
}

// ---- History / conversations --------------------------------------------

export async function listHistory(): Promise<HistoryItem[]> {
  const items = await invoke<HistoryItem[]>('list_history');
  return items.map((h) => (h.avatar ? { ...h, avatar: assetUrl(h.avatar) } : h));
}

export function listMessages(conversationId: string): Promise<ChatMessage[]> {
  return invoke('list_messages', { conversationId });
}

export function deleteConversation(conversationId: string): Promise<void> {
  return invoke('delete_conversation', { conversationId });
}

/**
 * Send a user message and get the assistant's reply.
 *
 * This is the simple request/response form. For token streaming,
 * see `streamMessage` below and the README's events section.
 */
export function sendMessage(
  conversationId: string,
  content: string,
): Promise<ChatMessage> {
  return invoke('send_message', { conversationId, content });
}

export function deleteMessage(conversationId: string, messageId: string): Promise<void> {
  return invoke('delete_message', { conversationId, messageId });
}

export function rewindToMessage(conversationId: string, messageId: string): Promise<void> {
  return invoke('rewind_to_message', { conversationId, messageId });
}

export function updateMessage(messageId: string, content: string): Promise<void> {
  return invoke('update_message', { messageId, content });
}

// ---- Settings / model ----------------------------------------------------

export function getSettings(): Promise<ModelSettings> {
  return invoke('get_settings');
}

export function saveSettings(settings: ModelSettings): Promise<void> {
  return invoke('save_settings', { settings });
}

export function testEndpoint(endpoint: string): Promise<EndpointTestResult> {
  return invoke('test_endpoint', { endpoint });
}

/** Models currently loaded on the server (LM Studio). */
export function loadedModels(endpoint: string): Promise<string[]> {
  return invoke('loaded_models', { endpoint });
}

/** Unload a model on the server (via the lms CLI). */
export function unloadModel(model: string): Promise<void> {
  return invoke('unload_model', { model });
}

export function loadModel(settings: ModelSettings): Promise<void> {
  return invoke('load_model', { settings });
}

// ---- Streaming (optional) -----------------------------------------------
//
// For live token streaming, have the Rust command emit events
// (e.g. `chat://token` and `chat://done`) and listen with
// `@tauri-apps/api/event`'s `listen`. Example consumer:
//
//   import { listen } from '@tauri-apps/api/event';
//   const un = await listen<string>('chat://token', e => append(e.payload));
//
// `streamMessage` just kicks off the backend job; tokens arrive via events.
export function streamMessage(
  conversationId: string,
  content: string,
): Promise<void> {
  return invoke('stream_message', { conversationId, content });
}

/**
 * Empty "continue" send: no user content. If the last message is the AI's, it
 * continues via a hidden technical message; if the last is the user's, it just
 * replies. Tokens arrive via the same `chat://*` events as `streamMessage`.
 */
export function streamContinue(conversationId: string): Promise<void> {
  return invoke('stream_continue', { conversationId });
}
