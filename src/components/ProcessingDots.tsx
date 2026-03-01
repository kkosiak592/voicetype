export function ProcessingDots() {
  return (
    <div className="flex items-center gap-[4px]">
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="w-[5px] h-[5px] rounded-full pill-dot-bounce"
          style={{
            background: "linear-gradient(135deg, #a78bfa, #818cf8)",
            animationDelay: `${i * 150}ms`,
          }}
        />
      ))}
    </div>
  );
}
