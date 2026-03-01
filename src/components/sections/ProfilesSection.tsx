interface ProfilesSectionProps {
  activeProfileId: string;
  onActiveProfileChange: (id: string) => void;
}

export function ProfilesSection({ activeProfileId, onActiveProfileChange }: ProfilesSectionProps) {
  return (
    <div>
      <h1 className="mb-5 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Profiles
      </h1>
      <p className="text-sm text-gray-400">Loading profiles...</p>
    </div>
  );
}
