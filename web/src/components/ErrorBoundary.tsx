import { Component, type ReactNode } from "react";

/**
 * 兜底：渲染出错时给一句人话，而不是整页白屏。
 *
 * 手机上打不开控制台，白屏就等于什么都查不到。
 */
export default class ErrorBoundary extends Component<
  { children: ReactNode },
  { message: string | null }
> {
  state = { message: null as string | null };

  static getDerivedStateFromError(error: unknown) {
    return { message: error instanceof Error ? error.message : String(error) };
  }

  render() {
    if (!this.state.message) return this.props.children;

    return (
      <div className="crash">
        <h1>页面出了点问题</h1>
        <p className="hint">{this.state.message}</p>

        <div className="crash-actions">
          <button onClick={() => window.location.reload()}>重新加载</button>
          <button
            className="ghost"
            onClick={() => {
              localStorage.clear();
              window.location.hash = "#/";
              window.location.reload();
            }}
          >
            清空本地数据后再试
          </button>
        </div>
      </div>
    );
  }
}
