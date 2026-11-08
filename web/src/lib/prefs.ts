/** 存在本机的偏好，不上传服务端。 */

export interface Prefs {
  /** 关掉弹入、粒子这类装饰动画 */
  reduceMotion: boolean;
  /** 发送后不清空参考图，方便连着改几轮 */
  keepInput: boolean;
}

const KEY = "tududu.prefs";

const DEFAULTS: Prefs = {
  reduceMotion: false,
  keepInput: false
};

export function readPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return { ...DEFAULTS };

    const parsed = JSON.parse(raw) as Partial<Prefs>;
    return { ...DEFAULTS, ...parsed };
  } catch {
    return { ...DEFAULTS };
  }
}

export function writePrefs(next: Prefs): void {
  localStorage.setItem(KEY, JSON.stringify(next));
}
