#!/usr/bin/env bash
# Cargo's runner for Linux targets. A performance test runs on the efficiency
# cores of a hybrid processor, where its budgets were recorded.
set -euo pipefail

if [[ "${1##*/}" == performance-* && -r /sys/devices/cpu_atom/cpus ]]; then
  exec taskset -c "$(< /sys/devices/cpu_atom/cpus)" "$@"
fi
exec "$@"
