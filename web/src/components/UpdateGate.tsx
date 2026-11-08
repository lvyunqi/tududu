import { useEffect, useState } from "react";

import UpdateDialog from "./UpdateDialog";
import { checkUpdate, tauri, type VersionInfo } from "../lib/updater";

/** 启动时查一次新版本，发现了就弹窗。浏览器里不做任何事。 */
export default function UpdateGate() {
  const [info, setInfo] = useState<VersionInfo | null>(null);

  useEffect(() => {
    if (!tauri()) return;

    let cancelled = false;
    // 稍等一会儿再查，别和首屏抢资源
    const timer = window.setTimeout(() => {
      void checkUpdate("android").then((found) => {
        if (!cancelled && found) setInfo(found);
      });
    }, 2500);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, []);

  if (!info) return null;

  return <UpdateDialog info={info} onClose={() => setInfo(null)} />;
}
