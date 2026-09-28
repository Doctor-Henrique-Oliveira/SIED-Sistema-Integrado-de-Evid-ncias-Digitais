#!/usr/bin/env bash
# Executa o teste ponta a ponta no Linux com display virtual e gerenciador de janelas.
# Uso: testes/e2e/executar.sh [binário]   (padrão: target/debug/sied-desktop)
set -euo pipefail
BIN="${1:-target/debug/sied-desktop}"
exec xvfb-run -a -s "-screen 0 1400x900x24" bash -c "
  if command -v openbox >/dev/null; then openbox >/dev/null 2>&1 & sleep 1; export SIED_E2E_WMCTRL=1; fi
  node testes/e2e/persistencia.mjs '$BIN'
"
