interface GroupAvatarProps {
  /** A custom group avatar, if the user picked one. */
  avatar?: string | null;
  /** Up to 4 member avatars (in add order), used when `avatar` is unset. */
  memberAvatars?: (string | null | undefined)[];
  /** Extra class for sizing (e.g. kc-hist-avatar). */
  className?: string;
}

const cell = (src: string | null | undefined, key: number) => (
  <div key={key} className="kc-group-avatar-cell">
    {src ? <img src={src} alt="" /> : <div className="kc-group-avatar-empty" />}
  </div>
);

/**
 * A group's avatar: either a custom image, or — when none was picked — a
 * collage of the first members' avatars (row of 2, "2 over 1" for 3, or a
 * 2x2 grid for 4+).
 */
export default function GroupAvatar({ avatar, memberAvatars, className }: GroupAvatarProps) {
  const cls = 'kc-avatar' + (className ? ' ' + className : '');

  if (avatar) {
    return (
      <div className={cls}>
        <img src={avatar} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: 'inherit' }} />
      </div>
    );
  }

  const avatars = (memberAvatars ?? []).slice(0, 4);

  if (avatars.length <= 2) {
    // A single row so two avatars sit side by side (the container itself is a
    // column flex — one row child would otherwise stack cells vertically).
    return (
      <div className={cls + ' kc-group-avatar'}>
        <div className="kc-group-avatar-row">{avatars.map((a, i) => cell(a, i))}</div>
      </div>
    );
  }
  if (avatars.length === 3) {
    return (
      <div className={cls + ' kc-group-avatar kc-group-avatar--3'}>
        <div className="kc-group-avatar-row">{[avatars[0], avatars[1]].map((a, i) => cell(a, i))}</div>
        <div className="kc-group-avatar-row kc-group-avatar-row--single">{cell(avatars[2], 2)}</div>
      </div>
    );
  }
  return (
    <div className={cls + ' kc-group-avatar kc-group-avatar--4'}>
      <div className="kc-group-avatar-row">{[avatars[0], avatars[1]].map((a, i) => cell(a, i))}</div>
      <div className="kc-group-avatar-row">{[avatars[2], avatars[3]].map((a, i) => cell(a, i + 2))}</div>
    </div>
  );
}
