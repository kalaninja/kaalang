#!/usr/bin/env bash
# Cargo's runner for Linux targets. A performance test runs on the efficiency
# cores of a hybrid processor, where its budgets were recorded.
set -euo pipefail

if [[ "${1##*/}" == performance-* && -r /sys/devices/cpu_atom/cpus ]]; then
  cores=$(< /sys/devices/cpu_atom/cpus)
  # A cpuset can exclude the cores the processor lists.
  if taskset -c "$cores" true 2> /dev/null; then
    exec taskset -c "$cores" "$@"
  fi
fi
exec "$@"
