# Operação de release e recuperação — E12

Este procedimento é local/offline por padrão e não substitui a validação do workflow
Windows. Ele serve para reproduzir o estado de uma versão sem alterar evidência.

## Antes do release

1. Confirmar `git status` e preservar qualquer `.claude/` local fora do commit.
2. Executar `npm.cmd run typecheck`, `npm.cmd test -- --run` e `npm.cmd run build` em
   `app/`.
3. Executar `cargo fmt --all -- --check` e `cargo metadata --no-deps --format-version 1`.
4. Em um runner Windows com MSVC/SDK, executar `cargo test --workspace`, `cargo clippy
   --workspace --all-targets -- -D warnings` e o workflow de instaladores.
5. Conferir que o commit e o artefato do instalador correspondem ao mesmo SHA; não
   instalar um artefato de outro commit apenas porque tem o mesmo nome.

## Limites e retomada

Os limites da ingestão podem ser reduzidos no ambiente do processo:

- `SIED_IMPORT_MAX_FILES` — quantidade máxima de arquivos;
- `SIED_IMPORT_MAX_BYTES` — bytes máximos declarados/lidos;
- `SIED_IMPORT_HASH_THREADS` — workers de hash, limitado internamente a 8.

Cada importação grava `importacoes/<id>/checkpoint-retomada.json`. O arquivo é derivado,
contém somente estado/contadores/fonte e não é evidência. `iniciada`, `inventariando`,
`inventario_concluido`, `cancelada` e `interrompida` são estados distintos. Somente o
estado final persistido no banco pode encerrar uma importação; um checkpoint parcial nunca
deve ser editado para forçar sucesso.

## Recuperação de falha

1. Preservar o caso e sua auditoria; não apagar `importacoes/` nem `cache/bancos/` para
   “limpar” uma execução.
2. Abrir o checkpoint e o log da importação para identificar o último estado e a origem.
3. Conferir os hashes do manifesto e a integridade do banco antes de qualquer nova
   tentativa.
4. Iniciar uma nova importação somente após decisão explícita do analista. A retomada
   automática da mesma transação permanece desabilitada até haver reabertura transacional
   que confira registros já gravados e impeça duplicação.
5. Registrar a nova execução separadamente na auditoria.

## Artefato de distribuição

O instalador Windows, quando produzido pela CI, deve ser baixado pelo SHA do commit e
verificado pelo hash publicado no próprio workflow. Build de interface local é útil para
validar UX, mas não comprova o binário Tauri/Rust nem substitui o instalador assinado.
