# Checkpoint — E11: controle de providers e estação

## Implementado

- Migração global 0003 com permissões por usuário/ação e trilha `auditoria_estacao` encadeada por SHA-256.
- Execução explícita de provider local habilitado, somente com `--version`, sem shell/rede, saída limitada a 2.000 caracteres e timeout de 2 segundos.
- Enriquecimento de IMEI local por JSON/CSV configurado, limitado a 4 MiB, sem consultas externas e sem alterar o dispositivo/evidência do caso.
- Backup do catálogo global por `VACUUM INTO` com SHA-256 e restauração por merge validado; casos e bancos de evidência nunca são apagados.
- Comandos Tauri, cliente TypeScript e controles na tela de Configurações para permissões, execução, IMEI, backup e verificação da trilha.

## Limites

Providers externos, rede, autenticação institucional e execução de pipelines de aquisição permanecem bloqueados.
Avilla continua reservado à E13. O backup desta entrega é do catálogo/configuração da estação; a pasta de cada
caso continua autocontida e deve ser copiada/restaurada como unidade, preservando seus hashes.

## Validação

`cargo fmt --all`, `cargo metadata`, typecheck e 29 arquivos/214 testes de frontend passaram. O build Rust não
foi concluído neste Windows porque `link.exe`/MSVC não estão disponíveis; nenhum Build Tools foi instalado.
