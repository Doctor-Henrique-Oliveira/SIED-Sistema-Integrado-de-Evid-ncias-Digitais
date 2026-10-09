# Checkpoint E11 — catálogo local de provedores

## Entrega

O primeiro bloco executável de E11 foi integrado ao banco global da estação:

- migração `global/0002_provedores_estacao.sql`;
- manifesto persistido com id, nome, tipo, modo (`local`/`externo`), caminho,
  versão, observação e estado habilitado;
- comandos `listar_provedores_estacao` e `salvar_provedor_estacao`;
- tela Configurações para cadastrar e consultar provedores locais;
- bloqueio no backend para habilitar provedores externos;
- nenhum segredo, token ou conteúdo de caso é persistido;
- testes direcionados para a regra de segurança e persistência (aguardando execução Rust no toolchain Windows).

O catálogo é somente configuração. Ele não testa executáveis, não inicia motores,
não abre rede e não altera extrações. A integração de execução opt-in, providers de
IMEI, usuários/perfis/permissões, backup/restauração e auditoria de ações permanece
para os próximos incrementos de E11.

## Validação

- `npm.cmd run typecheck`: passou.
- `npm.cmd test -- --run`: 29 arquivos / 214 testes passaram.
- `npm.cmd run build`: passou; Vite emitiu apenas o aviso existente de chunk acima
  de 500 kB.
- `cargo fmt --all`: passou.
- `cargo metadata --no-deps --format-version 1`: passou.
- `cargo check -p sied-armazenamento` e testes Rust: bloqueados antes da compilação
  por `link.exe` ausente no ambiente Windows (MSVC/Windows SDK indisponíveis).

O adaptador TypeScript da tela fica em `app/src/api/provedores.ts` até a CI/toolchain
Rust conseguir regenerar os contratos ts-rs genuínos; não há contrato manual dentro
de `app/src/api/tipos`.
