import type { Character } from '../types';
import { initialOf } from '../types';

interface AvatarProps {
  /** Only the name (for the monogram fallback) and avatar URL are needed. */
  character: Pick<Character, 'name' | 'avatar'>;
  /** Extra class for sizing/shape (e.g. kc-chat-head-avatar, kc-msg-avatar). */
  className?: string;
}

/**
 * Circular avatar: shows the character's image when present, otherwise the
 * monogram initial. Reuses the existing `.kc-avatar` styling.
 */
export default function Avatar({ character, className }: AvatarProps) {
  return (
    <div className={'kc-avatar' + (className ? ' ' + className : '')}>
      {character.avatar ? (
        <img
          src={character.avatar}
          alt=""
          style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: 'inherit' }}
        />
      ) : (
        initialOf(character.name)
      )}
    </div>
  );
}
