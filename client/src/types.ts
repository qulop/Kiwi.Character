/* ============================================================
   Shared domain types — mirror these in your Rust structs
   (serde-serialized) so `invoke` payloads line up.
   ============================================================ */

export interface Character {
  id: string;
  name: string;
  /** Short blurb shown under the name on cards. */
  info: string;
  /** Optional avatar image (data URL, file path, or asset URL). */
  avatar?: string | null;
  appearance?: string;
  description?: string;
  initialMessage?: string;
  /** Whether the user marked this character as a favourite. */
  isFavorite?: boolean;
  /** ms epoch the character was created. */
  createdAt?: number;
  /** ms epoch of the newest message with this character, or null if none. */
  lastMessageAt?: number | null;
}

/** Payload for creating a new character (no id yet). */
export interface NewCharacterInput {
  name: string;
  info: string;
  appearance: string;
  description: string;
  initialMessage: string;
  /** Base64 / data URL of the chosen image, if any. */
  avatar?: string | null;
}

export type Role = 'user' | 'assistant';

export interface ChatMessage {
  id: string;
  role: Role;
  content: string;
  /** ms epoch — optional, handy for ordering / display. */
  createdAt?: number;
}

/** A row in the sidebar "History" list. */
export interface HistoryItem {
  /** Conversation id. */
  id: string;
  characterId: string;
  name: string;
}

export interface ModelSettings {
  endpoint: string;
  model: string;
  contextLength: number;
  gpuOffload: number;
  temperature: number;
  maxTokens: number;
  systemPrompt: string;
}

export interface EndpointTestResult {
  ok: boolean;
  /** Models the endpoint reports as available. */
  models: string[];
  error?: string;
}

export const DEFAULT_SETTINGS: ModelSettings = {
  endpoint: 'http://localhost:1234/v1',
  model: 'llama-3.1-8b-instruct',
  contextLength: 100,
  gpuOffload: 60,
  temperature: 0.8,
  maxTokens: 2048,
  systemPrompt: '',
};

/** First letter for the monogram avatar (ignores a leading "The "). */
export function initialOf(name: string): string {
  return (name.replace(/^The /, '')[0] || '?').toUpperCase();
}
