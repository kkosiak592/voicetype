import { invoke } from '@tauri-apps/api/core';

export interface ProfileInfo {
  id: string;
  name: string;
  active: boolean;
}

const PROFILE_DESCRIPTIONS: Record<string, string> = {
  structural_engineering: 'Engineering terminology bias with domain corrections',
  general: 'No domain bias, default settings',
};

interface ProfileSwitcherProps {
  profiles: ProfileInfo[];
  activeId: string;
  onSelect: (id: string) => void;
}

export function ProfileSwitcher({ profiles, activeId, onSelect }: ProfileSwitcherProps) {
  async function handleSelect(profileId: string) {
    if (profileId === activeId) return;
    await invoke('set_active_profile', { profileId });
    onSelect(profileId);
  }

  return (
    <div className="flex gap-3">
      {profiles.map((profile) => {
        const isSelected = activeId === profile.id;
        const description = PROFILE_DESCRIPTIONS[profile.id] ?? 'Custom profile';
        return (
          <button
            key={profile.id}
            onClick={() => handleSelect(profile.id)}
            className={[
              'flex flex-1 flex-col rounded-lg border-2 px-3 py-2.5 text-left transition-colors duration-150 focus:outline-none',
              isSelected
                ? 'border-indigo-500 bg-indigo-50 dark:border-indigo-400 dark:bg-indigo-950'
                : 'border-gray-200 bg-white hover:border-gray-300 dark:border-gray-700 dark:bg-gray-800 dark:hover:border-gray-600',
            ].join(' ')}
          >
            <span
              className={[
                'text-sm font-medium',
                isSelected
                  ? 'text-indigo-700 dark:text-indigo-300'
                  : 'text-gray-900 dark:text-gray-100',
              ].join(' ')}
            >
              {profile.name}
            </span>
            <span className="mt-0.5 text-xs text-gray-500 dark:text-gray-400">
              {description}
            </span>
          </button>
        );
      })}
    </div>
  );
}
