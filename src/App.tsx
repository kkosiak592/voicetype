import { useEffect, useState } from 'react';
import { getStore, DEFAULTS } from './lib/store';
import { Sidebar, SectionId } from './components/Sidebar';
import { GeneralSection } from './components/sections/GeneralSection';
import { ProfilesSection } from './components/sections/ProfilesSection';
import { ModelSection } from './components/sections/ModelSection';
import { MicrophoneSection } from './components/sections/MicrophoneSection';
import { AppearanceSection } from './components/sections/AppearanceSection';

function App() {
  const [activeSection, setActiveSection] = useState<SectionId>('general');
  const [hotkey, setHotkey] = useState(DEFAULTS.hotkey);
  const [theme, setTheme] = useState<'light' | 'dark'>(DEFAULTS.theme);
  const [recordingMode, setRecordingMode] = useState<'hold' | 'toggle'>(DEFAULTS.recordingMode);
  const [activeProfile, setActiveProfile] = useState(DEFAULTS.activeProfile);
  const [selectedMic, setSelectedMic] = useState(DEFAULTS.selectedMic);
  const [selectedModel, setSelectedModel] = useState(DEFAULTS.selectedModel);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    async function loadSettings() {
      const store = await getStore();
      const savedHotkey = await store.get<string>('hotkey');
      const savedTheme = await store.get<'light' | 'dark'>('theme');
      const savedRecordingMode = await store.get<'hold' | 'toggle'>('recordingMode');
      const savedActiveProfile = await store.get<string>('activeProfile');
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
      if (savedActiveProfile) setActiveProfile(savedActiveProfile);
      if (savedSelectedMic) setSelectedMic(savedSelectedMic);
      if (savedSelectedModel !== null && savedSelectedModel !== undefined) {
        setSelectedModel(savedSelectedModel);
      }

      setLoaded(true);
    }

    loadSettings();
  }, []);

  if (!loaded) {
    return (
      <div className="flex h-screen items-center justify-center bg-white dark:bg-gray-900">
        <div className="text-sm text-gray-400">Loading...</div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-white dark:bg-gray-900">
      <Sidebar activeSection={activeSection} onSelect={setActiveSection} />
      <main className="flex-1 overflow-hidden px-6 py-5 text-gray-900 dark:text-gray-100">
        {activeSection === 'general' && (
          <GeneralSection
            hotkey={hotkey}
            onHotkeyChange={setHotkey}
            recordingMode={recordingMode}
            onRecordingModeChange={setRecordingMode}
          />
        )}
        {activeSection === 'profiles' && (
          <ProfilesSection
            activeProfileId={activeProfile}
            onActiveProfileChange={setActiveProfile}
          />
        )}
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
  );
}

export default App;
