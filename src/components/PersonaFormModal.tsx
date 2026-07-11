import React, { useRef, useState } from 'react';
import type { NewPersonaInput } from '../types';
import { PersonaGlyph, UploadGlyph, BackIcon } from './icons';

interface PersonaFormModalProps {
  title: string;
  submitLabel: string;
  /** Empty values for create, or the persona's current data for edit. */
  initial: NewPersonaInput;
  /** Always opened as an overlap of the Personas list: × closes everything,
   * the back arrow returns to just that list. Both are shown, × on the left. */
  onClose: () => void;
  onBack: () => void;
  onSubmit: (input: NewPersonaInput) => void | Promise<void>;
}

/** Create/edit persona form — same structure/style as CharacterFormModal, but
 * with only the two fields a persona needs. */
export default function PersonaFormModal({
  title,
  submitLabel,
  initial,
  onClose,
  onBack,
  onSubmit,
}: PersonaFormModalProps) {
  const [form, setForm] = useState<NewPersonaInput>(initial);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const set = <K extends keyof NewPersonaInput>(k: K, v: NewPersonaInput[K]) =>
    setForm((p) => ({ ...p, [k]: v }));

  const onPickImage = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => set('avatar', String(reader.result));
    reader.readAsDataURL(file);
  };

  const canSubmit = form.name.trim().length > 0 && !busy;

  const submit = async () => {
    const trimmed = { ...form, name: form.name.trim() };
    if (!trimmed.name) {
      setError('Persona name is required');
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await onSubmit(trimmed);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="kc-modal kc-modal--new" onClick={(e) => e.stopPropagation()}>
      <div className="kc-modal-head">
        <PersonaGlyph />
        <span className="kc-modal-title">{title}</span>
        <div className="kc-modal-head-actions">
          <button className="kc-modal-close" onClick={onClose} aria-label="Close">×</button>
          <button className="kc-modal-close" onClick={onBack} aria-label="Back">
            <BackIcon />
          </button>
        </div>
      </div>
      <div className="kc-divider" />

      <div className="kc-modal-body kc-modal-body--new">
        <div className="kc-avatar-upload-wrap">
          <button className="kc-avatar-upload" onClick={() => fileRef.current?.click()}>
            {form.avatar ? <img src={form.avatar} alt="" /> : <UploadGlyph />}
          </button>
          <input ref={fileRef} type="file" accept="image/*" hidden onChange={onPickImage} />
          <span className="kc-upload-hint">Click to upload an image</span>
        </div>

        <label className="kc-field">
          <span className="kc-field-label">Persona Name</span>
          <input className="kc-input" placeholder="Example: Cool Guy"
            value={form.name} onChange={(e) => set('name', e.target.value)} />
        </label>

        <label className="kc-field">
          <span className="kc-field-label">Description</span>
          <textarea className="kc-textarea" rows={3} placeholder="Example: Just cool"
            value={form.description} onChange={(e) => set('description', e.target.value)} />
        </label>
      </div>

      <div className="kc-modal-foot">
        {error && <span className="kc-form-error">{error}</span>}
        <button className="kc-primary-btn kc-primary-btn--pill" disabled={!canSubmit} onClick={submit}>
          {busy ? 'Saving…' : submitLabel}
        </button>
      </div>
    </div>
  );
}
