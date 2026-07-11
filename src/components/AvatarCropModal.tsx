import React, { useRef, useState } from 'react';

interface AvatarCropModalProps {
  /** The freshly picked image (data URL) to crop. */
  src: string;
  onCancel: () => void;
  /** Called with the cropped square image (data URL, PNG). */
  onApply: (dataUrl: string) => void;
}

/** Square viewport size, in CSS px. */
const V = 280;
/** Output resolution of the exported square avatar. */
const OUT = 512;
const MAX_ZOOM = 3;

/**
 * Discord-style avatar cropper: drag to reposition, slider to zoom, within a
 * square viewport with a circular guide ring. Exports a plain square image
 * (not a circular-alpha PNG) — the app already renders every avatar inside a
 * `border-radius: 50%` container, so a square crop is all that's needed.
 */
export default function AvatarCropModal({ src, onCancel, onApply }: AvatarCropModalProps) {
  const imgRef = useRef<HTMLImageElement>(null);
  const [natural, setNatural] = useState({ w: 0, h: 0 });
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const dragRef = useRef<{ startX: number; startY: number; offX: number; offY: number } | null>(null);

  const baseScale = natural.w && natural.h ? V / Math.min(natural.w, natural.h) : 1;
  const scale = baseScale * zoom;
  const dispW = natural.w * scale;
  const dispH = natural.h * scale;

  const clamp = (x: number, y: number) => ({
    x: Math.min(0, Math.max(V - dispW, x)),
    y: Math.min(0, Math.max(V - dispH, y)),
  });

  // Zooming can shrink the allowed offset range — reclamp against the NEW
  // zoom's bounds so zooming out never leaves a gap at the edges. Computed
  // inline (not in a useEffect keyed on natural.w/h) so it can't race with
  // onImgLoad's initial centering.
  const onZoomChange = (z: number) => {
    setZoom(z);
    const s = baseScale * z;
    const dW = natural.w * s;
    const dH = natural.h * s;
    setOffset((o) => ({
      x: Math.min(0, Math.max(V - dW, o.x)),
      y: Math.min(0, Math.max(V - dH, o.y)),
    }));
  };

  const onImgLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const el = e.currentTarget;
    const w = el.naturalWidth;
    const h = el.naturalHeight;
    setNatural({ w, h });
    const bs = V / Math.min(w, h);
    // Center the image in the viewport on first load.
    setOffset({ x: (V - w * bs) / 2, y: (V - h * bs) / 2 });
  };

  const onPointerDown = (e: React.PointerEvent) => {
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Some environments report no "active" pointer for the given id —
      // dragging still works via the window-level pointermove below.
    }
    dragRef.current = { startX: e.clientX, startY: e.clientY, offX: offset.x, offY: offset.y };
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!dragRef.current) return;
    const dx = e.clientX - dragRef.current.startX;
    const dy = e.clientY - dragRef.current.startY;
    setOffset(clamp(dragRef.current.offX + dx, dragRef.current.offY + dy));
  };
  const onPointerUp = () => { dragRef.current = null; };

  const apply = () => {
    const img = imgRef.current;
    if (!img || !natural.w) return;
    const k = OUT / V;
    const canvas = document.createElement('canvas');
    canvas.width = OUT;
    canvas.height = OUT;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.drawImage(img, offset.x * k, offset.y * k, dispW * k, dispH * k);
    onApply(canvas.toDataURL('image/png'));
  };

  return (
    <div className="kc-crop-overlay" onClick={onCancel}>
      <div className="kc-crop-modal" onClick={(e) => e.stopPropagation()}>
        <div className="kc-crop-head">
          <span className="kc-crop-title">Edit Image</span>
          <button className="kc-crop-close" onClick={onCancel} aria-label="Close">×</button>
        </div>

        <div
          className="kc-crop-viewport"
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerLeave={onPointerUp}
        >
          <img
            ref={imgRef}
            src={src}
            alt=""
            crossOrigin="anonymous"
            onLoad={onImgLoad}
            draggable={false}
            className="kc-crop-img"
            style={{ width: dispW, height: dispH, left: offset.x, top: offset.y }}
          />
          <div className="kc-crop-ring" />
        </div>

        <div className="kc-crop-zoom-row">
          <span className="kc-crop-zoom-icon kc-crop-zoom-icon--sm">◎</span>
          <input
            className="kc-range"
            type="range"
            min={1}
            max={MAX_ZOOM}
            step={0.01}
            value={zoom}
            onChange={(e) => onZoomChange(+e.target.value)}
          />
          <span className="kc-crop-zoom-icon kc-crop-zoom-icon--lg">◎</span>
        </div>

        <div className="kc-crop-foot">
          <button className="kc-crop-cancel" onClick={onCancel}>Cancel</button>
          <button className="kc-primary-btn kc-primary-btn--pill" onClick={apply}>Apply</button>
        </div>
      </div>
    </div>
  );
}
