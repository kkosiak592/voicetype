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

// Maps KeyboardEvent.code to canonical modifier token.
// Uses e.code (not e.ctrlKey/e.metaKey flags) because on keyup the flags
// reflect the post-release state (already false for the released key).
function modifierToken(code: string): string | null {
  switch (code) {
    case 'ControlLeft':
    case 'ControlRight':
      return 'ctrl';
    case 'MetaLeft':
    case 'MetaRight':
      return 'meta';
    case 'AltLeft':
    case 'AltRight':
      return 'alt';
    case 'ShiftLeft':
    case 'ShiftRight':
      return 'shift';
    default:
      return null;
  }
}

// Canonical sort order for modifier tokens — ensures stored combo is
// deterministic regardless of press order.
const MODIFIER_ORDER: Record<string, number> = { ctrl: 0, alt: 1, shift: 2, meta: 3 };

export function HotkeyCapture({ value, onChange }: HotkeyCaptureProps) {
  const [listening, setListening] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [heldDisplay, setHeldDisplay] = useState<string>('');
  const boxRef = useRef<HTMLDivElement>(null);
  // Tracks currently held modifier tokens across keydown/keyup events.
  // useRef (not useState) because mutations should not trigger re-renders.
  const heldRef = useRef<Set<string>>(new Set());
  // Tracks ALL modifiers pressed during this capture session (not depleted by keyup).
  // Used to build the final combo when all keys are released.
  const comboRef = useRef<Set<string>>(new Set());

  // Unregister the global hotkey when entering capture mode so that
  // pressing the current key combo gets captured as input rather than
  // triggering the shortcut action. Re-register when leaving capture
  // mode without a change (Escape / click-away).
  useEffect(() => {
    if (listening && value) {
      invoke('unregister_hotkey', { key: value }).catch(() => { });
    }
  }, [listening, value]);

  useEffect(() => {
    if (!listening) return;

    const handleKeyDown = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Ignore key repeat events to avoid re-adding already-tracked modifiers.
      if (e.repeat) return;

      if (e.code === 'Escape') {
        heldRef.current.clear();
        comboRef.current.clear();
        setHeldDisplay('');
        setListening(false);
        // Re-register the original hotkey since the user cancelled.
        if (value) {
          invoke('register_hotkey', { key: value }).catch(() => { });
        }
        return;
      }

      const token = modifierToken(e.code);
      if (token) {
        // Modifier key pressed — add to held set and combo set, update progressive display.
        heldRef.current.add(token);
        comboRef.current.add(token);
        const sortedTokens = [...heldRef.current].sort(
          (a, b) => (MODIFIER_ORDER[a] ?? 99) - (MODIFIER_ORDER[b] ?? 99)
        );
        setHeldDisplay(formatHotkey(sortedTokens.join('+')));
        return; // Wait for more keys or keyup
      }

      // Non-modifier key pressed — clear held/combo sets and proceed with standard combo path.
      heldRef.current.clear();
      comboRef.current.clear();
      setHeldDisplay('');

      const combo = normalizeKey(e);
      if (!combo) return;

      setListening(false);
      setError(null);

      if (combo === value) {
        // Same key pressed — just re-register it, no persist needed.
        invoke('register_hotkey', { key: value }).catch(() => { });
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
          invoke('register_hotkey', { key: value }).catch(() => { });
        }
        setError(String(err));
      }
    };

    const handleKeyUp = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const token = modifierToken(e.code);
      if (!token) {
        // Non-modifier released — clear held set.
        heldRef.current.clear();
        setHeldDisplay('');
        return;
      }

      // A modifier was released. Remove from held set (tracks what's currently down).
      heldRef.current.delete(token);

      if (heldRef.current.size === 0 && comboRef.current.size > 0) {
        // All modifiers released — use comboRef (full session set, not depleted by keyup).
        const tokens = [...comboRef.current].sort(
          (a, b) => (MODIFIER_ORDER[a] ?? 99) - (MODIFIER_ORDER[b] ?? 99)
        );
        const combo = tokens.join('+');
        comboRef.current.clear();
        setListening(false);
        setError(null);
        setHeldDisplay('');

        if (combo === value) {
          // Same combo — just re-register it, no persist needed.
          invoke('register_hotkey', { key: value }).catch(() => { });
          return;
        }

        try {
          await invoke('rebind_hotkey', { old: '', newKey: combo });
          const store = await getStore();
          await store.set('hotkey', combo);
          onChange(combo);
        } catch (err) {
          // Registration failed — restore the original hotkey.
          if (value) {
            invoke('register_hotkey', { key: value }).catch(() => { });
          }
          setError(String(err));
        }
      } else if (heldRef.current.size > 0) {
        // Some modifiers still held — update the progressive display.
        const remainingTokens = [...heldRef.current].sort(
          (a, b) => (MODIFIER_ORDER[a] ?? 99) - (MODIFIER_ORDER[b] ?? 99)
        );
        setHeldDisplay(formatHotkey(remainingTokens.join('+')));
      }
    };

    // Also handle click-away: if the user clicks outside the capture box,
    // cancel listening and re-register the original hotkey.
    const handleClickOutside = (e: MouseEvent) => {
      if (boxRef.current && !boxRef.current.contains(e.target as Node)) {
        heldRef.current.clear();
        comboRef.current.clear();
        setHeldDisplay('');
        setListening(false);
        if (value) {
          invoke('register_hotkey', { key: value }).catch(() => { });
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    window.addEventListener('keyup', handleKeyUp, true);
    document.addEventListener('mousedown', handleClickOutside, true);
    return () => {
      window.removeEventListener('keydown', handleKeyDown, true);
      window.removeEventListener('keyup', handleKeyUp, true);
      document.removeEventListener('mousedown', handleClickOutside, true);
      heldRef.current.clear();
      comboRef.current.clear();
      setHeldDisplay('');
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
          case 'win': return 'Win';
          case 'space': return 'Space';
          default: return part.toUpperCase();
        }
      })
      .join(' + ');
  }

  return (
    <div className="w-full">
      <div
        ref={boxRef}
        tabIndex={0}
        onClick={() => {
          setListening(true);
          setError(null);
          boxRef.current?.focus();
        }}
        className={[
          'cursor-pointer select-none rounded-xl px-5 py-3.5 text-sm font-medium font-mono',
          'transition-all duration-200 outline-none flex items-center justify-center max-w-sm',
          listening
            ? 'ring-2 ring-emerald-500 bg-emerald-50 text-emerald-700 dark:bg-emerald-500/10 dark:text-emerald-300 dark:ring-emerald-500/80 shadow-[0_0_15px_rgba(99,102,241,0.2)]'
            : 'ring-1 ring-gray-300 bg-gray-50 text-gray-800 hover:ring-gray-400 hover:bg-white dark:ring-gray-700 dark:bg-gray-800/50 dark:text-gray-200 dark:hover:bg-gray-800 dark:hover:ring-gray-600 shadow-inner',
        ].join(' ')}
      >
        <div className="flex gap-1.5 items-center justify-center">
          {listening
            ? heldDisplay
              ? <span className="animate-pulse">{`${heldDisplay}...`}</span>
              : 'Press a key combo...'
            : formatHotkey(value).split(' + ').map((key, i, arr) => (
              <span key={i} className="flex items-center">
                <span className="bg-white dark:bg-gray-700 px-2 py-1 rounded shadow-sm border border-gray-200 dark:border-gray-600 text-gray-700 dark:text-gray-300">
                  {key}
                </span>
                {i < arr.length - 1 && <span className="mx-1.5 text-gray-400 dark:text-gray-500">+</span>}
              </span>
            ))}
        </div>
      </div>
      {error && (
        <p className="mt-2 text-sm text-red-500 dark:text-red-400">{error}</p>
      )}
    </div>
  );
}
