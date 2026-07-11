import type { Persona } from '../types';
import Avatar from './Avatar';
import { PersonaGlyph, CrossIcon, PenIcon } from './icons';

interface PersonasModalProps {
  personas: Persona[];
  /** The persona currently active for this chat, if any — highlighted in the list. */
  activePersonaId?: string | null;
  onClose: () => void;
  onCreate: () => void;
  onEdit: (p: Persona) => void;
  onDelete: (p: Persona) => void;
  /** Clicking a card (not its pen/delete buttons) makes it the active persona. */
  onSelect: (p: Persona) => void;
}

/** Lists the user's personas; this is the root of the personas modal stack,
 * so its header always shows a plain close (×), never a back arrow. */
export default function PersonasModal({
  personas,
  activePersonaId,
  onClose,
  onCreate,
  onEdit,
  onDelete,
  onSelect,
}: PersonasModalProps) {
  return (
    <div className="kc-modal kc-modal--new" onClick={(e) => e.stopPropagation()}>
      <div className="kc-modal-head">
        <PersonaGlyph />
        <span className="kc-modal-title">Personas</span>
        <button className="kc-modal-close" onClick={onClose}>×</button>
      </div>
      <div className="kc-divider" />

      <div className="kc-modal-create-row">
        <button className="kc-primary-btn kc-primary-btn--pill" onClick={onCreate}>Create</button>
      </div>
      <div className="kc-divider" />

      <div className="kc-modal-body">
        {personas.length === 0 ? (
          <div className="kc-personas-empty">You don't have any created personas yet</div>
        ) : (
          <div className="kc-personas-list">
            {personas.map((p) => (
              <div
                key={p.id}
                className={'kc-persona-card' + (p.id === activePersonaId ? ' active' : '')}
                role="button"
                tabIndex={0}
                onClick={() => onSelect(p)}
                onKeyDown={(e) => { if (e.key === 'Enter') onSelect(p); }}
              >
                <Avatar character={p} className="kc-persona-avatar" />
                <span className="kc-persona-name">{p.name}</span>
                <button
                  className="kc-persona-edit"
                  aria-label="Edit persona"
                  onClick={(e) => { e.stopPropagation(); onEdit(p); }}
                >
                  <PenIcon />
                </button>
                <button
                  className="kc-persona-delete"
                  aria-label="Delete persona"
                  onClick={(e) => { e.stopPropagation(); onDelete(p); }}
                >
                  <CrossIcon />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
