import { useState } from 'react';
import type { Character } from '../types';
import Avatar from './Avatar';
import { GroupIcon, BackIcon } from './icons';

interface AddMembersModalProps {
  characters: Character[];
  /** Pre-check whoever's already selected in the New Group form. */
  initialSelected: string[];
  /** × always closes the whole group-creation flow. */
  onClose: () => void;
  /** < returns to New Group, discarding any changes made here. */
  onBack: () => void;
  onConfirm: (ids: string[]) => void;
}

/** Character picker stacked over "New Group" — always shows both × and <
 * since it's never opened standalone. */
export default function AddMembersModal({
  characters,
  initialSelected,
  onClose,
  onBack,
  onConfirm,
}: AddMembersModalProps) {
  const [selected, setSelected] = useState<string[]>(initialSelected);

  const toggle = (id: string) =>
    setSelected((s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]));

  const canAdd = selected.length >= 2;

  return (
    <div className="kc-nested-overlay" onClick={onBack}>
      <div className="kc-modal kc-modal--new" onClick={(e) => e.stopPropagation()}>
        <div className="kc-modal-head">
          <GroupIcon />
          <span className="kc-modal-title">Add member</span>
          <div className="kc-modal-head-actions">
            <button className="kc-modal-close" onClick={onClose} aria-label="Close">×</button>
            <button className="kc-modal-close" onClick={onBack} aria-label="Back">
              <BackIcon />
            </button>
          </div>
        </div>
        <div className="kc-divider" />

        <div className="kc-modal-body">
          {characters.length === 0 ? (
            <div className="kc-personas-empty">You don't have any characters yet</div>
          ) : (
            <div className="kc-member-list">
              {characters.map((c) => (
                <label key={c.id} className="kc-member-row">
                  <input
                    type="checkbox"
                    checked={selected.includes(c.id)}
                    onChange={() => toggle(c.id)}
                  />
                  <Avatar character={c} className="kc-persona-avatar" />
                  <span className="kc-persona-name">{c.name}</span>
                </label>
              ))}
            </div>
          )}
        </div>

        <div className="kc-modal-foot">
          {!canAdd && <span className="kc-form-error">Select at least two characters</span>}
          <button
            className="kc-primary-btn kc-primary-btn--pill"
            disabled={!canAdd}
            onClick={() => onConfirm(selected)}
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}
