import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { Character, ChatMessage } from '../types';
import * as api from '../api';
import { CHAT_TOKEN, CHAT_DONE, CHAT_ERROR } from '../events';
import Avatar from './Avatar';
import {
  SendIcon,
  UserGlyph,
  PersonaGlyph,
  DotsIcon,
  CopyIcon,
  TrashIcon,
  RewindIcon,
  PenIcon,
} from './icons';

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
  const [menuMsg, setMenuMsg] = useState<string | null>(null);
  const [menuUp, setMenuUp] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState('');
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

  // Close an open per-message menu on any outside click / Escape.
  useEffect(() => {
    if (!menuMsg) return;
    const close = () => setMenuMsg(null);
    document.addEventListener('click', close);
    document.addEventListener('keydown', close);
    return () => {
      document.removeEventListener('click', close);
      document.removeEventListener('keydown', close);
    };
  }, [menuMsg]);

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
    unlisten.push(
      await listen(CHAT_DONE, () => {
        cleanup();
        // Reload so messages carry real DB ids (needed for copy/remove/rewind).
        api.listMessages(conversationId).then((msgs) => { setMessages(msgs); scrollDown(); }).catch(() => {});
      }),
    );
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

  const copyMessage = (m: ChatMessage) => {
    navigator.clipboard.writeText(m.content).catch(() => {});
  };

  const removeMessage = async (m: ChatMessage) => {
    setMessages((list) => list.filter((x) => x.id !== m.id));
    try { await api.deleteMessage(conversationId, m.id); } catch (e) { console.error(e); }
  };

  const rewindTo = async (m: ChatMessage) => {
    // Keep up to and including the selected message; drop everything after it.
    setMessages((list) => {
      const i = list.findIndex((x) => x.id === m.id);
      return i === -1 ? list : list.slice(0, i + 1);
    });
    try { await api.rewindToMessage(conversationId, m.id); } catch (e) { console.error(e); }
  };

  const startEdit = (m: ChatMessage) => { setEditingId(m.id); setEditDraft(m.content); };
  const cancelEdit = () => { setEditingId(null); setEditDraft(''); };
  const saveEdit = async (m: ChatMessage) => {
    const content = editDraft.trim();
    if (!content) return;
    setMessages((list) => list.map((x) => (x.id === m.id ? { ...x, content } : x)));
    setEditingId(null);
    try { await api.updateMessage(m.id, content); } catch (e) { console.error(e); }
  };

  // Per-message three-dots menu (rendered inside each message column).
  const renderMenu = (m: ChatMessage) => (
    <div className="kc-msg-actions">
      <button
        className="kc-msg-dots"
        aria-label="Message actions"
        onClick={(e) => {
          e.stopPropagation();
          if (menuMsg === m.id) { setMenuMsg(null); return; }
          // Flip the menu upward if there isn't room below the button.
          const rect = e.currentTarget.getBoundingClientRect();
          const items = m.role === 'user' ? 4 : 2;
          const estHeight = items * 36 + 16;
          setMenuUp(window.innerHeight - rect.bottom < estHeight + 12);
          setMenuMsg(m.id);
        }}
      >
        <DotsIcon />
      </button>
      {menuMsg === m.id && (
        <div
          className={'kc-msg-menu' + (menuUp ? ' kc-msg-menu--up' : '')}
          onClick={(e) => e.stopPropagation()}
        >
          <button onClick={() => { copyMessage(m); setMenuMsg(null); }}><CopyIcon /> <span>Copy</span></button>
          {m.role === 'user' && (
            <button onClick={() => { startEdit(m); setMenuMsg(null); }}><PenIcon /> <span>Edit</span></button>
          )}
          {m.role === 'user' && (
            <button onClick={() => { rewindTo(m); setMenuMsg(null); }}><RewindIcon /> <span>Rewind to here</span></button>
          )}
          <button className="danger" onClick={() => { removeMessage(m); setMenuMsg(null); }}><TrashIcon /> <span>Remove</span></button>
        </div>
      )}
    </div>
  );

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
          <Avatar character={character} className="kc-chat-head-avatar" />
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
                  <Avatar character={character} className="kc-msg-avatar" />
                  <div className="kc-msg-col">
                    <div className="kc-msg-top">
                      <span className="kc-msg-author">{character.name}</span>
                      {renderMenu(m)}
                    </div>
                    <div className="kc-bubble">{m.content}</div>
                  </div>
                </div>
                )
              ) : (
                <div key={m.id} className="kc-msg-row user">
                  <div className="kc-avatar kc-avatar--user kc-msg-avatar"><UserGlyph /></div>
                  <div className="kc-msg-col">
                    <div className="kc-msg-top">
                      <span className="kc-msg-author">User</span>
                      {renderMenu(m)}
                    </div>
                    {editingId === m.id ? (
                      <div className="kc-bubble user kc-bubble-edit">
                        <textarea
                          className="kc-edit-area"
                          value={editDraft}
                          onChange={(e) => setEditDraft(e.target.value)}
                          autoFocus
                        />
                        <div className="kc-edit-actions">
                          <button className="kc-edit-save" onClick={() => saveEdit(m)}>Save</button>
                          <button className="kc-edit-cancel" onClick={cancelEdit}>Cancel</button>
                        </div>
                      </div>
                    ) : (
                      <div className="kc-bubble user">{m.content}</div>
                    )}
                  </div>
                </div>
              )
            ))}

            {awaitingFirstToken && (
              <div className="kc-msg-row">
                <Avatar character={character} className="kc-msg-avatar" />
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
          <Avatar character={character} className="kc-rpanel-avatar" />
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
