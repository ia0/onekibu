#!/bin/sh
set -e

info() {
  echo "[1;36m$1[m"
}

info_exec() {
  info "$*"
  "$@"
}

for board in nrf52840-dk nrf52840-dongle nrf52840-mdk-dongle solo; do
  xtask_build="info_exec cargo xtask build --board=$board"
  $xtask_build
  $xtask_build --release
  $xtask_build --release --log=error
done
info_exec cargo xtask test
info_exec cargo fmt -- --check
info_exec cargo xtask clippy

info "Done"
