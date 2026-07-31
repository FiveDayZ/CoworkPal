import type React from "react";

const coolingParticles = Array.from({ length: 12 }, (_, index) => ({
  delay: index * 72,
  drift: index % 2 === 0 ? -5 - index * 0.2 : 5 + index * 0.2,
  left: 54 + ((index * 11) % 58),
  top: 104 + ((index * 7) % 24),
}));

export function CoolingParticles({ count }: { count: number }) {
  return (
    <div className="cocat-cooling-particles" aria-hidden="true">
      {coolingParticles.slice(0, count).map((particle, index) => (
        <span
          className="cocat-cooling-pixel"
          key={index}
          style={
            {
              "--cooling-pixel-delay": `${particle.delay}ms`,
              "--cooling-pixel-drift": `${particle.drift}px`,
              "--cooling-pixel-left": `${particle.left}px`,
              "--cooling-pixel-top": `${particle.top}px`,
            } as React.CSSProperties
          }
        />
      ))}
    </div>
  );
}
