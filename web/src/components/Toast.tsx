import { useEffect } from "react";

/** 一条会自己烧完的提示。参考 ReactBits 的 Swipe Toast。 */
export default function Toast({
  text,
  onDone,
  duration = 2600
}: {
  text: string;
  onDone: () => void;
  duration?: number;
}) {
  useEffect(() => {
    const timer = window.setTimeout(onDone, duration);
    return () => window.clearTimeout(timer);
  }, [onDone, duration]);

  return (
    <div className="toast" role="status" onClick={onDone}>
      {text}
      <span className="fuse" style={{ animationDuration: duration + "ms" }} aria-hidden="true" />
    </div>
  );
}
