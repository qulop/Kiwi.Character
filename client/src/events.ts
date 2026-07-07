/* ============================================================
   Tauri event names for chat streaming.

   The Rust `stream_message` command emits these while a reply is
   being generated. Keep these in sync with the string literals in
   `src-tauri/src/commands.rs`.
   ============================================================ */

/** One content delta (token/chunk) of the assistant's reply. Payload: string. */
export const CHAT_TOKEN = 'chat://token';

/** The reply finished streaming successfully. Payload: none. */
export const CHAT_DONE = 'chat://done';

/** Generation failed. Payload: an error message string. */
export const CHAT_ERROR = 'chat://error';
