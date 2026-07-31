import type React from "react";
import type { CoCatStarParticle } from "../../animation/animationTypes";

export function ClickStars({ stars }: { stars: CoCatStarParticle[] }) {
  return (
    <>
      {stars.map((star) => (
        <span
          className="cocat-click-star"
          key={star.id}
          style={
            {
              "--star-delay": `${star.delayMs}ms`,
              "--star-dx": `${star.dx}px`,
              "--star-dy": `${star.dy}px`,
            } as React.CSSProperties
          }
        />
      ))}
    </>
  );
}
