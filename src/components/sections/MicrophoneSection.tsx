interface MicrophoneSectionProps {
  selectedMic: string;
  onSelectedMicChange: (device: string) => void;
}

export function MicrophoneSection({ selectedMic, onSelectedMicChange }: MicrophoneSectionProps) {
  return (
    <div>
      <h1 className="mb-5 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Microphone
      </h1>
      <p className="text-sm text-gray-400">Loading devices...</p>
    </div>
  );
}
