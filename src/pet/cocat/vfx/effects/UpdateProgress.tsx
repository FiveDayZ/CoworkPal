import type React from "react";

export function UpdateProgress({ progress }: { progress: number }) {
  const clampedProgress = Math.max(0, Math.min(1, progress));

  return (
    <div
      className="cocat-update-progress"
      style={
        {
          "--cocat-update-progress": clampedProgress,
        } as React.CSSProperties
      }
      aria-hidden="true"
    >
      <span className="cocat-update-track" />
      <span className="cocat-update-fill" />
      <span className="cocat-update-tick" />
    </div>
  );
}
