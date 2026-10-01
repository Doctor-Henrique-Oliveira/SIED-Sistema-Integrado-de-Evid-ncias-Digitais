# Checkpoint: conversas grandes servidas pelo índice (01/10/2026)

Fase de dívida técnica **medida** (HANDOFF §25, item F1), na construção contínua.

## IMPLEMENTADO

- **Migração 6 (`ordem_conversa`)**: índice `idx_artefato_conversa_ordem (extracao_id, app,
  conversa_id_externo, data_hora_ms)` no lugar de `idx_artefato_conversa (app,
  conversa_id_externo, data_hora_ms)`. Só índice: nenhum dado muda, nada é reconstruído, e a
  cópia `.antes-v6.bak` é feita como em toda migração.
- **Ordem da conversa lida do índice, sem ordenar** (`crates/sied-armazenamento/src/conversas.rs`).
  A ordem continua a mesma (datadas por data e id; sem data no fim, por id), mas é lida em duas
  partes que o índice entrega prontas. Antes, o `IS NULL` na frente do `ORDER BY` obrigava a
  ordenar a conversa inteira a cada página.
  - Páginas (`mensagens`): contagem das duas partes e `LIMIT/OFFSET` em cada uma.
  - Posição de uma mensagem: soma de contagens por intervalo do índice, sem `OR`.
  - Localizar: uma passada na ordem do índice, sem função de janela nem cópia do texto.
  - Trecho (evidência e conferência no relatório): posição pelas contagens e leitura do intervalo,
    no lugar de três funções de janela sobre a conversa inteira.

## MEDIÇÃO (`tests/desempenho.rs`, 200 mil SMS sintéticos, conversa de 100 mil, release)

| Operação | Antes | Depois |
|---|---:|---:|
| Página da conversa grande (início) | 58 ms | 7 ms |
| Página da conversa grande (fim) | 148 ms | 12–14 ms |
| Posição de uma mensagem | 34 ms | 6 ms |
| Localizar na conversa grande | 530 ms | 139 ms |
| Marcar trecho de 2.000 mensagens | 505 ms | 11–14 ms |
| Conversa curta (20 mensagens) | 0,3 ms | 0,1–0,2 ms |

As demais operações não mudaram (variação de ruído).

## TESTES

- 267 Rust (+3 medições ignoradas), 56 vitest. Todos passam (fmt e clippy limpos).
- Novo `volume.rs::paginas_posicoes_e_trecho_batem_com_a_ordem_calculada_a_parte`: a fixture
  passou a ter datas empatadas (de três em três) e um marcador sem data; o teste compara a ordem
  com uma calculada à parte, páginas de 7 cruzando a divisa datadas/sem data, a posição de todas
  as 1.200 mensagens, o marcador sem data no localizar e um trecho cruzando a divisa, informado de
  trás para frente.
- Novo unitário `conversas::testes::paginas_da_conversa_saem_do_indice_sem_ordenar`: o plano da
  consulta usa o índice novo e não tem `TEMP B-TREE`.
- Conferido que os testes pegam erro: ordem invertida das sem data e desempate errado na posição
  fazem o teste de volume falhar; o `ORDER BY` antigo faz o teste do plano falhar.
- Casos antigos: os testes que voltam o banco para v1–v4 desfazem também a migração 6.

## ARQUIVOS

- `crates/sied-armazenamento/migracoes/caso/0006_ordem_conversa.sql` (novo)
- `crates/sied-armazenamento/src/migracoes.rs`, `src/conversas.rs`
- `crates/sied-armazenamento/tests/volume.rs`, `importacao.rs`, `whatsapp.rs`, `persistencia.rs`
- `docs/CONTINUIDADE.md`, `docs/HANDOFF-CODEX.md`

## DECISÕES

- **F4 (`PRAGMA optimize` após importar) não entra**: medido com a mesma bateria, nenhum ganho
  (as consultas críticas já fixam o índice com `INDEXED BY`), e as estatísticas mudariam as
  escolhas do planejador nas demais consultas sem benefício demonstrado.
- Busca por telefone (~0,26 s) fica como está: acelerar exige uma tabela derivada de dígitos
  com gatilhos e preenchimento, mudança maior que o ganho.
- A conferência de todos os registros do caso (5,2 s) só é usada nos testes; o relatório
  confere apenas os registros das evidências. Sem mudança.

## PROBLEMAS

Nenhum em aberto.

## COMO EXECUTAR / COMO TESTAR

```text
cargo test --workspace
SIED_VOLUME=200000 cargo test --release -p sied-armazenamento --test desempenho -- --ignored --nocapture
```

Um caso existente migra para a v6 ao abrir (cópia `caso.sied.db.antes-v6.bak` antes).

## LIMITAÇÕES

- Localizar ainda passa por todas as mensagens da conversa (busca por trecho de texto, não por
  palavra); 0,14 s com 100 mil.
- Uma versão anterior do SIED recusa abrir um caso já migrado para a v6 (regra geral de
  versão mais nova).

## PRÓXIMA FASE

Seguir a dívida técnica: F9 (aviso de PDF órfão na pasta de relatórios) e F8 (destaque da
busca por posições em vez de marcadores no texto).
