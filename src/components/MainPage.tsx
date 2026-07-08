import React, { useEffect, useState } from 'react';
import type { Character } from '../types';
import { initialOf } from '../types';
import { DotsIcon, HeartIcon, HeartFilledIcon, CrossIcon, PenIcon } from './icons';

interface MainPageProps {
  sidebar: React.ReactNode;
  characters: Character[];
  onOpenCharacter: (c: Character) => void;
  onEditCharacter: (c: Character) => void;
  onToggleFavorite: (c: Character) => void;
  onDeleteCharacter: (c: Character) => void;
}

const FILTERS = ['All', 'Recent', 'Favourites'] as const;

/** "Recent" = created or spoken-with within this window. */
const RECENT_WINDOW_MS = 3 * 24 * 60 * 60 * 1000; // 3 days

const EMPTY_MESSAGE: Record<(typeof FILTERS)[number], string> = {
  All: 'No characters yet — click “+ Create” to add one.',
  Recent: 'Nothing recent. Start a chat or create a character.',
  Favourites: 'No favourites yet. Mark a character with the heart.',
};

/** Character pickup screen — the grid of character cards. */
export default function MainPage({
  sidebar,
  characters,
  onOpenCharacter,
  onEditCharacter,
  onToggleFavorite,
  onDeleteCharacter,
}: MainPageProps) {
  const [filter, setFilter] = useState<(typeof FILTERS)[number]>('All');
  const [menuFor, setMenuFor] = useState<string | null>(null);

  // Close an open card menu on any outside click / Escape.
  useEffect(() => {
    if (!menuFor) return;
    const close = () => setMenuFor(null);
    document.addEventListener('click', close);
    document.addEventListener('keydown', close);
    return () => {
      document.removeEventListener('click', close);
      document.removeEventListener('keydown', close);
    };
  }, [menuFor]);

  const now = Date.now();
  const visible = characters.filter((c) => {
    if (filter === 'All') return true;
    if (filter === 'Favourites') return !!c.isFavorite;
    // Recent: created OR last spoken-with within the window.
    const created = c.createdAt ?? 0;
    const spoke = c.lastMessageAt ?? 0;
    return now - created <= RECENT_WINDOW_MS || now - spoke <= RECENT_WINDOW_MS;
  });

  return (
    <div className="kc-app">
      {sidebar}

      <main className="kc-main">
        <div className="kc-main-head">
          <div>
            <div className="kc-title">Characters</div>
            <div className="kc-subtitle">Pick someone to talk with</div>
          </div>
          <div className="kc-filters">
            {FILTERS.map((f) => (
              <button
                key={f}
                className={'kc-chip' + (f === filter ? ' active' : '')}
                onClick={() => setFilter(f)}
              >
                {f}
              </button>
            ))}
          </div>
        </div>

        <div className="kc-grid-wrap">
          {visible.length === 0 && <div className="kc-empty">{EMPTY_MESSAGE[filter]}</div>}
          <div className="kc-grid">
            {visible.map((c) => (
              <div
                key={c.id}
                className="kc-card"
                role="button"
                tabIndex={0}
                onClick={() => onOpenCharacter(c)}
                onKeyDown={(e) => { if (e.key === 'Enter') onOpenCharacter(c); }}
              >
                {c.isFavorite && (
                  <span className="kc-card-fav" aria-label="Favourite"><HeartFilledIcon /></span>
                )}

                <div className="kc-card-avatar">
                  {c.avatar ? (
                    <img src={c.avatar} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: '30%' }} />
                  ) : (
                    initialOf(c.name)
                  )}
                </div>
                <div className="kc-card-name">{c.name}</div>
                <div className="kc-card-info">{c.info}</div>

                <button
                  className="kc-card-dots"
                  aria-label="More actions"
                  onClick={(e) => {
                    e.stopPropagation();
                    setMenuFor(menuFor === c.id ? null : c.id);
                  }}
                >
                  <DotsIcon />
                </button>

                {menuFor === c.id && (
                  <div className="kc-card-menu" onClick={(e) => e.stopPropagation()}>
                    <button onClick={() => { onToggleFavorite(c); setMenuFor(null); }}>
                      {c.isFavorite ? <HeartFilledIcon /> : <HeartIcon />}
                      <span>{c.isFavorite ? 'Remove favourite' : 'Mark as favourite'}</span>
                    </button>
                    <button onClick={() => { onEditCharacter(c); setMenuFor(null); }}>
                      <PenIcon />
                      <span>Edit</span>
                    </button>
                    <button className="danger" onClick={() => { onDeleteCharacter(c); setMenuFor(null); }}>
                      <CrossIcon />
                      <span>Delete</span>
                    </button>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </main>
    </div>
  );
}
