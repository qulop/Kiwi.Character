import React, { useRef, useState } from 'react';
import type { Character, NewGroupInput } from '../types';
import { GroupIcon, UploadGlyph } from './icons';
import Avatar from './Avatar';
import AvatarCropModal from './AvatarCropModal';
import AddMembersModal from './AddMembersModal';

interface GroupFormModalProps {
  /** All characters, for the Add Member picker. */
  characters: Character[];
  onClose: () => void;
  onSubmit: (input: NewGroupInput) => void | Promise<void>;
}

interface FormState {
  name: string;
  topic: string;
  background: string;
  avatar: string | null;
}

const EMPTY: FormState = { name: '', topic: '', background: '', avatar: null };

/**
 * "New Group" pop-up. Add Member and the avatar cropper are rendered as
 * internal overlays (not pushed onto App's modal stack) so this form's local
 * state — name/topic/background/members typed so far — survives the round
 * trip instead of being unmounted.
 */
export default function GroupFormModal({ characters, onClose, onSubmit }: GroupFormModalProps) {
  const [form, setForm] = useState<FormState>(EMPTY);
  const [memberIds, setMemberIds] = useState<string[]>([]);
  const [addMembersOpen, setAddMembersOpen] = useState(false);
  const [pendingImage, setPendingImage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const set = <K extends keyof FormState>(k: K, v: FormState[K]) =>
    setForm((p) => ({ ...p, [k]: v }));

  const onPickImage = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setPendingImage(String(reader.result));
    reader.readAsDataURL(file);
    e.target.value = '';
  };

  const selectedMembers = characters.filter((c) => memberIds.includes(c.id));
  const canSubmit = form.name.trim().length > 0 && !busy;

  const submit = async () => {
    if (!form.name.trim()) {
      setError('Group name is required');
      return;
    }
    // Unlike Add Member's disabled button, Create group stays clickable and
    // just warns — matching the spec's "if the user would try to press this
    // button while less than 2 characters selected" wording.
    if (memberIds.length < 2) {
      setError('Select at least two characters to form a group');
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await onSubmit({
        name: form.name.trim(),
        topic: form.topic,
        background: form.background,
        avatar: form.avatar,
        memberIds,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="kc-modal kc-modal--new" onClick={(e) => e.stopPropagation()}>
        <div className="kc-modal-head">
          <GroupIcon />
          <span className="kc-modal-title">New Group</span>
          <button className="kc-modal-close" onClick={onClose} aria-label="Close">×</button>
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
            <span className="kc-field-label">Group Name</span>
            <input className="kc-input" placeholder="Example: B-Day Party"
              value={form.name} onChange={(e) => set('name', e.target.value)} />
          </label>

          <label className="kc-field">
            <span className="kc-field-label">Group Topic</span>
            <textarea className="kc-textarea" rows={2} placeholder="Example: A birthday celebration with friends"
              value={form.topic} onChange={(e) => set('topic', e.target.value)} />
          </label>

          <label className="kc-field">
            <span className="kc-field-label">Background &amp; Relationships</span>
            <textarea className="kc-textarea" rows={4}
              placeholder="Example: Dina has a birthday, so she invited @user and Andrew to celebrate it together"
              value={form.background} onChange={(e) => set('background', e.target.value)} />
          </label>

          <div className="kc-field">
            <span className="kc-field-label">Members</span>
            <button className="kc-add-members-btn" onClick={() => setAddMembersOpen(true)} aria-label="Add members">
              +
            </button>
          </div>

          {selectedMembers.length > 0 && (
            <div className="kc-group-members-list">
              {selectedMembers.map((c) => (
                <div key={c.id} className="kc-group-member-row">
                  <Avatar character={c} className="kc-persona-avatar" />
                  <span className="kc-persona-name">{c.name}</span>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="kc-modal-foot">
          {error && <span className="kc-form-error">{error}</span>}
          <button className="kc-primary-btn kc-primary-btn--pill" disabled={!canSubmit} onClick={submit}>
            {busy ? 'Creating…' : 'Create group'}
          </button>
        </div>
      </div>

      {addMembersOpen && (
        <AddMembersModal
          characters={characters}
          initialSelected={memberIds}
          onClose={onClose}
          onBack={() => setAddMembersOpen(false)}
          onConfirm={(ids) => { setMemberIds(ids); setAddMembersOpen(false); }}
        />
      )}

      {pendingImage && (
        <AvatarCropModal
          src={pendingImage}
          onCancel={() => setPendingImage(null)}
          onApply={(cropped) => { set('avatar', cropped); setPendingImage(null); }}
        />
      )}
    </>
  );
}
