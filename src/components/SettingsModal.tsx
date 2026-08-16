import { useEffect, useState } from 'react';
import type { EndpointTestResult, LoadedModel, MemorySettings, ModelLoadResult, ModelSettings } from '../types';
import { DEFAULT_MEMORY_SETTINGS, DEFAULT_SETTINGS } from '../types';

type Section = 'models' | 'memory';
type Notice = { ok: boolean; text: string } | null;

interface Props {
  initial?: ModelSettings; memoryInitial?: MemorySettings; initialVerified?: boolean;
  initialModels?: string[]; initialLoaded?: LoadedModel[]; onClose: () => void;
  onTest: (endpoint: string) => Promise<EndpointTestResult>;
  onLoad: (settings: ModelSettings) => Promise<ModelLoadResult>;
  onLoadEmbedding: (endpoint: string, model: string) => Promise<ModelLoadResult>;
  onRefreshLoaded: (endpoint: string) => Promise<LoadedModel[]>;
  onUnload: (model: string) => Promise<void>;
  onSaveMemory: (settings: MemorySettings) => Promise<void>;
}

const typeLabel = (kind: string) => {
  const normalized = kind.toLowerCase();
  if (normalized.includes('embed')) return 'embedding';
  if (normalized.includes('rerank')) return 'reranker';
  return 'character';
};

export default function SettingsModal({
  initial = DEFAULT_SETTINGS, memoryInitial = DEFAULT_MEMORY_SETTINGS, initialVerified = false,
  initialModels = [], initialLoaded = [], onClose, onTest, onLoad, onLoadEmbedding,
  onRefreshLoaded, onUnload, onSaveMemory,
}: Props) {
  const [section, setSection] = useState<Section>('models');
  const [s, setS] = useState(initial);
  const [memory, setMemory] = useState(memoryInitial);
  const [models, setModels] = useState(initialModels);
  const [embeddingModels, setEmbeddingModels] = useState<string[]>([]);
  const [loaded, setLoaded] = useState(initialLoaded);
  const [verified, setVerified] = useState(initialVerified);
  const [embeddingVerified, setEmbeddingVerified] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [busy, setBusy] = useState<'character' | 'embedding' | 'test' | 'embedding-test' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice>(null);
  const [characterTestStatus, setCharacterTestStatus] = useState<Notice>(null);
  const [embeddingTestStatus, setEmbeddingTestStatus] = useState<Notice>(null);
  const set = <K extends keyof ModelSettings>(key: K, value: ModelSettings[K]) => setS((old) => ({ ...old, [key]: value }));
  const setMemoryField = <K extends keyof MemorySettings>(key: K, value: MemorySettings[K]) => setMemory((old) => ({ ...old, [key]: value }));

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(null), 5000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const refreshLoaded = async () => { try { setLoaded(await onRefreshLoaded(s.endpoint)); } catch { setLoaded([]); } };
  const testCharacter = async () => {
    setBusy('test'); setCharacterTestStatus(null);
    try { const result = await onTest(s.endpoint); setVerified(result.ok); if (!result.ok) throw new Error(result.error); setModels(result.models); if (result.models[0] && !result.models.includes(s.model)) set('model', result.models[0]); await refreshLoaded(); setCharacterTestStatus({ ok: true, text: 'Endpoint verified — model settings unlocked.' }); }
    catch (e) { setVerified(false); setCharacterTestStatus({ ok: false, text: String(e) }); } finally { setBusy(null); }
  };
  const testEmbedding = async () => {
    setBusy('embedding-test'); setEmbeddingTestStatus(null);
    try { const result = await onTest(memory.embeddingEndpoint); setEmbeddingVerified(result.ok); if (!result.ok) throw new Error(result.error); setEmbeddingModels(result.models); if (result.models[0] && !result.models.includes(memory.embeddingModel)) setMemoryField('embeddingModel', result.models[0]); setEmbeddingTestStatus({ ok: true, text: 'Embedding endpoint verified — embedding settings unlocked.' }); }
    catch (e) { setEmbeddingVerified(false); setEmbeddingTestStatus({ ok: false, text: String(e) }); } finally { setBusy(null); }
  };
  const saveMemory = async (next = memory) => { await onSaveMemory(next); setMemory(next); };
  const loadCharacter = async () => { setBusy('character'); setError(null); try { const result = await onLoad(s); setNotice({ ok: true, text: `Character model “${s.model}” loaded${result.contextLength ? ` with ${result.contextLength.toLocaleString()} tokens` : ''}.` }); await refreshLoaded(); } catch (e) { setError(String(e)); } finally { setBusy(null); } };
  const loadEmbedding = async () => { setBusy('embedding'); setError(null); try { await saveMemory(); await onLoadEmbedding(memory.embeddingEndpoint, memory.embeddingModel); setNotice({ ok: true, text: `Embedding model “${memory.embeddingModel}” loaded.` }); } catch (e) { setError(String(e)); } finally { setBusy(null); } };
  const unloadModel = async (model: string) => {
    setBusy('character'); setError(null);
    try { await onUnload(model); await refreshLoaded(); setNotice({ ok: true, text: `Model “${model}” unloaded.` }); }
    catch (e) { setError(String(e)); } finally { setBusy(null); }
  };

  return <div className="kc-modal kc-modal--settings" onClick={(e) => e.stopPropagation()}>
    <div className="kc-modal-head"><span style={{ fontSize: 21 }}>⚙︎</span><span className="kc-modal-title">Settings</span><button className="kc-modal-close" onClick={onClose}>×</button></div><div className="kc-divider" />
    <div className="kc-modal-body kc-modal-body--settings">
      <nav className="kc-settings-nav"><button className={'kc-settings-nav-item' + (section === 'models' ? ' active' : '')} onClick={() => setSection('models')}>Models</button><button className={'kc-settings-nav-item' + (section === 'memory' ? ' active' : '')} onClick={() => setSection('memory')}>Memory</button></nav>
      <div className="kc-settings-content">
        {error && <div className="kc-form-error">⚠ {error}</div>}
        {section === 'models' && <>
          <div className="kc-field"><span className="kc-field-label">API Endpoint</span><div className="kc-endpoint-row"><input className="kc-input kc-endpoint-input" value={s.endpoint} onChange={(e) => { set('endpoint', e.target.value); setVerified(false); setCharacterTestStatus(null); }} /><button className="kc-test-btn" onClick={testCharacter} disabled={busy !== null}>{busy === 'test' ? 'Testing…' : 'Test'}</button></div>{characterTestStatus && <div className={characterTestStatus.ok ? 'kc-status-ok' : 'kc-form-error'}>{characterTestStatus.ok ? '✓ ' : '⚠ '}{characterTestStatus.text}</div>}</div>
          <div className="kc-divider" /><section className="kc-loaded-section"><div className="kc-section-label" style={{ padding: 0 }}>Loaded models</div>{loaded.length ? loaded.map((model) => <div className="kc-loaded-model" key={model.id}><span className={`kc-model-badge kc-model-badge--${typeLabel(model.kind)}`}>[{typeLabel(model.kind)}]</span><span>{model.id}</span><button className="kc-unload-btn" onClick={() => unloadModel(model.id)} disabled={busy !== null}>Unload</button></div>) : <div className="kc-status-wait">No model is loaded on the server.</div>}</section>
          <div className="kc-divider" /><section className={'kc-model-section' + (verified ? '' : ' kc-locked')}><div className="kc-section-label" style={{ padding: 0 }}>Character model configuration</div><label className="kc-field"><span className="kc-field-label">Model</span><select className="kc-select" value={s.model} onChange={(e) => set('model', e.target.value)}>{models.map((model) => <option key={model}>{model}</option>)}</select></label><div className="kc-field"><div className="kc-rangerow"><span>Context length</span><span className="val">{s.contextLength}k</span></div><input className="kc-range" type="range" min={0} max={100} value={s.contextLength} onChange={(e) => set('contextLength', +e.target.value)} /></div><div className="kc-field"><div className="kc-rangerow"><span>GPU offload</span><span className="val">{s.gpuOffload} / Max</span></div><input className="kc-range" type="range" min={0} max={100} value={s.gpuOffload} onChange={(e) => set('gpuOffload', +e.target.value)} /></div><button className="kc-advanced-toggle" onClick={() => setAdvanced(!advanced)}><span className="chev">{advanced ? '▾' : '▸'}</span> Advanced settings</button>{advanced && <label className="kc-field"><span className="kc-field-label">Temperature: {s.temperature.toFixed(1)}</span><input className="kc-range" type="range" min={0} max={2} step={0.1} value={s.temperature} onChange={(e) => set('temperature', +e.target.value)} /></label>}<button className="kc-primary-btn" onClick={loadCharacter} disabled={busy !== null}>{busy === 'character' ? 'Loading…' : 'Load character model'}</button></section>
          <div className="kc-divider" /><section className={'kc-model-section' + (embeddingVerified ? '' : ' kc-locked')}><div className="kc-section-label" style={{ padding: 0 }}>Embedding model configuration</div><label className="kc-field"><span className="kc-field-label">Model</span><select className="kc-select" value={memory.embeddingModel} onChange={(e) => setMemoryField('embeddingModel', e.target.value)}>{embeddingModels.map((model) => <option key={model}>{model}</option>)}</select></label><button className="kc-primary-btn" onClick={loadEmbedding} disabled={busy !== null}>{busy === 'embedding' ? 'Loading…' : 'Load embedding model'}</button></section>
        </>}
        {section === 'memory' && <section className="kc-memory-section"><div className="kc-section-label" style={{ padding: 0 }}>Long-term memory</div><label className="kc-memory-toggle"><input type="checkbox" checked={memory.enabled} onChange={(e) => { const next = { ...memory, enabled: e.target.checked }; void saveMemory(next).catch((err) => setError(String(err))); }} /><span>Enable long-term memory</span></label><div className={'kc-memory-controls' + (memory.enabled ? '' : ' kc-locked')}><div className="kc-field"><span className="kc-field-label">Embedding API endpoint</span><div className="kc-endpoint-row"><input className="kc-input kc-endpoint-input" value={memory.embeddingEndpoint} onChange={(e) => { setMemoryField('embeddingEndpoint', e.target.value); setEmbeddingVerified(false); setEmbeddingTestStatus(null); }} /><button className="kc-test-btn" onClick={testEmbedding} disabled={busy !== null}>{busy === 'embedding-test' ? 'Testing…' : 'Test'}</button></div>{embeddingTestStatus && <div className={embeddingTestStatus.ok ? 'kc-status-ok' : 'kc-form-error'}>{embeddingTestStatus.ok ? '✓ ' : '⚠ '}{embeddingTestStatus.text}</div>}</div><div className="kc-field"><div className="kc-rangerow"><span>Short-term history length</span><span className="val">{memory.recentMessageLimit} messages ({memory.recentMessageLimit / 2} turns)</span></div><input className="kc-range" type="range" min={4} max={40} step={2} value={memory.recentMessageLimit} onChange={(e) => setMemoryField('recentMessageLimit', +e.target.value)} /></div><div className="kc-field"><div className="kc-rangerow"><span>Recall depth <button type="button" className="kc-help-tip" data-tooltip="How many relevant memories are added to each reply.">?</button></span><span className="val">{memory.recallDepth} memories</span></div><input className="kc-range" type="range" min={1} max={12} value={memory.recallDepth} onChange={(e) => setMemoryField('recallDepth', +e.target.value)} /></div><button className="kc-primary-btn" onClick={() => saveMemory().then(() => setNotice({ ok: true, text: 'Memory settings saved.' })).catch((e) => setError(String(e)))}>Save memory settings</button></div></section>}
      </div>
    </div>
    {notice && <div className={'kc-settings-toast ' + (notice.ok ? 'kc-settings-toast--ok' : '')}>{notice.ok ? '✓ ' : '⚠ '}{notice.text}</div>}
  </div>;
}
