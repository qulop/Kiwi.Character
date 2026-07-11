import type { Persona } from '../types';
import Avatar from './Avatar';
import { PersonaGlyph, CrossIcon } from './icons';

interface PersonasModalProps {
  personas: Persona[];
  onClose: () => void;
  onCreate: () => void;
  onDelete: (p: Persona) => void;
}

/** Lists the user's personas; this is the root of the personas modal stack,
 * so its header always shows a plain close (×), never a back arrow. */
export default function PersonasModal({ personas, onClose, onCreate, onDelete }: PersonasModalProps) {
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
              <div key={p.id} className="kc-persona-card">
                <Avatar character={p} className="kc-persona-avatar" />
                <span className="kc-persona-name">{p.name}</span>
                <button className="kc-persona-delete" aria-label="Delete persona" onClick={() => onDelete(p)}>
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
