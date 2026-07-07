import type { HistoryItem } from '../types';
import { initialOf } from '../types';
import { SearchIcon } from './icons';

interface SidebarProps {
  history: HistoryItem[];
  activeId?: string | null;
  search: string;
  onSearch: (value: string) => void;
  onBrand: () => void;
  onCreate: () => void;
  onSettings: () => void;
  onSelect: (item: HistoryItem) => void;
}

/** Left navigation rail — identical on the Main and Chat pages. */
export default function Sidebar({
  history,
  activeId,
  search,
  onSearch,
  onBrand,
  onCreate,
  onSettings,
  onSelect,
}: SidebarProps) {
  const filtered = search.trim()
    ? history.filter((h) => h.name.toLowerCase().includes(search.trim().toLowerCase()))
    : history;

  return (
    <aside className="kc-side">
      <button className="kc-brand" onClick={onBrand}>
        <div className="kc-logo">K</div>
        <span className="kc-brand-name">Kiwi.Character</span>
      </button>

      <button className="kc-create-btn" onClick={onCreate}>
        <span className="plus">+</span> Create
      </button>

      <div className="kc-search">
        <SearchIcon />
        <input
          placeholder="Search history…"
          value={search}
          onChange={(e) => onSearch(e.target.value)}
        />
      </div>

      <div className="kc-section-label">History</div>

      <div className="kc-history">
        {filtered.map((h) => (
          <button
            key={h.id}
            className={'kc-hist-item' + (h.id === activeId ? ' active' : '')}
            onClick={() => onSelect(h)}
          >
            <div className="kc-avatar kc-hist-avatar">{initialOf(h.name)}</div>
            <span className="kc-hist-name">{h.name}</span>
            <span
              className="kc-hist-more"
              role="button"
              onClick={(e) => e.stopPropagation()}
            >
              ⋯
            </span>
          </button>
        ))}
      </div>

      <button className="kc-settings-btn" onClick={onSettings}>
        <span style={{ fontSize: 17 }}>⚙︎</span> Settings
      </button>
    </aside>
  );
}
