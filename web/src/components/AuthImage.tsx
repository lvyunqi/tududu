import { useEffect, useState } from "react";

import { BASE, getToken } from "../lib/api";

/**
 * 图片接口要带令牌，直接把地址丢给 img 标签拿不到，这里先取成 blob 再显示。
 *
 * variant：
 * - thumb   列表用，512 长边
 * - preview 点开查看用，1280 长边
 * - full    原图，只有下载才用
 */
export default function AuthImage({
  assetId,
  alt,
  variant = "preview"
}: {
  assetId: string;
  alt: string;
  variant?: "full" | "thumb" | "preview";
}) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let revoked = false;
    let objectUrl: string | null = null;

    async function load() {
      const token = getToken();
      const suffix = variant === "full" ? "" : "?size=" + variant;

      const response = await fetch(BASE + "/api/v1/assets/" + assetId + "/file" + suffix, {
        headers: token ? { Authorization: "Bearer " + token } : {}
      });

      if (!response.ok) return;

      const blob = await response.blob();
      objectUrl = URL.createObjectURL(blob);
      if (!revoked) setUrl(objectUrl);
    }

    void load();

    return () => {
      revoked = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [assetId, variant]);

  if (!url) return <div className="thumb placeholder" />;

  return <img className="thumb" alt={alt} src={url} loading="lazy" decoding="async" />;
}
