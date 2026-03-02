import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getStore } from '../lib/store';

interface HotkeyCaptureProps {
  value: string;
  onChange: (newHotkey: string) => void;
}

function normalizeKey(e: KeyboardEvent): string | null {
  const modifiers: string[] = [];
  if (e.ctrlKey) modifiers.push('ctrl');
  if (e.altKey) modifiers.push('alt');
  if (e.shiftKey) modifiers.push('shift');
  if (e.metaKey) modifiers.push('meta');

  // Determine the base key from e.code
  let baseKey: string;
  const code = e.code;

  if (code.startsWith('Key')) {
    // e.g. "KeyA" -> "a"
    baseKey = code.slice(3).toLowerCase();
  } else if (code.startsWith('Digit')) {
    // e.g. "Digit1" -> "1"
    baseKey = code.slice(5);
  } else if (code === 'Space') {
    baseKey = 'space';
  } else if (code.startsWith('F') && /^F\d+$/.test(code)) {
    // e.g. "F5" -> "f5"
    baseKey = code.toLowerCase();
  } else if (
    code === 'Backspace' ||
    code === 'Delete' ||
    code === 'Insert' ||
    code === 'Home' ||
    code === 'End' ||
    code === 'PageUp' ||
    code === 'PageDown' ||
    code === 'Enter' ||
    code === 'Tab' ||
    code === 'Escape' ||
    code.startsWith('Arrow')
  ) {
    // Navigation/control keys — map to lowercase
    baseKey = code.toLowerCase().replace('arrow', '');
  } else {
    // Punctuation, brackets, etc. — use e.key as fallback, lowercase
    const key = e.key.toLowerCase();
    // Skip pure modifier keys
    if (['control', 'alt', 'shift', 'meta'].includes(key)) {
      return null;
    }
    baseKey = key;
  }

  // Must have at least one modifier to be a valid global hotkey
  if (modifiers.length === 0) return null;

  return [...modifiers, baseKey].join('+');
}

export function HotkeyCapture({ value, onChange }: HotkeyCaptureProps) {
  const [listening, setListening] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const boxRef = useRef<HTMLDivElement>(null);

  // Unregister the global hotkey when entering capture mode so that
  // pressing the current key combo gets captured as input rather than
  // triggering the shortcut action. Re-register when leaving capture
  // mode without a change (Escape / click-away).
  useEffect(() => {
    if (listening && value) {
      invoke('unregister_hotkey', { key: value }).catch(() => {});
    }
  }, [listening, value]);

  useEffect(() => {
    if (!listening) return;

    const handleKeyDown = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.code === 'Escape') {
        setListening(false);
        // Re-register the original hotkey since the user cancelled.
        if (value) {
          invoke('register_hotkey', { key: value }).catch(() => {});
        }
        return;
      }

      const combo = normalizeKey(e);
      if (!combo) return; // Modifier-only — wait for a real key

      setListening(false);
      setError(null);

      if (combo === value) {
        // Same key pressed — just re-register it, no persist needed.
        invoke('register_hotkey', { key: value }).catch(() => {});
        return;
      }

      try {
        // Old key is already unregistered; just register the new one.
        await invoke('rebind_hotkey', { old: '', newKey: combo });
        const store = await getStore();
        await store.set('hotkey', combo);
        onChange(combo);
      } catch (err) {
        // Registration failed — restore the original hotkey.
        if (value) {
          invoke('register_hotkey', { key: value }).catch(() => {});
        }
        setError(String(err));
      }
    };

    // Also handle click-away: if the user clicks outside the capture box,
    // cancel listening and re-register the original hotkey.
    const handleClickOutside = (e: MouseEvent) => {
      if (boxRef.current && !boxRef.current.contains(e.target as Node)) {
        setListening(false);
        if (value) {
          invoke('register_hotkey', { key: value }).catch(() => {});
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    document.addEventListener('mousedown', handleClickOutside, true);
    return () => {
      window.removeEventListener('keydown', handleKeyDown, true);
      document.removeEventListener('mousedown', handleClickOutside, true);
    };
  }, [listening, value, onChange]);

  function formatHotkey(hotkey: string): string {
    return hotkey
      .split('+')
      .map((part) => {
        switch (part) {
          case 'ctrl': return 'Ctrl';
          case 'alt': return 'Alt';
          case 'shift': return 'Shift';
          case 'meta': return 'Win';
          case 'space': return 'Space';
          default: return part.toUpperCase();
        }
      })
      .join(' + ');
  }

  return (
    <div>
      <div
        ref={boxRef}
        tabIndex={0}
        onClick={() => {
          setListening(true);
          setError(null);
          boxRef.current?.focus();
        }}
        className={[
          'cursor-pointer select-none rounded-md px-4 py-2 text-sm font-mono',
          'border-2 transition-colors duration-150 outline-none',
          listening
            ? 'border-blue-500 bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300 dark:border-blue-400'
            : 'border-gray-300 bg-gray-100 text-gray-800 hover:border-gray-400 dark:border-gray-600 dark:bg-gray-700 dark:text-gray-200',
        ].join(' ')}
      >
        {listening ? 'Press a key combo...' : formatHotkey(value)}
      </div>
      {error && (
        <p className="mt-1 text-xs text-red-500">{error}</p>
      )}
    </div>
  );
}
