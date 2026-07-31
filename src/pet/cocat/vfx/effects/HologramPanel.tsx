const codeLines = ["scan bus", "patch mem", "sync temp", "ok"];

export function HologramPanel() {
  return (
    <div className="cocat-hologram-panel" aria-hidden="true">
      <span className="cocat-hologram-grid" />
      <div className="cocat-hologram-code">
        {codeLines.concat(codeLines).map((line, index) => (
          <span key={`${line}-${index}`}>{line}</span>
        ))}
      </div>
    </div>
  );
}
