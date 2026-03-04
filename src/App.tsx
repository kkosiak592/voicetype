import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getStore, DEFAULTS } from './lib/store';
import { Sidebar, SectionId } from './components/Sidebar';
import { GeneralSection } from './components/sections/GeneralSection';
import { VocabularySection } from './components/sections/VocabularySection';
import { ModelSection } from './components/sections/ModelSection';
import { MicrophoneSection } from './components/sections/MicrophoneSection';
import { AppearanceSection } from './components/sections/AppearanceSection';
import { HistorySection } from './components/sections/HistorySection';
import { FirstRun } from './components/FirstRun';
import { useUpdater } from './lib/updater';
import { UpdateBanner } from './components/UpdateBanner';

interface FirstRunStatus {
  needsSetup: boolean;
  gpuDetected: boolean;
  gpuName: string;
  directmlAvailable: boolean;
  recommendedModel: string;
}

function App() {
  const [activeSection, setActiveSection] = useState<SectionId>('general');
  const updater = useUpdater();
  const [hotkey, setHotkey] = useState(DEFAULTS.hotkey);
  const [theme, setTheme] = useState<'light' | 'dark'>(DEFAULTS.theme);
  const [recordingMode, setRecordingMode] = useState<'hold' | 'toggle'>(DEFAULTS.recordingMode);
  const [selectedMic, setSelectedMic] = useState(DEFAULTS.selectedMic);
  const [selectedModel, setSelectedModel] = useState(DEFAULTS.selectedModel);
  const [loaded, setLoaded] = useState(false);
  const [firstRunStatus, setFirstRunStatus] = useState<FirstRunStatus | null>(null);
  const [hookAvailable, setHookAvailable] = useState(true); // default true = no warning

  // Hook status uses a two-flag handshake to avoid the Tauri startup event-loss race.
  //
  // Problem: Tauri events are fire-and-forget with no queue. setup() emits
  // "hook-status-changed" once, but if the JS listener hasn't registered yet
  // (listen() round-trip not complete, or webview JS not yet loaded), the event
  // is silently dropped (Tauri issue #3484).
  //
  // Solution: After registering the listener, call notify_frontend_ready().
  // The backend coordinates via SetupComplete + FrontendReady flags:
  //   - If setup() has already finished: notify_frontend_ready emits immediately.
  //   - If setup() is still running: setup() emits when it completes (sees FrontendReady=true).
  // Either way, the emit happens after the listener is guaranteed to be registered.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<boolean>('hook-status-changed', (event) => {
      if (!cancelled) setHookAvailable(event.payload);
    }).then((fn) => {
      unlisten = fn;
      // Listener is now registered — signal the backend it can safely emit.
      if (!cancelled) {
        invoke('notify_frontend_ready').catch(() => {});
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    async function loadSettings() {
      try {
        const status = await invoke<FirstRunStatus>('check_first_run');
        setFirstRunStatus(status);
      } catch {
        // check_first_run unavailable (whisper feature not compiled) — skip first-run gate
        setFirstRunStatus({ needsSetup: false, gpuDetected: false, gpuName: '', directmlAvailable: false, recommendedModel: '' });
      }

      const store = await getStore();
      const savedHotkey = await store.get<string>('hotkey');
      const savedTheme = await store.get<'light' | 'dark'>('theme');
      const savedRecordingMode = await store.get<'hold' | 'toggle'>('recordingMode');
      const savedSelectedMic = await store.get<string>('selectedMic');
      const savedSelectedModel = await store.get<string>('selectedModel');

      if (savedHotkey) setHotkey(savedHotkey);

      const resolvedTheme = savedTheme ?? DEFAULTS.theme;
      setTheme(resolvedTheme);

      // Apply theme to DOM
      if (resolvedTheme === 'dark') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }

      if (savedRecordingMode) setRecordingMode(savedRecordingMode);
      if (savedSelectedMic) setSelectedMic(savedSelectedMic);
      // Reconcile selectedModel with the backend's actual engine state.
      // The Tauri store and Rust settings.json can desync (separate I/O paths),
      // so the backend engine is the source of truth.
      try {
        const backendEngine = await invoke<string>('get_engine');
        if (backendEngine === 'parakeet') {
          setSelectedModel('parakeet-tdt-v2-fp32');
        } else if (savedSelectedModel && savedSelectedModel !== 'parakeet-tdt-v2-fp32') {
          setSelectedModel(savedSelectedModel);
        } else {
          // Backend is whisper but store says parakeet — pick the active whisper model
          const models = await invoke<{ id: string; available: boolean }[]>('list_models');
          const available = models.find(m => m.available && m.id !== 'parakeet-tdt-v2-fp32');
          setSelectedModel(available?.id ?? '');
        }
      } catch {
        // get_engine not available — fall back to stored value
        if (savedSelectedModel !== null && savedSelectedModel !== undefined) {
          setSelectedModel(savedSelectedModel);
        }
      }

      // Hook status is not queried here to avoid reading during the startup race window
      // (webview2 COM init allows IPC before setup() has installed the hook, so any
      // query here may return a stale false). Instead, the listen effect above registers
      // a listener and re-queries after registration, which is race-safe.

      setLoaded(true);
    }

    loadSettings().catch((err) => console.error('Failed to load settings:', err));
  }, []);

  if (!loaded) {
    return (
      <div className="flex h-screen items-center justify-center bg-white dark:bg-gray-900">
        <div className="text-sm text-gray-400">Loading...</div>
      </div>
    );
  }

  if (firstRunStatus?.needsSetup) {
    return (
      <div className="flex h-screen bg-white dark:bg-gray-900">
        <FirstRun
          gpuDetected={firstRunStatus.gpuDetected}
          gpuName={firstRunStatus.gpuName}
          directmlAvailable={firstRunStatus.directmlAvailable}
          recommendedModel={firstRunStatus.recommendedModel}
          onComplete={async (downloadedModelId) => {
            try {
              const store = await getStore();
              await store.set('selectedModel', downloadedModelId);
              setSelectedModel(downloadedModelId);
            } catch (e) {
              console.warn('Failed to save selected model:', e);
            }
            try {
              await invoke('set_model', { modelId: downloadedModelId });
            } catch (e) {
              console.warn('Failed to load whisper model (will load on next start):', e);
            }
            setFirstRunStatus({ ...firstRunStatus, needsSetup: false });
          }}
        />
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-white dark:bg-gray-900">
      <Sidebar activeSection={activeSection} onSelect={setActiveSection} />
      <div className="flex flex-1 flex-col">
        <UpdateBanner
          state={updater.state}
          onDownload={updater.startDownload}
          onCancel={updater.cancelDownload}
          onRestart={updater.restartNow}
          onLater={updater.restartLater}
          onDismiss={updater.dismiss}
          onRetry={updater.checkNow}
        />
        <main className="flex-1 overflow-y-auto px-6 py-5 text-gray-900 dark:text-gray-100">
          {activeSection === 'general' && (
            <GeneralSection
              hotkey={hotkey}
              onHotkeyChange={(newKey) => {
                setHotkey(newKey);
                // Re-query hook status — rebind_hotkey may have installed the hook on-demand
                invoke<boolean>('get_hook_status').then(setHookAvailable).catch(() => {});
              }}
              recordingMode={recordingMode}
              onRecordingModeChange={setRecordingMode}
              updaterState={updater.state}
              onCheckForUpdates={updater.checkNow}
              hookAvailable={hookAvailable}
            />
          )}
          {activeSection === 'vocabulary' && <VocabularySection />}
          {activeSection === 'history' && <HistorySection />}
          {activeSection === 'model' && (
            <ModelSection
              selectedModel={selectedModel}
              onSelectedModelChange={setSelectedModel}
            />
          )}
          {activeSection === 'microphone' && (
            <MicrophoneSection
              selectedMic={selectedMic}
              onSelectedMicChange={setSelectedMic}
            />
          )}
          {activeSection === 'appearance' && (
            <AppearanceSection theme={theme} onThemeChange={setTheme} />
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
