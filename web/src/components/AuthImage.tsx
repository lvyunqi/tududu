import { useEffect, useState } from "react";

import { getToken } from "../lib/api";

/** 图片接口要带令牌，直接把地址丢给 img 标签拿不到，这里先取成 blob 再显示。 */
export default function AuthImage({
  assetId,
  alt
}: {
  assetId: string;
  alt: string;
}) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    let revoked = false;
    let objectUrl: string | null = null;

    async function load() {
      const token = getToken();
      const response = await fetch(`/api/v1/assets/${assetId}/file`, {
        headers: token ? { Authorization: `Bearer ${token}` } : {}
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
  }, [assetId]);

  if (!url) return <div className="thumb placeholder" />;
  return <img className="thumb" alt={alt} src={url} />;
}
