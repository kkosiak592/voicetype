import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ProfileSwitcher, ProfileInfo } from '../ProfileSwitcher';
import { DictionaryEditor } from '../DictionaryEditor';
import { getStore } from '../../lib/store';

interface ProfilesSectionProps {
  activeProfileId: string;
  onActiveProfileChange: (id: string) => void;
}

export function ProfilesSection({ activeProfileId, onActiveProfileChange }: ProfilesSectionProps) {
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [corrections, setCorrections] = useState<Record<string, string>>({});
  const [allCaps, setAllCaps] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadInitial() {
      // Sync backend with frontend's active profile before reading corrections
      await invoke('set_active_profile', { profileId: activeProfileId });
      const [profileList, correctionMap] = await Promise.all([
        invoke<ProfileInfo[]>('get_profiles'),
        invoke<Record<string, string>>('get_corrections'),
      ]);
      setProfiles(profileList);
      setCorrections(correctionMap);
      setLoading(false);
    }
    loadInitial().catch(err => {
      console.error('Failed to load profiles:', err);
      setLoading(false);
    });
  }, [activeProfileId]);

  async function handleProfileSelect(id: string) {
    try {
      await invoke('set_active_profile', { profileId: id });
      const store = await getStore();
      await store.set('activeProfile', id);
      const correctionMap = await invoke<Record<string, string>>('get_corrections');
      setCorrections(correctionMap);
      onActiveProfileChange(id);
    } catch (err) {
      console.error('Failed to switch profile:', err);
    }
  }

  async function handleAllCapsToggle() {
    const next = !allCaps;
    try {
      await invoke('set_all_caps', { enabled: next });
      setAllCaps(next);
    } catch (err) {
      console.error('Failed to toggle ALL CAPS:', err);
    }
  }

  async function handleCorrectionsChange(updated: Record<string, string>) {
    try {
      await invoke('save_corrections', { corrections: updated });
      setCorrections(updated);
    } catch (err) {
      console.error('Failed to save corrections:', err);
    }
  }

  if (loading) {
    return (
      <div>
        <h1 className="mb-5 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
          Profiles
        </h1>
        <div className="space-y-2">
          <div className="h-16 animate-pulse rounded-lg bg-gray-100 dark:bg-gray-800" />
          <div className="h-16 animate-pulse rounded-lg bg-gray-100 dark:bg-gray-800" />
        </div>
      </div>
    );
  }

  return (
    <div>
      <h1 className="mb-4 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Profiles
      </h1>

      <div className="space-y-4">
        {/* Profile switcher */}
        <ProfileSwitcher
          profiles={profiles}
          activeId={activeProfileId}
          onSelect={handleProfileSelect}
        />

        {/* ALL CAPS toggle */}
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-900 dark:text-gray-100">ALL CAPS output</p>
            <p className="text-xs text-gray-500 dark:text-gray-400">
              Uppercase all injected text for engineering drawings
            </p>
          </div>
          <button
            onClick={handleAllCapsToggle}
            className={[
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 focus:outline-none',
              allCaps ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-600',
            ].join(' ')}
            role="switch"
            aria-checked={allCaps}
          >
            <span
              className={[
                'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform duration-200',
                allCaps ? 'translate-x-6' : 'translate-x-1',
              ].join(' ')}
            />
          </button>
        </div>

        <hr className="border-gray-200 dark:border-gray-700" />

        {/* Corrections dictionary */}
        <section>
          <h2 className="mb-2 text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400">
            Corrections Dictionary
          </h2>
          <DictionaryEditor corrections={corrections} onChange={handleCorrectionsChange} />
        </section>
      </div>
    </div>
  );
}
