import { useState } from 'react';
import type { ModelSettings, EndpointTestResult } from '../types';
import { DEFAULT_SETTINGS } from '../types';

interface SettingsModalProps {
  initial?: ModelSettings;
  /** Whether the app's background ping currently sees the endpoint as online. */
  initialVerified?: boolean;
  /** Models from the app's last successful ping. */
  initialModels?: string[];
  /** Models currently loaded on the server, from the app's last ping. */
  initialLoaded?: string[];
  onClose: () => void;
  /** Ping the endpoint; resolve with the available model list. */
  onTest: (endpoint: string) => Promise<EndpointTestResult>;
  /** Persist + load the chosen model on the server. Rejects on failure. */
  onLoad: (settings: ModelSettings) => Promise<void>;
  /** Fetch the models currently loaded on the given endpoint. */
  onRefreshLoaded: (endpoint: string) => Promise<string[]>;
  /** Unload a model on the server. */
  onUnload: (model: string) => Promise<void>;
  /** Persist settings without loading (e.g. when picking an already-loaded model). */
  onSave: (settings: ModelSettings) => Promise<void>;
}

/**
 * Model / endpoint settings. Model controls stay locked (dimmed,
 * non-interactive) until the endpoint has been verified with "Test".
 */
export default function SettingsModal({
  initial = DEFAULT_SETTINGS,
  initialVerified = false,
  initialModels = [],
  initialLoaded = [],
  onClose,
  onTest,
  onLoad,
  onRefreshLoaded,
  onUnload,
  onSave,
}: SettingsModalProps) {
  const [s, setS] = useState<ModelSettings>(initial);
  // Start unlocked if the app's background ping already sees the endpoint online.
  const [verified, setVerified] = useState(initialVerified);
  const [models, setModels] = useState<string[]>(
    initialModels.length
      ? initialModels
      : ['llama-3.1-8b-instruct', 'mistral-7b-instruct-v0.3', 'qwen2.5-14b-instruct', 'phi-3-mini-4k'],
  );
  const [advanced, setAdvanced] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testError, setTestError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadResult, setLoadResult] = useState<{ ok: boolean; text: string } | null>(null);
  const [loaded, setLoaded] = useState<string[]>(initialLoaded);
  const [unloading, setUnloading] = useState(false);

  const set = <K extends keyof ModelSettings>(k: K, v: ModelSettings[K]) =>
    setS((prev) => ({ ...prev, [k]: v }));

  const runTest = async () => {
    setTesting(true);
    setTestError(null);
    try {
      const res = await onTest(s.endpoint);
      setVerified(res.ok);
      if (res.ok) {
        if (res.models?.length) {
          setModels(res.models);
          if (!res.models.includes(s.model)) set('model', res.models[0]);
        }
        void refreshLoaded(s.endpoint);
      } else {
        setTestError(res.error || 'Could not connect to the endpoint.');
      }
    } catch (e) {
      setVerified(false);
      setTestError(String(e));
    } finally {
      setTesting(false);
    }
  };

  const refreshLoaded = async (endpoint: string) => {
    try {
      setLoaded(await onRefreshLoaded(endpoint));
    } catch {
      setLoaded([]);
    }
  };

  const load = async () => {
    setLoading(true);
    setLoadResult(null);
    try {
      await onLoad(s);
      setLoadResult({ ok: true, text: `Loaded “${s.model}”.` });
      await refreshLoaded(s.endpoint);
    } catch (e) {
      setLoadResult({ ok: false, text: String(e) });
    } finally {
      setLoading(false);
    }
  };

  // Which loaded model is used with characters (falls back to the first).
  const loadedPick = loaded.includes(s.model) ? s.model : loaded[0] ?? '';

  const unloadSelected = async () => {
    if (!loadedPick) return;
    setUnloading(true);
    setLoadResult(null);
    try {
      await onUnload(loadedPick);
      await refreshLoaded(s.endpoint);
      setLoadResult({ ok: true, text: `Unloaded “${loadedPick}”.` });
    } catch (e) {
      setLoadResult({ ok: false, text: String(e) });
    } finally {
      setUnloading(false);
    }
  };

  const lock = verified ? '' : ' kc-locked';

  return (
    <div className="kc-modal kc-modal--settings" onClick={(e) => e.stopPropagation()}>
      <div className="kc-modal-head">
        <span style={{ fontSize: 21 }}>⚙︎</span>
        <span className="kc-modal-title">Settings</span>
        <button className="kc-modal-close" onClick={onClose}>×</button>
      </div>
      <div className="kc-divider" />

      <div className="kc-modal-body">
        <div>
          <div className="kc-endpoint-row">
            <span className="label">API Endpoint:</span>
            <input
              className="kc-input kc-endpoint-input"
              value={s.endpoint}
              onChange={(e) => { set('endpoint', e.target.value); setVerified(false); setTestError(null); }}
            />
            <button className="kc-test-btn" onClick={runTest} disabled={testing}>
              {testing ? 'Testing…' : 'Test'}
            </button>
          </div>
          {verified ? (
            <div className="kc-status-ok">✓ Endpoint verified — model settings unlocked.</div>
          ) : testError ? (
            <div className="kc-form-error">⚠️ {testError}</div>
          ) : (
            <div className="kc-status-wait">Press “Test” to ping the endpoint and unlock model settings.</div>
          )}
        </div>

        <div className="kc-divider" />

        <div className="kc-loaded-section">
          <div className="kc-section-label" style={{ padding: 0 }}>Loaded model</div>
          {loaded.length === 0 ? (
            <div className="kc-status-wait">No model is loaded on the server.</div>
          ) : (
            <div className="kc-loaded-row">
              {loaded.length > 1 ? (
                <select
                  className="kc-select"
                  value={loadedPick}
                  onChange={(e) => {
                    // Picking an already-loaded model takes effect right away.
                    const ns = { ...s, model: e.target.value };
                    setS(ns);
                    onSave(ns).catch(console.error);
                  }}
                >
                  {loaded.map((m) => <option key={m} value={m}>{m}</option>)}
                </select>
              ) : (
                <span className="kc-loaded-name">{loaded[0]}</span>
              )}
              <button
                className="kc-unload-btn"
                onClick={unloadSelected}
                disabled={unloading || !loadedPick}
              >
                {unloading ? 'Unloading…' : 'Unload'}
              </button>
            </div>
          )}
        </div>

        <div className="kc-divider" />

        <div className={'kc-model-section' + lock}>
          <div className="kc-section-label" style={{ padding: 0 }}>Model configuration</div>

          <label className="kc-field">
            <span className="kc-field-label">Model</span>
            <select className="kc-select" value={s.model} onChange={(e) => set('model', e.target.value)}>
              {models.map((m) => <option key={m} value={m}>{m}</option>)}
            </select>
          </label>

          <div className="kc-field">
            <div className="kc-rangerow"><span>Context length</span><span className="val">{s.contextLength}k</span></div>
            <input className="kc-range" type="range" min={0} max={100}
              value={s.contextLength} onChange={(e) => set('contextLength', +e.target.value)} />
          </div>

          <div className="kc-field">
            <div className="kc-rangerow"><span>GPU offload</span><span className="val">{s.gpuOffload} / Max</span></div>
            <input className="kc-range" type="range" min={0} max={100}
              value={s.gpuOffload} onChange={(e) => set('gpuOffload', +e.target.value)} />
          </div>

          <div className="kc-advanced">
            <button className="kc-advanced-toggle" onClick={() => setAdvanced((a) => !a)}>
              <span className="chev">{advanced ? '▾' : '▸'}</span> Advanced settings
            </button>
            {advanced && (
              <div className="kc-advanced-body">
                <div className="kc-field">
                  <div className="kc-rangerow"><span>Temperature</span><span className="val">{s.temperature.toFixed(1)}</span></div>
                  <input className="kc-range" type="range" min={0} max={2} step={0.1}
                    value={s.temperature} onChange={(e) => set('temperature', +e.target.value)} />
                </div>
                <label className="kc-field" style={{ flexDirection: 'row', alignItems: 'center', gap: 12, fontSize: 16 }}>
                  <span style={{ flex: '0 0 auto' }}>Max tokens</span>
                  <input className="kc-input" style={{ flex: '0 0 120px' }} type="number"
                    value={s.maxTokens} onChange={(e) => set('maxTokens', +e.target.value)} />
                </label>
                <label className="kc-field">
                  <span className="kc-field-label">System prompt</span>
                  <textarea className="kc-textarea" rows={3} placeholder="You are a helpful assistant…"
                    value={s.systemPrompt} onChange={(e) => set('systemPrompt', e.target.value)} />
                </label>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="kc-modal-foot">
        {loadResult && (
          <span
            className={loadResult.ok ? 'kc-status-ok' : 'kc-form-error'}
            style={{ marginRight: 'auto', alignSelf: 'center' }}
          >
            {loadResult.ok ? '✓ ' : '⚠️ '}{loadResult.text}
          </span>
        )}
        <button className={'kc-primary-btn' + lock} onClick={load} disabled={!verified || loading}>
          {loading ? 'Loading…' : 'Load model'}
        </button>
      </div>
    </div>
  );
}
