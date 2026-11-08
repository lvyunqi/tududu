#!/usr/bin/env bash
# 图嘟嘟服务端启动器
#
# 托管主程序：负责环境、参数、信号、更新后的重启。
#
# 用法：
#   tududu-launcher.sh [--env-file PATH] [--] [主程序参数...]
#
# 环境变量：
#   TUDUDU_BIN       主程序路径，默认 /opt/tududu/tududu
#   TUDUDU_WORKDIR   主程序工作目录，默认当前目录
#
# 要点：
#   1. 环境文件（默认工作目录下的 .env）只用来"补缺"：父进程已有的同名变量优先。
#   2. 主程序参数放在 -- 之后，例如 -- --log-filter debug。
#   3. 标准输出与错误输出直接沿用启动器的，交给服务管理器收日志。
#   4. 主程序以 75 退出表示"新版本已装好"，启动器重新拉起；其它退出码原样退出。
#   5. 收到 TERM/INT 先转发给主程序，等它自己收尾。

set -euo pipefail

BIN="${TUDUDU_BIN:-/opt/tududu/tududu}"
WORKDIR="${TUDUDU_WORKDIR:-}"
ENV_FILE=""
PROG_ARGS=()
CHILD=""

usage() {
  sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --env-file)
      [ "$#" -ge 2 ] || { echo "缺少 --env-file 的参数"; exit 2; }
      ENV_FILE="$2"; shift 2 ;;
    --env-file=*)
      ENV_FILE="${1#*=}"; shift ;;
    --)
      shift; PROG_ARGS=("$@"); break ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      echo "未知参数：$1"; echo "主程序参数请放在 -- 之后"; exit 2 ;;
  esac
done

if [ -n "$WORKDIR" ]; then
  cd "$WORKDIR" || { echo "进不去工作目录：$WORKDIR"; exit 1; }
fi

[ -x "$BIN" ] || { echo "找不到可执行文件：$BIN"; exit 1; }

if [ -z "$ENV_FILE" ] && [ -f ".env" ]; then
  ENV_FILE=".env"
fi

if [ -n "$ENV_FILE" ]; then
  [ -r "$ENV_FILE" ] || { echo "环境文件不可读：$ENV_FILE"; exit 1; }

  lineno=0
  while IFS= read -r raw || [ -n "$raw" ]; do
    lineno=$((lineno + 1))
    line=$(printf '%s' "$raw" | tr -d '\r')
    # 有些编辑器会加 BOM，先去掉再解析
    line=$(printf '%s' "$line" | sed -e '1s/^\xEF\xBB\xBF//')
    case "$line" in ''|'#'*) continue ;; esac
    # 格式不对就报错退出，只报行号，不回显内容（里面可能是密钥）
    case "$line" in
      *=*) ;;
      *) echo "环境文件第 $lineno 行缺少等号：$ENV_FILE"; exit 1 ;;
    esac
    key="${line%%=*}"
    case "$key" in
      ''|*[!A-Za-z0-9_]*) echo "环境文件第 $lineno 行的键名不合法：$ENV_FILE"; exit 1 ;;
    esac
    value="${line#*=}"
    # 父进程已经有这个变量就跳过，dotenv 的常规语义
    if [ -z "${!key+set}" ]; then
      export "$key=$value"
    fi
  done < "$ENV_FILE"

  echo "已加载环境文件：$ENV_FILE"
fi

forward_signal() {
  local signal="$1"
  if [ -n "$CHILD" ] && kill -0 "$CHILD" 2>/dev/null; then
    kill "-$signal" "$CHILD" 2>/dev/null || true
    wait "$CHILD" 2>/dev/null || true
  fi
  exit 0
}

trap 'forward_signal TERM' TERM
trap 'forward_signal INT' INT

echo "启动器托管：$BIN ${PROG_ARGS[*]:-（无参数）}"

while :; do
  set +e
  # 主程序不接面板的 stdin，避免误读；日志走 stdout/stderr
  "$BIN" "${PROG_ARGS[@]}" < /dev/null &
  CHILD=$!
  wait "$CHILD"
  code=$?
  set -e
  CHILD=""

  if [ "$code" -eq 75 ]; then
    echo "主程序报告新版本已安装，重新启动"
    continue
  fi

  if [ "$code" -ne 0 ]; then
    echo "主程序退出，代码 $code"
  fi

  exit "$code"
done
