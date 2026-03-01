export type SectionId = 'general' | 'profiles' | 'model' | 'microphone' | 'appearance';

interface SidebarItem {
  id: SectionId;
  label: string;
  icon: string;
}

const ITEMS: SidebarItem[] = [
  { id: 'general', label: 'General', icon: '⌨' },
  { id: 'profiles', label: 'Profiles', icon: '◈' },
  { id: 'model', label: 'Model', icon: '◎' },
  { id: 'microphone', label: 'Microphone', icon: '◉' },
  { id: 'appearance', label: 'Appearance', icon: '◐' },
];

interface SidebarProps {
  activeSection: SectionId;
  onSelect: (id: SectionId) => void;
}

export function Sidebar({ activeSection, onSelect }: SidebarProps) {
  return (
    <nav className="flex w-44 flex-col border-r border-gray-200 bg-gray-50 pt-4 dark:border-gray-700 dark:bg-gray-800/50">
      {ITEMS.map((item) => {
        const isActive = activeSection === item.id;
        return (
          <button
            key={item.id}
            onClick={() => onSelect(item.id)}
            className={[
              'flex items-center gap-2.5 px-4 py-2.5 text-sm transition-colors duration-100 focus:outline-none text-left',
              isActive
                ? 'bg-indigo-50 text-indigo-600 dark:bg-indigo-950 dark:text-indigo-400'
                : 'text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-800',
            ].join(' ')}
          >
            <span className="text-base leading-none" aria-hidden="true">
              {item.icon}
            </span>
            <span>{item.label}</span>
          </button>
        );
      })}
    </nav>
  );
}
