import type React from "react";

const steamPuffs = [
  { delay: 0, x: -6 },
  { delay: 90, x: 4 },
  { delay: 180, x: 10 },
];

export function MemorySteam({ count }: { count: number }) {
  return (
    <div className="cocat-memory-steam" aria-hidden="true">
      {steamPuffs.slice(0, count).map((puff, index) => (
        <span
          className="cocat-memory-steam-puff"
          key={index}
          style={
            {
              "--steam-delay": `${puff.delay}ms`,
              "--steam-x": `${puff.x}px`,
            } as React.CSSProperties
          }
        />
      ))}
    </div>
  );
}
