import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { Character, ChatMessage } from '../types';
import { initialOf } from '../types';
import * as api from '../api';
import { CHAT_TOKEN, CHAT_DONE, CHAT_ERROR } from '../events';
import { SendIcon, UserGlyph, PersonaGlyph } from './icons';

interface ChatPageProps {
  sidebar: React.ReactNode;
  character: Character;
  conversationId: string;
}

/** Conversation screen — message thread, composer, character panel. */
export default function ChatPage({ sidebar, character, conversationId }: ChatPageProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [typing, setTyping] = useState(false);
  const [draft, setDraft] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);

  const scrollDown = () =>
    requestAnimationFrame(() => {
      const el = scrollRef.current;
      if (el) el.scrollTop = el.scrollHeight;
    });

  // Load the conversation when the active character / conversation changes.
  useEffect(() => {
    let alive = true;
    api.listMessages(conversationId).then((msgs) => {
      if (alive) { setMessages(msgs); scrollDown(); }
    });
    return () => { alive = false; };
  }, [conversationId]);

  const send = async () => {
    const content = draft.trim();
    if (!content || typing) return;
    setDraft('');

    // Optimistic user bubble + an empty assistant bubble we stream tokens into.
    const userMsg: ChatMessage = {
      id: 'local-' + Date.now(),
      role: 'user',
      content,
      createdAt: Date.now(),
    };
    const assistantId = 'stream-' + Date.now();
    setMessages((m) => [
      ...m,
      userMsg,
      { id: assistantId, role: 'assistant', content: '', createdAt: Date.now() },
    ]);
    setTyping(true);
    scrollDown();

    const unlisten: Array<() => void> = [];
    const cleanup = () => {
      unlisten.forEach((u) => u());
      setTyping(false);
      scrollDown();
    };

    // Append each token to the assistant bubble.
    unlisten.push(
      await listen<string>(CHAT_TOKEN, (e) => {
        setMessages((m) =>
          m.map((x) => (x.id === assistantId ? { ...x, content: x.content + e.payload } : x)),
        );
        scrollDown();
      }),
    );
    unlisten.push(await listen(CHAT_DONE, cleanup));
    unlisten.push(
      await listen<string>(CHAT_ERROR, (e) => {
        setMessages((m) =>
          m.map((x) => (x.id === assistantId ? { ...x, content: `⚠️ ${e.payload}` } : x)),
        );
        cleanup();
      }),
    );

    try {
      await api.streamMessage(conversationId, content);
    } catch {
      // The error is surfaced via the CHAT_ERROR event; if the invoke itself
      // rejected before any event fired, make sure we stop the typing state.
      cleanup();
    }
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') { e.preventDefault(); send(); }
  };

  // Show the typing dots only until the first token arrives (the streaming
  // assistant bubble is the last message and still empty).
  const last = messages[messages.length - 1];
  const awaitingFirstToken =
    typing && last?.role === 'assistant' && last.content === '';

  return (
    <div className="kc-app">
      {sidebar}

      <main className="kc-main">
        <div className="kc-chat-head">
          <div className="kc-avatar kc-chat-head-avatar">{initialOf(character.name)}</div>
          <div className="kc-chat-name">{character.name}</div>
          <div className="kc-status"><span className="kc-status-dot" /> local model</div>
        </div>

        <div className="kc-messages-wrap" ref={scrollRef}>
          <div className="kc-messages">
            {messages.map((m) => (
              m.role === 'assistant' ? (
                // Skip the empty placeholder bubble while the reply is still
                // streaming — the typing indicator stands in for it.
                m.content === '' ? null : (
                <div key={m.id} className="kc-msg-row">
                  <div className="kc-avatar kc-msg-avatar">{initialOf(character.name)}</div>
                  <div className="kc-msg-col">
                    <span className="kc-msg-author">{character.name}</span>
                    <div className="kc-bubble">{m.content}</div>
                  </div>
                </div>
                )
              ) : (
                <div key={m.id} className="kc-msg-row user">
                  <div className="kc-avatar kc-avatar--user kc-msg-avatar"><UserGlyph /></div>
                  <div className="kc-msg-col">
                    <span className="kc-msg-author">User</span>
                    <div className="kc-bubble user">{m.content}</div>
                  </div>
                </div>
              )
            ))}

            {awaitingFirstToken && (
              <div className="kc-msg-row">
                <div className="kc-avatar kc-msg-avatar">{initialOf(character.name)}</div>
                <div className="kc-msg-col">
                  <span className="kc-msg-author">{character.name}</span>
                  <div className="kc-typing"><span /><span /><span /></div>
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="kc-composer">
          <input
            placeholder={`Message ${character.name}…`}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={onKey}
          />
          <button className="kc-send" onClick={send} disabled={!draft.trim() || typing}>
            <SendIcon />
          </button>
        </div>
      </main>

      <aside className="kc-rpanel">
        <div className="kc-rpanel-head">
          <div className="kc-avatar kc-rpanel-avatar">{initialOf(character.name)}</div>
          <div className="kc-rpanel-name">{character.name}</div>
        </div>
        <div className="kc-rpanel-info">{character.info}</div>
        <div className="kc-divider" style={{ margin: '2px 0' }} />
        <div className="kc-section-label" style={{ padding: 0 }}>Personas</div>
        <button className="kc-personas-btn" disabled>
          <PersonaGlyph /> Personas
        </button>
        <div className="kc-personas-hint">Persona selection — coming soon</div>
      </aside>
    </div>
  );
}
