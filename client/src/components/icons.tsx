/* Inline SVG icons — stroke uses currentColor where it matters. */

export const SearchIcon = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" style={{ flex: '0 0 auto' }}>
    <circle cx="6.7" cy="6.7" r="4.6" stroke="#868c91" strokeWidth="1.4" />
    <line x1="10.4" y1="10.4" x2="14" y2="14" stroke="#868c91" strokeWidth="1.4" strokeLinecap="round" />
  </svg>
);

export const SendIcon = () => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
    <line x1="4" y1="12" x2="18" y2="12" stroke="#fff" strokeWidth="1.9" strokeLinecap="round" />
    <polyline points="12,6 18,12 12,18" stroke="#fff" strokeWidth="1.9" fill="none" strokeLinecap="round" strokeLinejoin="round" />
  </svg>
);

export const UserGlyph = () => (
  <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
    <circle cx="12" cy="9" r="3.3" stroke="#aeb3b6" strokeWidth="1.5" />
    <path d="M5.5 19c0-3.2 2.9-5.3 6.5-5.3s6.5 2.1 6.5 5.3" stroke="#aeb3b6" strokeWidth="1.5" strokeLinecap="round" />
  </svg>
);

export const PersonaGlyph = () => (
  <svg width="17" height="17" viewBox="0 0 24 24" fill="none">
    <circle cx="12" cy="9" r="3.4" stroke="#868c91" strokeWidth="1.5" />
    <path d="M5.5 19c0-3.3 2.9-5.5 6.5-5.5s6.5 2.2 6.5 5.5" stroke="#868c91" strokeWidth="1.5" strokeLinecap="round" />
  </svg>
);

export const NewCharIcon = () => (
  <svg width="23" height="23" viewBox="0 0 24 24" fill="none">
    <circle cx="9.5" cy="8" r="3.4" stroke="#e7e9ea" strokeWidth="1.5" />
    <path d="M3.5 19.5c0-3.3 2.7-5.5 6-5.5s6 2.2 6 5.5" stroke="#e7e9ea" strokeWidth="1.5" strokeLinecap="round" />
    <line x1="19" y1="6" x2="19" y2="12" stroke="#b08bff" strokeWidth="1.7" strokeLinecap="round" />
    <line x1="16" y1="9" x2="22" y2="9" stroke="#b08bff" strokeWidth="1.7" strokeLinecap="round" />
  </svg>
);

export const UploadGlyph = () => (
  <svg width="42" height="42" viewBox="0 0 24 24" fill="none">
    <circle cx="12" cy="9.5" r="3.6" stroke="#868c91" strokeWidth="1.5" />
    <path d="M5 20c0-3.6 3.1-6 7-6s7 2.4 7 6" stroke="#868c91" strokeWidth="1.5" strokeLinecap="round" />
  </svg>
);
