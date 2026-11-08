/**
 * 一组用基础图形拼出来的实心图标。
 *
 * 不依赖系统字体里的符号（⚙ ◎ ▤ ✦ 在不同系统上粗细不一），
 * 自己画能保证形状一致，也能给图形本身加渐变，做出厚一点的观感。
 */

type Props = { size?: number };

/** 每个图标共用一套明暗：上亮下暗，看起来是凸出来的。 */
function Shade({ id }: { id: string }) {
  return (
    <defs>
      <linearGradient id={id} x1="0.3" y1="0" x2="0.7" y2="1">
        <stop offset="0%" stopColor="#525a69" />
        <stop offset="55%" stopColor="#262b34" />
        <stop offset="100%" stopColor="#101318" />
      </linearGradient>
    </defs>
  );
}

export function GearIcon({ size = 17 }: Props) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
      <Shade id="gear-shade" />

      {[0, 45, 90, 135].map((angle) => (
        <rect
          key={angle}
          x="10.4"
          y="1.4"
          width="3.2"
          height="5"
          rx="1.1"
          fill="url(#gear-shade)"
          transform={"rotate(" + angle + " 12 12)"}
        />
      ))}

      <circle cx="12" cy="12" r="6.6" fill="url(#gear-shade)" />
      <circle cx="12" cy="12" r="2.5" fill="#f5f6fa" />
    </svg>
  );
}

export function WalletIcon({ size = 17 }: Props) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
      <Shade id="wallet-shade" />

      <rect x="2.6" y="5.6" width="18.8" height="13.4" rx="3.6" fill="url(#wallet-shade)" />
      <rect x="15.2" y="10.2" width="6.2" height="4.4" rx="2.2" fill="#f5f6fa" />
      <circle cx="17.6" cy="12.4" r="1.05" fill="url(#wallet-shade)" />
    </svg>
  );
}

export function GalleryIcon({ size = 17 }: Props) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
      <Shade id="gallery-shade" />

      <rect x="2.8" y="4.2" width="18.4" height="15.6" rx="3.4" fill="url(#gallery-shade)" />
      <circle cx="8.4" cy="9.6" r="1.9" fill="#f5f6fa" />
      <path d="M4.4 18.2l4.9-4.6 3.6 3.4 2.6-2.6 4.1 3.8z" fill="#f5f6fa" />
    </svg>
  );
}

/**
 * 广场入口：一颗小星球，卫星绕着飞。
 *
 * 纯 CSS 动画，卫星转到星球后面时会被挡住，看起来更像真的在绕行。
 */
export function PlanetIcon({ size = 24 }: { size?: number }) {
  return (
    <span className="planet" style={{ width: size, height: size }}>
      <span className="planet-orbit">
        <span className="planet-sat" />
      </span>

      <span className="planet-body" />
      <span className="planet-shine" />
      <span className="planet-shadow" />
    </span>
  );
}
