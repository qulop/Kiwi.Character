import { useEffect, useState } from 'react';
import type { HistoryItem } from '../types';
import Avatar from './Avatar';
import { SearchIcon, TrashIcon } from './icons';

interface SidebarProps {
  history: HistoryItem[];
  activeId?: string | null;
  search: string;
  onSearch: (value: string) => void;
  onBrand: () => void;
  onCreate: () => void;
  onSettings: () => void;
  onSelect: (item: HistoryItem) => void;
  onDeleteChat: (item: HistoryItem) => void;
}

const BUCKET_LABELS = [
  'Today',
  'Yesterday',
  'This week',
  'This month',
  'This year',
  'A while ago',
] as const;

const DAY_MS = 24 * 60 * 60 * 1000;

/** Which time bucket (index into BUCKET_LABELS) a timestamp falls into. */
function bucketOf(ts: number, now: Date): number {
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const startOfYesterday = startOfToday - DAY_MS;
  const mondayOffset = (now.getDay() + 6) % 7; // Monday = 0
  const startOfWeek = startOfToday - mondayOffset * DAY_MS;
  const startOfMonth = new Date(now.getFullYear(), now.getMonth(), 1).getTime();
  const startOfYear = new Date(now.getFullYear(), 0, 1).getTime();

  if (ts >= startOfToday) return 0;
  if (ts >= startOfYesterday) return 1;
  if (ts >= startOfWeek) return 2;
  if (ts >= startOfMonth) return 3;
  if (ts >= startOfYear) return 4;
  return 5;
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
  onDeleteChat,
}: SidebarProps) {
  const [menuFor, setMenuFor] = useState<string | null>(null);
  const [menuUp, setMenuUp] = useState(false);

  // Close an open history menu on any outside click / Escape.
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

  const filtered = search.trim()
    ? history.filter((h) => h.name.toLowerCase().includes(search.trim().toLowerCase()))
    : history;

  // Group by time bucket, preserving the backend's newest-first order.
  const now = new Date();
  const groups: HistoryItem[][] = BUCKET_LABELS.map(() => []);
  for (const h of filtered) groups[bucketOf(h.lastMessageAt ?? 0, now)].push(h);

  const renderItem = (h: HistoryItem) => (
    <div
      key={h.id}
      className={'kc-hist-item' + (h.id === activeId ? ' active' : '')}
      role="button"
      tabIndex={0}
      onClick={() => onSelect(h)}
      onKeyDown={(e) => { if (e.key === 'Enter') onSelect(h); }}
    >
      <Avatar character={{ name: h.name, avatar: h.avatar }} className="kc-hist-avatar" />
      <span className="kc-hist-name">{h.name}</span>
      <button
        className="kc-hist-more"
        aria-label="Chat actions"
        onClick={(e) => {
          e.stopPropagation();
          if (menuFor === h.id) { setMenuFor(null); return; }
          const rect = e.currentTarget.getBoundingClientRect();
          setMenuUp(window.innerHeight - rect.bottom < 70);
          setMenuFor(h.id);
        }}
      >
        ⋯
      </button>
      {menuFor === h.id && (
        <div
          className={'kc-hist-menu' + (menuUp ? ' kc-hist-menu--up' : '')}
          onClick={(e) => e.stopPropagation()}
        >
          <button className="danger" onClick={() => { onDeleteChat(h); setMenuFor(null); }}>
            <TrashIcon /> <span>Delete chat</span>
          </button>
        </div>
      )}
    </div>
  );

  return (
    <aside className="kc-side">
      <button className="kc-brand" onClick={onBrand}>
        <img className="kc-logo" src="/icon.svg" alt="" />
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

      <div className="kc-history">
        {BUCKET_LABELS.map((label, i) =>
          groups[i].length > 0 ? (
            <div key={label} className="kc-hist-group">
              <div className="kc-hist-group-label">{label}</div>
              {groups[i].map(renderItem)}
            </div>
          ) : null,
        )}
      </div>

      <button className="kc-settings-btn" onClick={onSettings}>
        <span style={{ fontSize: 17 }}>⚙︎</span> Settings
      </button>
    </aside>
  );
}
