import type React from "react";

const burstPixels = Array.from({ length: 16 }, (_, index) => {
  const angle = (Math.PI * 2 * index) / 16;
  return {
    dx: Math.cos(angle) * (22 + (index % 4) * 4),
    dy: Math.sin(angle) * (12 + (index % 3) * 3),
    delay: (index % 4) * 28,
  };
});

export function CelebrateBurst({ count }: { count: number }) {
  return (
    <div className="cocat-celebrate-burst" aria-hidden="true">
      <span className="cocat-celebrate-steam-ring" />
      {burstPixels.slice(0, count).map((pixel, index) => (
        <span
          className="cocat-celebrate-pixel"
          key={index}
          style={
            {
              "--celebrate-delay": `${pixel.delay}ms`,
              "--celebrate-dx": `${pixel.dx.toFixed(1)}px`,
              "--celebrate-dy": `${pixel.dy.toFixed(1)}px`,
            } as React.CSSProperties
          }
        />
      ))}
    </div>
  );
}
