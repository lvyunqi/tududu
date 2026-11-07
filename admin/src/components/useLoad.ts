import { useCallback, useEffect, useState } from "react";

export function useLoad<T>(loader: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const value = await loader();
      setData(value);
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载失败");
    } finally {
      setLoading(false);
    }
    // loader 每次渲染都是新函数，这里只按 deps 触发
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  useEffect(() => {
    void reload();
  }, [reload]);

  // 已经有数据时不再把页面切成「加载中」，刷新时保持旧内容，视觉上更稳
  return { data, error, loading: loading && data === null, reload };
}
