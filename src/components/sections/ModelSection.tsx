interface ModelSectionProps {
  selectedModel: string;
  onSelectedModelChange: (id: string) => void;
}

export function ModelSection({ selectedModel, onSelectedModelChange }: ModelSectionProps) {
  return (
    <div>
      <h1 className="mb-5 text-base font-semibold tracking-tight text-gray-900 dark:text-gray-100">
        Model
      </h1>
      <p className="text-sm text-gray-400">Loading models...</p>
    </div>
  );
}
