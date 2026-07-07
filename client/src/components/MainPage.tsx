import React, { useState } from 'react';
import type { Character } from '../types';
import { initialOf } from '../types';

interface MainPageProps {
  sidebar: React.ReactNode;
  characters: Character[];
  onOpenCharacter: (c: Character) => void;
}

const FILTERS = ['All', 'Recent', 'Favorites'] as const;

/** Character pickup screen — the grid of character cards. */
export default function MainPage({ sidebar, characters, onOpenCharacter }: MainPageProps) {
  const [filter, setFilter] = useState<(typeof FILTERS)[number]>('All');

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
          <div className="kc-grid">
            {characters.map((c) => (
              <button key={c.id} className="kc-card" onClick={() => onOpenCharacter(c)}>
                <div className="kc-card-avatar">
                  {c.avatar ? (
                    <img src={c.avatar} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: '30%' }} />
                  ) : (
                    initialOf(c.name)
                  )}
                </div>
                <div className="kc-card-name">{c.name}</div>
                <div className="kc-card-info">{c.info}</div>
              </button>
            ))}
          </div>
        </div>
      </main>
    </div>
  );
}
