/** 各个二级页面左上角统一的返回按钮，样式只在这里定义。 */
export default function BackHome() {
  return (
    <a className="back-home" href="#/" aria-label="回到对话">
      <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
        <path
          d="M14.6 5.2L8 12l6.6 6.8"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </a>
  );
}
