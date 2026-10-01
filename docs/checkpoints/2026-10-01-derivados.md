# Checkpoint: conferência e reconstrução dos dados derivados (01/10/2026)

Dívida técnica da auditoria (HANDOFF §25, item F2), na construção contínua. Sem migração.

## IMPLEMENTADO

- **Conferência** (`crates/sied-armazenamento/src/derivados.rs`, `conferir`; comando
  `conferir_derivados`). Para cada tabela derivada (conversa, conversa_participante,
  busca_artefato, busca_arquivo) calcula o que os registros de origem produzem, com a mesma regra
  dos gatilhos da migração 3, e compara com o que está gravado (`EXCEPT` nos dois sentidos):
  linhas faltando e linhas a mais ou alteradas. Nos índices FTS também roda o `integrity-check`
  (índice danificado com o texto intacto). Só lê.
- **Reconstrução** (`reconstruir`; comando `reconstruir_derivados`), numa transação:
  - conversas corrigidas pela chave (extração, app, id no app): as que continuam existindo
    mantêm o `id`; as que não têm mais mensagens saem;
  - participantes refeitos;
  - índices FTS refeitos com `rebuild` (conserta índice danificado) e repreenchidos a partir da
    origem;
  - confere de novo e só confirma sem diferença; registro `derivados_reconstruidos` na
    auditoria com o que estava divergente antes e as linhas depois.
  - Recusada enquanto houver importação em andamento.
- **Tela**: Dashboard → Auditoria → "Conversas e índices de busca", com **Conferir**; com
  diferença, a lista por tabela e **Reconstruir**. Depois da reconstrução todas as consultas da
  tela são refeitas e a atividade "Conversas e índices reconstruídos" entra na lista.
- **F6 (hash em paralelo) medido e não adotado**: SHA-256 numa thread com instruções SHA da CPU
  passa de 1 GB/s, acima da leitura de discos de evidência típicos; registrado no HANDOFF §25.

## TELAS (app real, caso SINTÉTICO)

- `docs/checkpoints/derivados/d2-derivados-divergem.png`: diferenças depois de alterar as tabelas
  derivadas por fora do SIED (conversa com contador errado, participante e 3 linhas do índice
  apagados).
- `docs/checkpoints/derivados/d3-derivados-reconstruidos.png`: depois de Reconstruir, com a
  atividade na auditoria.

## TESTES

- 282 Rust (+3 medições ignoradas), 59 vitest, roteiro E2E no app real: todos passam.
- `sied-armazenamento/tests/derivados.rs`:
  - caso recém-importado confere;
  - conversa alterada, conversa inventada com participante, participante apagado, linhas do
    índice apagadas e linha a mais no índice de arquivos: cada diferença é contada; a
    reconstrução corrige, mantém os ids das conversas, a busca volta a dar os mesmos achados,
    as tabelas de origem (hash de cada registro e arquivo) não mudam e a auditoria fica íntegra;
  - segmentos do índice FTS apagados com o texto intacto: só o `integrity-check` acusa, e a
    reconstrução refaz.
- Medição (`desempenho.rs`, 200 mil registros, release): conferir ~3 s, reconstruir ~10 s.
- Vitest (`SecoesCaso.test.tsx`): mensagem quando confere, lista das diferenças e reconstrução.

## ARQUIVOS

- `crates/sied-armazenamento/src/derivados.rs` (novo), `lib.rs`, `caso.rs`
- `crates/sied-armazenamento/tests/derivados.rs` (novo), `tests/desempenho.rs`
- `app/src-tauri/src/servico.rs`, `comandos.rs`, `lib.rs`
- `app/src/api/cliente.ts`, `consultas.ts`, `tipos/{ConferenciaDerivados,DiferencaDerivado,ReconstrucaoDerivados}.ts`
- `app/src/componentes/SecoesCaso.tsx`, `SecoesCaso.test.tsx` (novo)

## DECISÕES

- Correção no lugar em vez de apagar e recriar as conversas: links e telas abertas que guardam o
  `id` da conversa continuam válidos.
- A regra dos derivados fica escrita duas vezes (gatilhos da migração 3 e consultas de
  `derivados.rs`). Uma migração nova que mude um gatilho precisa mudar as consultas também; a
  conferência acusa a diferença num caso recém-importado (teste `caso_recem_importado_confere`).
- A conferência não roda sozinha ao abrir o caso (3 s em 200 mil registros); fica no botão.

## PROBLEMAS

Nenhum em aberto.

## COMO EXECUTAR / COMO TESTAR

```text
cargo test --workspace
cd app && npm test
testes/e2e/executar.sh target/debug/sied-desktop
SIED_VOLUME=200000 cargo test --release -p sied-armazenamento --test desempenho -- --ignored --nocapture
```

## LIMITAÇÕES

- Miniaturas (também derivadas, fora do banco) não entram: são refeitas sob demanda e conferidas
  pelo SHA-256 do inventário antes de decodificar.

## PRÓXIMA FASE

Avilla segue aguardando a árvore real. Da dívida técnica restam F3 (assinatura, depende de
decisões do usuário) e F5 (unidade mapeada).
