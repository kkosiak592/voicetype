import "../pill.css";

export function ProcessingDots() {
  return (
    <div className="flex items-center justify-center gap-1.5 w-full h-full relative z-10">
      <div className="w-2 h-2 rounded-full bg-cyan-400 processing-dot" style={{ animationDelay: "0ms" }} />
      <div className="w-2 h-2 rounded-full bg-indigo-400 processing-dot" style={{ animationDelay: "150ms" }} />
      <div className="w-2 h-2 rounded-full bg-purple-400 processing-dot" style={{ animationDelay: "300ms" }} />
    </div>
  );
}
