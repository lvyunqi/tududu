#!/usr/bin/env bash
# 图嘟嘟服务端启动器
#
# 职责和 AnyFlows 的启动器一样：托管主程序、传递环境变量、
# 发现主程序以 75 退出（表示刚装好新版本）就重新拉起，遇到正常退出就跟着退出。
#
# 用法：
#   ./tududu-launcher.sh            启动（默认在 /opt/tududu/tududu）
#   TUDUDU_BIN=/path ./tududu-launcher.sh
#   TUDUDU_ARGS="serve" ./tududu-launcher.sh   主程序参数

set -euo pipefail

BIN="${TUDUDU_BIN:-/opt/tududu/tududu}"
ARGS="${TUDUDU_ARGS:-serve}"

if [ ! -x "$BIN" ]; then
  echo "找不到可执行文件：$BIN"
  exit 1
fi

echo "启动器托管：$BIN $ARGS"

while true; do
  set +e
  # shellcheck disable=SC2086
  "$BIN" $ARGS
  code=$?
  set -e

  if [ "$code" -eq 75 ]; then
    echo "检测到新版本已安装，重新启动"
    continue
  fi

  if [ "$code" -ne 0 ]; then
    echo "主程序异常退出：$code"
  fi

  exit "$code"
done
