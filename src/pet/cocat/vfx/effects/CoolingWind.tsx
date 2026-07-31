import type React from "react";

const windBands = [
  { delay: 0, y: 0, scale: 1 },
  { delay: 120, y: 9, scale: 0.84 },
  { delay: 240, y: -8, scale: 0.72 },
];

export function CoolingWind() {
  return (
    <div className="cocat-cooling-wind" aria-hidden="true">
      {windBands.map((band, index) => (
        <span
          className="cocat-cooling-wind-band"
          key={index}
          style={
            {
              "--cooling-band-delay": `${band.delay}ms`,
              "--cooling-band-scale": band.scale,
              "--cooling-band-y": `${band.y}px`,
            } as React.CSSProperties
          }
        />
      ))}
    </div>
  );
}
