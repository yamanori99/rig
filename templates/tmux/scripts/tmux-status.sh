#!/bin/bash
# tmux ステータスバー用のシステム情報
# CPU load average + 使用率

# CPU load average (1分平均)
cpu_load=$(sysctl -n vm.loadavg | awk '{print $2}')

# CPU 使用率
cpu_usage=$(ps -A -o %cpu | awk '{s+=$1} END {print int(s)}')
cores=$(sysctl -n hw.ncpu)
cpu_percent=$((cpu_usage / cores))

echo "${cpu_load}|${cpu_percent}%"

