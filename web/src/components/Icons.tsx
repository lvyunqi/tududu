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

/** 广场入口：一颗主火花加一颗小火花，像 AI 生成的那种记号。 */
export function StarIcon({ size = 22 }: Props) {
  const small = Math.round(size * 0.44);

  return (
    <span className="sparkle-mark" style={{ width: size, height: size }}>
      <svg viewBox="0 0 24 24" width={size} height={size} aria-hidden="true">
        <defs>
          <linearGradient id="sparkle-main" x1="0.3" y1="0" x2="0.75" y2="1">
            <stop offset="0%" stopColor="#5f6875" />
            <stop offset="46%" stopColor="#252a33" />
            <stop offset="100%" stopColor="#0b0e13" />
          </linearGradient>
        </defs>

        <path
          d="M12 1.6c.72 5.72 4.68 9.68 10.4 10.4-5.72.72-9.68 4.68-10.4 10.4C11.28 16.68 7.32 12.72 1.6 12 7.32 11.28 11.28 7.32 12 1.6Z"
          fill="url(#sparkle-main)"
        />

        {/* 内芯一抹亮，火花才有中间发亮的感觉 */}
        <path
          d="M12 5.2c.34 2.7 2.1 4.46 4.8 4.8-2.7.34-4.46 2.1-4.8 4.8-.34-2.7-2.1-4.46-4.8-4.8 2.7-.34 4.46-2.1 4.8-4.8Z"
          fill="#ffffff"
          opacity="0.16"
        />
      </svg>

      {/* 右上角的小火花，自己有个闪烁节奏 */}
      <svg
        className="sparkle-small"
        viewBox="0 0 24 24"
        width={small}
        height={small}
        aria-hidden="true"
      >
        <path
          d="M12 1.6c.72 5.72 4.68 9.68 10.4 10.4-5.72.72-9.68 4.68-10.4 10.4C11.28 16.68 7.32 12.72 1.6 12 7.32 11.28 11.28 7.32 12 1.6Z"
          fill="url(#sparkle-main)"
        />
      </svg>
    </span>
  );
}
