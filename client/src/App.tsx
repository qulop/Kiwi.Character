import { useEffect, useState } from 'react';
import type { Character, HistoryItem, ModelSettings, NewCharacterInput } from './types';
import { DEFAULT_SETTINGS } from './types';
import * as api from './api';
import Sidebar from './components/Sidebar';
import MainPage from './components/MainPage';
import ChatPage from './components/ChatPage';
import SettingsModal from './components/SettingsModal';
import CharacterFormModal from './components/CharacterFormModal';
import { NewCharIcon, InfoIcon } from './components/icons';

type Page = 'main' | 'chat';
type Modal = 'settings' | 'new' | 'info' | null;

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
  const [modal, setModal] = useState<Modal>(null);
  const [search, setSearch] = useState('');

  const [characters, setCharacters] = useState<Character[]>([]);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [settings, setSettings] = useState<ModelSettings>(DEFAULT_SETTINGS);

  const [activeCharacter, setActiveCharacter] = useState<Character | null>(null);
  const [conversationId, setConversationId] = useState<string>('');

  // Initial load from the Rust backend.
  useEffect(() => {
    api.listCharacters().then(setCharacters).catch(console.error);
    api.listHistory().then(setHistory).catch(console.error);
    api.getSettings().then(setSettings).catch(() => { /* keep defaults */ });
  }, []);

  const openCharacter = (c: Character) => {
    setActiveCharacter(c);
    // A real app maps a character to (or creates) a conversation id here.
    setConversationId('conv-' + c.id);
    setPage('chat');
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
    setModal(null);
    openCharacter(created);
  };

  const updateCharacter = async (input: NewCharacterInput) => {
    if (!activeCharacter) return;
    // Throws on collision so the modal stays open and shows the error.
    const updated = await api.updateCharacter(activeCharacter.id, input);
    setCharacters((cs) => cs.map((x) => (x.id === updated.id ? updated : x)));
    setActiveCharacter(updated);
    setModal(null);
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

  const sidebar = (
    <Sidebar
      history={history}
      activeId={page === 'chat' ? conversationId : null}
      search={search}
      onSearch={setSearch}
      onBrand={() => setPage('main')}
      onCreate={() => setModal('new')}
      onSettings={() => setModal('settings')}
      onSelect={openHistory}
    />
  );

  return (
    <>
      {page === 'main' || !activeCharacter ? (
        <MainPage
          sidebar={sidebar}
          characters={characters}
          onOpenCharacter={openCharacter}
          onToggleFavorite={toggleFavorite}
          onDeleteCharacter={deleteCharacter}
        />
      ) : (
        <ChatPage
          sidebar={sidebar}
          character={activeCharacter}
          conversationId={conversationId}
          onOpenInfo={() => setModal('info')}
        />
      )}

      {modal && (
        <div className="kc-overlay" onClick={() => setModal(null)}>
          {modal === 'settings' && (
            <SettingsModal
              initial={settings}
              onClose={() => setModal(null)}
              onTest={api.testEndpoint}
              onLoad={(s) => { setSettings(s); api.saveSettings(s).catch(console.error); api.loadModel(s).catch(console.error); setModal(null); }}
            />
          )}
          {modal === 'new' && (
            <CharacterFormModal
              title="New Character"
              icon={<NewCharIcon />}
              submitLabel="Create Character"
              initial={EMPTY_INPUT}
              onClose={() => setModal(null)}
              onSubmit={createCharacter}
            />
          )}
          {modal === 'info' && activeCharacter && (
            <CharacterFormModal
              title="Character Info"
              icon={<InfoIcon />}
              submitLabel="Save"
              initial={characterToInput(activeCharacter)}
              excludeId={activeCharacter.id}
              onClose={() => setModal(null)}
              onSubmit={updateCharacter}
            />
          )}
        </div>
      )}
    </>
  );
}
