import { useEffect, useState } from 'react';
import type {
  Character,
  HistoryItem,
  ModelSettings,
  NewCharacterInput,
  Persona,
  NewPersonaInput,
} from './types';
import { DEFAULT_SETTINGS } from './types';
import * as api from './api';
import Sidebar from './components/Sidebar';
import MainPage from './components/MainPage';
import ChatPage from './components/ChatPage';
import SettingsModal from './components/SettingsModal';
import CharacterFormModal from './components/CharacterFormModal';
import PersonasModal from './components/PersonasModal';
import PersonaFormModal from './components/PersonaFormModal';
import { NewCharIcon, InfoIcon } from './components/icons';

type Page = 'main' | 'chat';
type ModalKind = 'settings' | 'new' | 'info' | 'personas' | 'new-persona';

const EMPTY_INPUT: NewCharacterInput = {
  name: '', info: '', appearance: '', description: '', initialMessage: '', avatar: null,
};

const characterToInput = (c: Character): NewCharacterInput => ({
  name: c.name,
  info: c.info,
  appearance: c.appearance ?? '',
  description: c.description ?? '',
  initialMessage: c.initialMessage ?? '',
  avatar: c.avatar ?? null,
});

export default function App() {
  const [page, setPage] = useState<Page>('main');
  // A stack (not a single value) so an overlapping pop-up — e.g. "New Persona"
  // opened from "Personas" — remembers what's underneath it and can return to
  // it (back arrow) instead of only ever closing everything.
  const [modalStack, setModalStack] = useState<ModalKind[]>([]);
  const modal = modalStack[modalStack.length - 1] ?? null;
  const openModal = (m: ModalKind) => setModalStack((s) => [...s, m]);
  const closeModals = () => setModalStack([]);
  const backModal = () => setModalStack((s) => s.slice(0, -1));
  const [search, setSearch] = useState('');

  const [characters, setCharacters] = useState<Character[]>([]);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [personas, setPersonas] = useState<Persona[]>([]);
  const [settings, setSettings] = useState<ModelSettings>(DEFAULT_SETTINGS);
  const [endpointStatus, setEndpointStatus] = useState<{
    online: boolean;
    models: string[];
    loaded: string[];
  }>({ online: false, models: [], loaded: [] });

  const [activeCharacter, setActiveCharacter] = useState<Character | null>(null);
  const [conversationId, setConversationId] = useState<string>('');
  const [editingCharacter, setEditingCharacter] = useState<Character | null>(null);

  // Initial load from the Rust backend.
  useEffect(() => {
    api.listCharacters().then(setCharacters).catch(console.error);
    api.listHistory().then(setHistory).catch(console.error);
    api.listPersonas().then(setPersonas).catch(console.error);
    api.getSettings().then(setSettings).catch(() => { /* keep defaults */ });
  }, []);

  // Remember the last endpoint and keep its online status fresh by pinging it
  // now and every 45s (and whenever the saved endpoint changes). This lets the
  // Settings modal open already-unlocked when the endpoint is reachable.
  useEffect(() => {
    let alive = true;
    const ping = async () => {
      try {
        const res = await api.testEndpoint(settings.endpoint);
        let loaded: string[] = [];
        if (res.ok) {
          // Best-effort: the loaded-model API is LM Studio specific.
          try { loaded = await api.loadedModels(settings.endpoint); } catch { /* ignore */ }
        }
        if (alive) setEndpointStatus({ online: res.ok, models: res.models ?? [], loaded });
      } catch {
        if (alive) setEndpointStatus((prev) => ({ ...prev, online: false }));
      }
    };
    ping();
    const id = window.setInterval(ping, 45000);
    return () => { alive = false; clearInterval(id); };
  }, [settings.endpoint]);

  const openCharacter = (c: Character) => {
    setActiveCharacter(c);
    const convId = 'conv-' + c.id;
    setConversationId(convId);
    setPage('chat');
    // Surface the conversation in history right away (no reload needed).
    setHistory((h) =>
      h.some((x) => x.id === convId)
        ? h
        : [
            { id: convId, characterId: c.id, name: c.name, avatar: c.avatar ?? null, lastMessageAt: Date.now() },
            ...h,
          ],
    );
  };

  const openCharacterInfo = (c: Character) => {
    setEditingCharacter(c);
    openModal('info');
  };

  const createPersona = async (input: NewPersonaInput) => {
    const created = await api.createPersona(input);
    setPersonas((ps) => [created, ...ps]);
    // Return to the Personas list (not a full close) so the new card is visible.
    backModal();
  };

  const deletePersona = async (p: Persona) => {
    setPersonas((ps) => ps.filter((x) => x.id !== p.id));
    try {
      await api.deletePersona(p.id);
    } catch (e) {
      console.error(e);
    }
  };

  const openHistory = (h: HistoryItem) => {
    const c = characters.find((x) => x.id === h.characterId);
    if (c) setActiveCharacter(c);
    setConversationId(h.id);
    setPage('chat');
  };

  const createCharacter = async (input: NewCharacterInput) => {
    // Throws on failure (e.g. a duplicate name) so the modal can keep itself
    // open and show the error; on success we jump straight into the new chat.
    const created = await api.createCharacter(input);
    setCharacters((cs) => [created, ...cs]);
    closeModals();
    openCharacter(created);
  };

  const updateCharacter = async (input: NewCharacterInput) => {
    if (!editingCharacter) return;
    // Throws on collision so the modal stays open and shows the error.
    const updated = await api.updateCharacter(editingCharacter.id, input);
    setCharacters((cs) => cs.map((x) => (x.id === updated.id ? updated : x)));
    if (activeCharacter?.id === updated.id) setActiveCharacter(updated);
    // Keep the history row's name/avatar in sync with the edit.
    setHistory((h) =>
      h.map((x) =>
        x.characterId === updated.id ? { ...x, name: updated.name, avatar: updated.avatar ?? null } : x,
      ),
    );
    closeModals();
    setEditingCharacter(null);
  };

  const toggleFavorite = async (c: Character) => {
    const next = !c.isFavorite;
    setCharacters((cs) => cs.map((x) => (x.id === c.id ? { ...x, isFavorite: next } : x)));
    if (activeCharacter?.id === c.id) setActiveCharacter({ ...activeCharacter, isFavorite: next });
    try {
      await api.setFavorite(c.id, next);
    } catch (e) {
      console.error(e);
    }
  };

  const deleteCharacter = async (c: Character) => {
    setCharacters((cs) => cs.filter((x) => x.id !== c.id));
    setHistory((h) => h.filter((x) => x.characterId !== c.id));
    if (activeCharacter?.id === c.id) {
      setActiveCharacter(null);
      setPage('main');
    }
    try {
      await api.deleteCharacter(c.id);
    } catch (e) {
      console.error(e);
    }
  };

  // Bump a conversation to the top of history (into "Today") as the user chats.
  const onChatActivity = (convId: string) => {
    setHistory((h) => {
      const idx = h.findIndex((x) => x.id === convId);
      if (idx === -1) return h; // openCharacter already inserts it
      const item = { ...h[idx], lastMessageAt: Date.now() };
      return [item, ...h.filter((_, i) => i !== idx)];
    });
  };

  const deleteChat = async (h: HistoryItem) => {
    setHistory((list) => list.filter((x) => x.id !== h.id));
    if (conversationId === h.id) {
      setActiveCharacter(null);
      setPage('main');
    }
    try {
      await api.deleteConversation(h.id);
    } catch (e) {
      console.error(e);
    }
  };

  const sidebar = (
    <Sidebar
      history={history}
      activeId={page === 'chat' ? conversationId : null}
      search={search}
      onSearch={setSearch}
      onBrand={() => setPage('main')}
      onCreate={() => openModal('new')}
      onSettings={() => openModal('settings')}
      onSelect={openHistory}
      onDeleteChat={deleteChat}
    />
  );

  return (
    <>
      {page === 'main' || !activeCharacter ? (
        <MainPage
          sidebar={sidebar}
          characters={characters}
          onOpenCharacter={openCharacter}
          onEditCharacter={openCharacterInfo}
          onToggleFavorite={toggleFavorite}
          onDeleteCharacter={deleteCharacter}
        />
      ) : (
        <ChatPage
          sidebar={sidebar}
          character={activeCharacter}
          conversationId={conversationId}
          onOpenInfo={() => openCharacterInfo(activeCharacter)}
          onToggleFavorite={() => toggleFavorite(activeCharacter)}
          onActivity={onChatActivity}
          onOpenPersonas={() => openModal('personas')}
        />
      )}

      {modal && (
        // Backdrop no longer closes the modal — use the close (×) button.
        <div className="kc-overlay">
          {modal === 'settings' && (
            <SettingsModal
              initial={settings}
              initialVerified={endpointStatus.online}
              initialModels={endpointStatus.models}
              initialLoaded={endpointStatus.loaded}
              onClose={closeModals}
              onTest={api.testEndpoint}
              onLoad={async (s) => { setSettings(s); await api.loadModel(s); }}
              onRefreshLoaded={api.loadedModels}
              onUnload={api.unloadModel}
              onSave={async (s) => { setSettings(s); await api.saveSettings(s); }}
            />
          )}
          {modal === 'new' && (
            <CharacterFormModal
              title="New Character"
              icon={<NewCharIcon />}
              submitLabel="Create Character"
              initial={EMPTY_INPUT}
              onClose={closeModals}
              onSubmit={createCharacter}
            />
          )}
          {modal === 'info' && editingCharacter && (
            <CharacterFormModal
              title="Character Info"
              icon={<InfoIcon />}
              submitLabel="Save"
              initial={characterToInput(editingCharacter)}
              excludeId={editingCharacter.id}
              onClose={() => { closeModals(); setEditingCharacter(null); }}
              onSubmit={updateCharacter}
            />
          )}
          {modal === 'personas' && (
            <PersonasModal
              personas={personas}
              onClose={closeModals}
              onCreate={() => openModal('new-persona')}
              onDelete={deletePersona}
            />
          )}
          {modal === 'new-persona' && (
            <PersonaFormModal onBack={backModal} onSubmit={createPersona} />
          )}
        </div>
      )}
    </>
  );
}
