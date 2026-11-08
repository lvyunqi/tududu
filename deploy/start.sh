#!/usr/bin/env bash
# 发行包入口：解压后直接执行 ./start.sh
#
# 等价于用默认参数调用启动器；需要自定义就用 tududu-launcher.sh。

set -euo pipefail

cd "$(dirname "$0")"
exec ./tududu-launcher.sh "$@"
