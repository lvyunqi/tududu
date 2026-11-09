import type { ReactNode } from "react";

/**
 * 极简 Markdown 渲染。
 *
 * 更新说明是给用户看的，只用到标题、列表、加粗、行内代码和链接这几种，
 * 引一个完整解析器不值当，这里按行扫一遍就够。
 */
export function renderMarkdown(source: string): ReactNode[] {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const blocks: ReactNode[] = [];

  let list: ReactNode[] = [];
  let code: string[] | null = null;
  let key = 0;

  function flushList() {
    if (list.length === 0) return;
    blocks.push(<ul key={"ul" + key++}>{list}</ul>);
    list = [];
  }

  for (const raw of lines) {
    const line = raw.trimEnd();

    if (line.trimStart().startsWith("```")) {
      if (code) {
        blocks.push(
          <pre key={"code" + key++}>
            <code>{code.join("\n")}</code>
          </pre>
        );
        code = null;
      } else {
        flushList();
        code = [];
      }
      continue;
    }

    if (code) {
      code.push(raw);
      continue;
    }

    if (!line.trim()) {
      flushList();
      continue;
    }

    const heading = line.match(/^(#{1,3})\s+(.*)$/);
    if (heading) {
      flushList();
      const level = heading[1].length;
      blocks.push(
        <p key={"h" + key++} className={"md-h md-h" + level}>
          {inline(heading[2])}
        </p>
      );
      continue;
    }

    const bullet = line.match(/^\s*[-*]\s+(.*)$/);
    if (bullet) {
      list.push(<li key={"li" + key++}>{inline(bullet[1])}</li>);
      continue;
    }

    const numbered = line.match(/^\s*\d+\.\s+(.*)$/);
    if (numbered) {
      list.push(<li key={"li" + key++}>{inline(numbered[1])}</li>);
      continue;
    }

    flushList();
    blocks.push(<p key={"p" + key++}>{inline(line)}</p>);
  }

  flushList();

  if (code) {
    blocks.push(
      <pre key={"code" + key++}>
        <code>{code.join("\n")}</code>
      </pre>
    );
  }

  return blocks;
}

/// 行内：加粗、行内代码、链接
function inline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\))/g;

  let last = 0;
  let key = 0;

  for (const match of text.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > last) nodes.push(text.slice(last, index));

    const token = match[0];
    if (token.startsWith("**")) {
      nodes.push(<strong key={"b" + key++}>{token.slice(2, -2)}</strong>);
    } else if (token.startsWith("`")) {
      nodes.push(<code key={"c" + key++}>{token.slice(1, -1)}</code>);
    } else {
      const link = token.match(/\[([^\]]+)\]\(([^)]+)\)/);
      if (link) {
        nodes.push(
          <a key={"a" + key++} href={link[2]} target="_blank" rel="noreferrer">
            {link[1]}
          </a>
        );
      } else {
        nodes.push(token);
      }
    }

    last = index + token.length;
  }

  if (last < text.length) nodes.push(text.slice(last));

  return nodes;
}
