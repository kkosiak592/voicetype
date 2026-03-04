import { useState, useEffect, useRef, useCallback } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invoke } from '@tauri-apps/api/core';

export type UpdateStatus =
  | 'idle'         // No update activity
  | 'checking'     // Checking for update
  | 'available'    // Update found, waiting for user action
  | 'downloading'  // Download in progress
  | 'ready'        // Downloaded, waiting for restart
  | 'error';       // Check or download failed

export interface UpdateState {
  status: UpdateStatus;
  version: string;
  body: string;           // Release notes markdown
  downloaded: number;     // Bytes downloaded
  contentLength: number;  // Total bytes (0 if unknown)
  errorMessage: string;
  lastChecked: number;    // Timestamp of last completed check (0 if never)
  dismissed: boolean;     // Whether the banner was dismissed this session
}

export interface UseUpdaterReturn {
  state: UpdateState;
  checkNow: () => Promise<void>;
  startDownload: () => Promise<void>;
  cancelDownload: () => void;
  restartNow: () => Promise<void>;
  restartLater: () => void;
  dismiss: () => void;
}

const FOUR_HOURS_MS = 4 * 60 * 60 * 1000;
const LAUNCH_DELAY_MS = 4000;

const INITIAL_STATE: UpdateState = {
  status: 'idle',
  version: '',
  body: '',
  downloaded: 0,
  contentLength: 0,
  errorMessage: '',
  lastChecked: 0,
  dismissed: false,
};

export function useUpdater(): UseUpdaterReturn {
  const [state, setState] = useState<UpdateState>(INITIAL_STATE);
  const stateRef = useRef<UpdateState>(INITIAL_STATE);
  const updateRef = useRef<Update | null>(null);
  const cancelledRef = useRef(false);
  const isCheckingRef = useRef(false);

  const checkNow = useCallback(async () => {
    // Don't interrupt active download or ready state
    if (isCheckingRef.current) return;
    isCheckingRef.current = true;

    setState(prev => ({ ...prev, status: 'checking', errorMessage: '' }));

    try {
      const update = await check();

      if (update) {
        updateRef.current = update;
        setState(prev => ({
          ...prev,
          status: 'available',
          version: update.version,
          body: update.body ?? '',
          downloaded: 0,
          contentLength: 0,
          lastChecked: Date.now(),
          dismissed: false,
        }));

        // Notify tray — fire and forget
        invoke('set_update_available', { available: true }).catch(() => {});
      } else {
        setState(prev => ({
          ...prev,
          status: 'idle',
          version: '',
          body: '',
          lastChecked: Date.now(),
        }));
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setState(prev => ({
        ...prev,
        status: 'error',
        errorMessage: msg,
        lastChecked: Date.now(),
      }));
    } finally {
      isCheckingRef.current = false;
    }
  }, []);

  const startDownload = useCallback(async () => {
    if (!updateRef.current) return;

    cancelledRef.current = false;

    setState(prev => ({
      ...prev,
      status: 'downloading',
      downloaded: 0,
      contentLength: 0,
      errorMessage: '',
    }));

    let totalDownloaded = 0;

    try {
      await updateRef.current.downloadAndInstall((progress) => {
        if (cancelledRef.current) return;

        if (progress.event === 'Started') {
          const total = progress.data.contentLength ?? 0;
          setState(prev => ({ ...prev, contentLength: total }));
        } else if (progress.event === 'Progress') {
          totalDownloaded += progress.data.chunkLength ?? 0;
          const downloaded = totalDownloaded;
          setState(prev => ({ ...prev, downloaded }));
        } else if (progress.event === 'Finished') {
          // Will be handled after promise resolves
        }
      });

      if (cancelledRef.current) {
        // Download completed but user cancelled UI — go back to available
        setState(prev => ({ ...prev, status: 'available', downloaded: 0, contentLength: 0 }));
      } else {
        setState(prev => ({ ...prev, status: 'ready' }));
      }
    } catch (e) {
      if (cancelledRef.current) {
        // Ignore errors from cancelled downloads — reset to available
        setState(prev => ({ ...prev, status: 'available', downloaded: 0, contentLength: 0 }));
      } else {
        const msg = e instanceof Error ? e.message : String(e);
        setState(prev => ({ ...prev, status: 'error', errorMessage: msg }));
      }
    }
  }, []);

  const cancelDownload = useCallback(() => {
    // The JS plugin API does not support real cancellation.
    // Set cancelled flag — the UI stops showing progress and resets to 'available'.
    // The actual download continues in the background (plugin limitation).
    cancelledRef.current = true;
    setState(prev => ({ ...prev, status: 'available', downloaded: 0, contentLength: 0 }));
  }, []);

  const restartNow = useCallback(async () => {
    // Check if recording/dictation is active before relaunching
    // Poll until pipeline is idle
    const POLL_INTERVAL_MS = 500;
    const MAX_WAIT_MS = 60000; // Wait at most 60 seconds
    const startWait = Date.now();

    setState(prev => ({ ...prev, errorMessage: '' }));

    // eslint-disable-next-line no-constant-condition
    while (true) {
      try {
        const isActive = await invoke<boolean>('is_pipeline_active');
        if (!isActive) break;
      } catch {
        // Command unavailable — proceed with relaunch
        break;
      }

      if (Date.now() - startWait > MAX_WAIT_MS) break;

      setState(prev => ({
        ...prev,
        errorMessage: 'Waiting for dictation to finish...',
      }));

      await new Promise(resolve => setTimeout(resolve, POLL_INTERVAL_MS));
    }

    setState(prev => ({ ...prev, errorMessage: '' }));

    try {
      await relaunch();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setState(prev => ({ ...prev, status: 'error', errorMessage: msg }));
    }
  }, []);

  const restartLater = useCallback(() => {
    // downloadAndInstall() already wrote update to disk.
    // Next app launch applies it automatically.
    // Just dismiss the banner.
    setState(prev => ({ ...prev, dismissed: true }));
  }, []);

  const dismiss = useCallback(() => {
    setState(prev => ({ ...prev, dismissed: true }));
  }, []);

  // Keep stateRef in sync so the periodic interval can read status without re-registering
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  // Launch-time check: delay 4 seconds so startup is not blocked
  useEffect(() => {
    const timer = setTimeout(() => {
      checkNow();
    }, LAUNCH_DELAY_MS);

    return () => clearTimeout(timer);
    // checkNow is stable (useCallback with empty deps) — safe to omit from deps
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Periodic check every 4 hours — only when not downloading or ready
  useEffect(() => {
    const interval = setInterval(() => {
      // Don't check during active download/ready states
      if (['downloading', 'ready', 'checking'].some(s => stateRef.current?.status === s)) {
        return;
      }
      checkNow();
    }, FOUR_HOURS_MS);

    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    state,
    checkNow,
    startDownload,
    cancelDownload,
    restartNow,
    restartLater,
    dismiss,
  };
}
