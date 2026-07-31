import type React from "react";

const sparks = [
  { delay: 0, dx: 14, dy: -14 },
  { delay: 18, dx: 20, dy: -5 },
  { delay: 36, dx: 10, dy: 8 },
  { delay: 54, dx: 24, dy: 9 },
];

export function RepairSparks({ count }: { count: number }) {
  return (
    <div className="cocat-repair-sparks" aria-hidden="true">
      {sparks.slice(0, count).map((spark, index) => (
        <span
          className="cocat-repair-spark"
          key={index}
          style={
            {
              "--spark-delay": `${spark.delay}ms`,
              "--spark-dx": `${spark.dx}px`,
              "--spark-dy": `${spark.dy}px`,
            } as React.CSSProperties
          }
        />
      ))}
    </div>
  );
}
