import React, { useRef, useState } from 'react';
import type { NewCharacterInput } from '../types';
import * as api from '../api';
import { NewCharIcon, UploadGlyph } from './icons';

interface NewCharacterModalProps {
  onClose: () => void;
  onCreate: (input: NewCharacterInput) => void | Promise<void>;
}

/** Create-character form. Submits a NewCharacterInput to the caller. */
export default function NewCharacterModal({ onClose, onCreate }: NewCharacterModalProps) {
  const [form, setForm] = useState<NewCharacterInput>({
    name: '',
    info: '',
    appearance: '',
    description: '',
    initialMessage: '',
    avatar: null,
  });
  const fileRef = useRef<HTMLInputElement>(null);

  const [error, setError] = useState<string | null>(null);
  const [nameTaken, setNameTaken] = useState(false);
  const [busy, setBusy] = useState(false);

  const set = <K extends keyof NewCharacterInput>(k: K, v: NewCharacterInput[K]) =>
    setForm((p) => ({ ...p, [k]: v }));

  const onNameChange = (v: string) => {
    set('name', v);
    setNameTaken(false);
    setError(null);
  };

  // Early, non-blocking availability hint when the user leaves the name field.
  const checkName = async () => {
    const n = form.name.trim();
    if (!n) return;
    try {
      setNameTaken(!(await api.characterNameAvailable(n)));
    } catch {
      // Ignore — the create-time check still guards against duplicates.
    }
  };

  const onPickImage = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => set('avatar', String(reader.result));
    reader.readAsDataURL(file);
  };

  const canCreate = form.name.trim().length > 0 && !nameTaken && !busy;

  const submit = async () => {
    const trimmed = { ...form, name: form.name.trim() };
    if (!trimmed.name) {
      setError('Character name is required');
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await onCreate(trimmed);
    } catch (e) {
      // e.g. "A character named 'John' already exists" — keep the form open.
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="kc-modal kc-modal--new" onClick={(e) => e.stopPropagation()}>
      <div className="kc-modal-head">
        <NewCharIcon />
        <span className="kc-modal-title">New Character</span>
        <button className="kc-modal-close" onClick={onClose}>×</button>
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
          <span className="kc-field-label">Character Name</span>
          <input className={'kc-input' + (nameTaken ? ' kc-input--error' : '')} placeholder="Example: John Doe"
            value={form.name} onChange={(e) => onNameChange(e.target.value)} onBlur={checkName} />
          {nameTaken && <span className="kc-field-error">That name is already taken</span>}
        </label>

        <label className="kc-field">
          <span className="kc-field-label">Short info <span className="kc-field-note">· shown under the name on cards</span></span>
          <input className="kc-input" placeholder="Example: Mysterious man with kind heart"
            value={form.info} onChange={(e) => set('info', e.target.value)} />
        </label>

        <label className="kc-field">
          <span className="kc-field-label">Character Appearance Description</span>
          <textarea className="kc-textarea" rows={3} placeholder="Example: A tall man, usually wearing a cloak with fedora hat"
            value={form.appearance} onChange={(e) => set('appearance', e.target.value)} />
        </label>

        <label className="kc-field">
          <span className="kc-field-label">Overall Description</span>
          <textarea className="kc-textarea" rows={3} placeholder="Example: A man with kind soul. Likes chocolate and hats"
            value={form.description} onChange={(e) => set('description', e.target.value)} />
        </label>

        <label className="kc-field">
          <span className="kc-field-label">Initial Chat Message <span className="kc-field-note">· starts the conversation</span></span>
          <textarea className="kc-textarea" rows={3} placeholder="Example: Hello! My name is John! Who are you, stranger?"
            value={form.initialMessage} onChange={(e) => set('initialMessage', e.target.value)} />
        </label>
      </div>

      <div className="kc-modal-foot">
        {error && <span className="kc-form-error">{error}</span>}
        <button className="kc-primary-btn kc-primary-btn--pill" disabled={!canCreate}
          onClick={submit}>
          {busy ? 'Creating…' : 'Create Character'}
        </button>
      </div>
    </div>
  );
}
