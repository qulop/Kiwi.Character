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
} from './types';

/**
 * The backend returns `avatar` as an absolute filesystem path (or null). The
 * webview can't load a raw path, so convert it to an `asset:` URL. Anything
 * that already looks like a URL (data:/http:/asset:) is left untouched.
 */
function withAvatarUrl(c: Character): Character {
  if (!c.avatar) return c;
  // Skip values that are already loadable URLs. NB: a Windows path like
  // "C:\\Users\\..." must NOT be treated as a scheme — only these real ones.
  if (/^(data|blob|https?|asset|tauri):/i.test(c.avatar)) return c;
  return { ...c, avatar: convertFileSrc(c.avatar) };
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

export function characterNameAvailable(name: string): Promise<boolean> {
  return invoke('character_name_available', { name });
}

export function setFavorite(characterId: string, favorite: boolean): Promise<void> {
  return invoke('set_favorite', { characterId, favorite });
}

export function deleteCharacter(characterId: string): Promise<void> {
  return invoke('delete_character', { characterId });
}

// ---- History / conversations --------------------------------------------

export function listHistory(): Promise<HistoryItem[]> {
  return invoke('list_history');
}

export function listMessages(conversationId: string): Promise<ChatMessage[]> {
  return invoke('list_messages', { conversationId });
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
