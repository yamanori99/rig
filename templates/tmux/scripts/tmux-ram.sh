#!/bin/bash
# RAM 使用率を出力

ram_used=$(memory_pressure 2>/dev/null | grep "System-wide memory free percentage" | awk '{print 100-$5}')
if [ -z "$ram_used" ]; then
    ram_used="N/A"
fi

echo "${ram_used}%"

