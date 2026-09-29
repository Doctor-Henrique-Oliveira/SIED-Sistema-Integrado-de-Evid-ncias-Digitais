# Checkpoint — Auditoria pré-produção e hardening (29/09/2026)

Auditoria técnica final antes da entrega a outro agente. Método: **o código foi lido e
testado, não a documentação**. Cada problema corrigido foi reproduzido antes (teste que
falha sem a correção, conferido revertendo a mudança) e ficou com teste de regressão.

- Branch: `claude/auditoria-hardening-7p20bx` · PR **#11** · base `main` = `5b68c8d`
  (merge do PR #10).
- Nenhuma migração nova, nenhuma mudança incompatível de formato de banco, nenhuma
  funcionalidade removida, nenhum teste enfraquecido. Política de somente leitura intacta.
  Importador Avilla **não** iniciado.
- Commit final da `main` e resultado da CI: ver o fim deste documento e o relatório da
  thread (o hash do merge só existe depois do merge).

---

## 1. Resumo por severidade

| Severidade | Encontrados | Corrigidos | Pendentes |
|---|---|---|---|
| CRÍTICO | 1 | 1 | 0 |
| ALTO | 7 | 7 | 0 |
| MÉDIO | 11 | 11 | 0 |
| BAIXO | 9 | 5 | 4 (documentados) |
| FUTURO | 9 | — | 9 (roadmap) |

## 2. Achados e correções

### CRÍTICO

| # | Problema | Correção | Commit | Teste |
|---|---|---|---|---|
| C1 | O relatório dava como **conferida** uma evidência de registro ou trecho só relendo a coluna `hash_registro`, sem recalcular a partir do conteúdo. Um registro alterado no banco do caso (fora do SIED, sem os gatilhos) saía "conferido". | `NovoArtefato::hash_registro()` é a única regra do hash; `extracoes::conferir_registro` remonta o artefato a partir das colunas e origens gravadas e recalcula. O relatório usa o recalculado (registro e cada mensagem do trecho) e marca **divergente**, listando o que mudou. Para o recálculo ser exato: `serde_json` com `float_roundtrip` (sem ele, reais aleatórios não voltam iguais do JSON) e `-0.0` normalizado antes do hash (coluna REAL perde o sinal). | `97f59f2` | `tests/hash_registro.rs` (dados hostis conferem; 5 adulterações detectadas), `relatorio.rs::conteudo_do_registro_alterado_no_banco_do_caso_diverge`, `artefato.rs` (hash fixo conhecido, 200 mil reais ida e volta) |

### ALTO

| # | Problema | Correção | Commit |
|---|---|---|---|
| A1 | O mesmo caso abria em duas janelas (ou duas estações na mesma pasta): a segunda abertura marcava como "interrompida" a importação que a primeira fazia, e as duas gravavam no mesmo banco. | Trava exclusiva `caso.sied.lock` (`File::try_lock`), pega antes de tocar no banco e mantida enquanto o caso está aberto. Erro `CasoEmUso` → código `conflito`, mensagem em pt-BR. | `f24df36` |
| A2 | Cópia de segurança da migração com nome fixo `.antes-vN.bak`: se já existisse (migração anterior interrompida), `VACUUM INTO` falhava e **o caso não abria mais**. | Nome livre (`-2`, `-3`...). | `ac63ae0` |
| A3 | Lista de conversas com custo quadrático: `coalesce(a.app,'') = c.app` na prévia da última mensagem impedia o índice (3,0 s com 20 mil mensagens). Abrir uma conversa curta levava ~145 ms com 200 mil (o SQLite, sem estatísticas, escolhia o índice errado e percorria a extração). | `a.app IS nullif(c.app, '')` e `INDEXED BY idx_artefato_conversa` nas consultas da conversa; pipeline nunca grava `app` vazio (normalizado antes do hash). | `1a49ab0` |
| A4 | Uma data absurda numa mensagem (lixo ou valor hostil, ex.: 9e15 ms) virava `utc_ms`, e a interface lançava `RangeError` ao formatar: **a tela de conversas inteira caía**. | Parsers só aceitam datas entre 1970 e 31/12/9999 (`DataHora::ms_plausivel`; o valor fica em `original`); `dataHoraMs`, `dia` e `hora` mostram "data fora do intervalo" para dados já importados. | `2e48156` |
| A5 | Imagem enorme (ex.: 192 MP com orientação EXIF) era girada na resolução cheia (pico ~1,1 GB); várias ao mesmo tempo podiam esgotar a memória. Pânico num decodificador, no Typst ou num parser derrubava a importação inteira ou o app. | Reduz antes de girar (pico ~573 MB, 4,4 s → 1,4 s); no máximo uma decodificação grande por vez; `catch_unwind` em `parser.processar`, no pipeline da importação, na decodificação de imagem e no Typst (compilar e gerar PDF). | `ec0d609` |
| A6 | Apagar os registros finais da trilha de auditoria, ou reescrevê-la inteira fora do SIED, deixava uma cadeia que, recalculada sozinha, parecia íntegra. | A verificação confere a âncora (`auditoria_ate_seq` + `auditoria_hash`) gravada por cada relatório (também impressa no PDF). | `cd1af15` |
| A7 | Um parser em pânico ou dado hostil podia encerrar a importação sem registro de falha. | Execução registrada como falha com a mensagem; os demais parsers seguem; o que foi emitido antes é gravado. | `ec0d609` |

### MÉDIO

| # | Problema | Correção | Commit |
|---|---|---|---|
| M1 | Todos os comandos Tauri eram síncronos e rodavam na thread da janela: durante o relatório (segundos) a janela congelava ("Não respondendo"). | Comandos `async` com `spawn_blocking`. | `506af8a` |
| M2 | Abrir um arquivo que não é banco do SIED ativava WAL (escrevendo no arquivo alheio) antes de conferir o `application_id`; arquivo vazio era inicializado como banco novo; erros do SQLite chegavam crus em inglês. | Tipo conferido antes de qualquer escrita; vazio só em `Criar`; corrupção vira `Inconsistente`; erros SQLite traduzidos (disco cheio, somente leitura, corrompido, ocupado) com código estável. | `ac63ae0` |
| M3 | Filtros de conversa, localizar na conversa, arquivos e identificadores usavam `lower()` do SQLite (só ASCII): "SINTÉTICA" não achava "Sintética". | Função SQL `sied_normal()` (NFD, sem acentos, minúsculas Unicode), registrada em toda conexão, usada só em consultas (nunca no esquema). | `dac7b39` |
| M4 | `CaminhoFonte` só recusava `C:` no início; `a/C:/x` passava e, no Windows, `PathBuf::push("C:")` troca o caminho inteiro (sai da raiz da extração). Também aceitava `.` e `...` no meio. | Cada componente conferido; a leitura da pasta confere de novo cada parte com as regras da plataforma (`Component::Normal`). Nome assim na listagem vira item ignorado com aviso. | `dab6953` |
| M5 | `-wal`/`-journal` listado mas ilegível era pulado em silêncio: o banco era interpretado sem ele. | Aviso gravado e execução parcial. | `06d960e` |
| M6 | Trecho marcado enquanto a extração ainda importava podia ganhar mensagens e divergir no relatório sem adulteração. | Recusado enquanto a extração está `em_andamento`. | `5636248` |
| M7 | Pasta padrão dos casos (`Documentos`) costuma estar no OneDrive no Windows 11: dados de investigação sairiam da estação e a sincronização pode danificar o SQLite aberto. Caso em compartilhamento de rede: WAL não é confiável fora do disco local. | Aviso na tela de novo caso para OneDrive, Dropbox, Google Drive, iCloud e caminhos UNC. O analista decide. | `985e20c`, `6ca06ef` |
| M8 | WhatsApp interpretava coordenadas fora de ±90/±180 (ou incompletas); REAL infinito virava `null` nos valores brutos (perda de informação). | Coordenada inválida não interpretada (`coordenadas_invalidas`); infinito gravado como `{"real_nao_finito": "inf"}`. | `72a4672` |
| M9 | Emojis, caracteres de controle e invisíveis (RLO, ZWJ) sem glifo nas fontes embutidas saíam como caixas vazias ou sumiam no PDF. | Trocados por `[U+XXXX ...]` legíveis; metodologia do relatório explica a notação. | `ec0d609` |
| M10 | Texto do relatório afirmava que a geração seria "o registro nº N+1" da trilha; uma importação terminando durante a geração tornava isso falso. | "registro posterior ao nº N". (Classificado médio por ser afirmação num documento pericial.) | `5e9e196` |
| M11 | Evidência de arquivo e miniatura: nome temporário do cache podia colidir entre processos. | Nome com pid e sequência. | `ec0d609` |

### BAIXO

| # | Problema | Situação |
|---|---|---|
| B1 | `.parcial` ficava na pasta de relatórios após falha de gravação. | Corrigido (`37afac8`) |
| B2 | Procedimento chamado `CON`, `NUL`, `COM1`... gerava pasta impossível no Windows. | Corrigido (`72685f1`) |
| B3 | `.gitignore` não cobria `*.bak`, `*.parcial`, trava, log e manifesto do caso. | Corrigido (`eb89f48`) |
| B4 | Importação com nomes perigosos (`C:`, `...`) interrompia tudo em vez de ignorar o item. | Corrigido (`dab6953`) |
| B5 | `app` vazio e ausente eram tratados de forma diferente em consultas. | Corrigido (`1a49ab0`) |
| B6 | Marcadores `\u0001`/`\u0002` do trecho da busca podem vir do próprio conteúdo e destacar o texto errado (só visual; o React escapa o texto). | Pendente, documentado |
| B7 | Se o SIED cair entre renomear o PDF e registrá-lo, fica um PDF com `.sha256` sem linha na tabela `relatorio` (nunca o contrário). | Pendente, documentado |
| B8 | Página da conversa de 100 mil mensagens 79–185 ms; localizar 0,5 s; busca por telefone 0,3 s com 200 mil. | Pendente (ver F1), documentado |
| B9 | Trava do caso impede copiar a pasta **com o caso aberto** no Windows (arquivo travado); copiar com o caso aberto também perderia o `-wal`. | Documentado: copiar só com o caso fechado |

### Problema encontrado na CI durante a auditoria

Os testes Linux da CI falharam duas vezes **na ligação** dos binários de teste (`rust-lld`
terminou com sinal 7, "Bus error"), não num teste: o disco do executor acabava. Cada binário de
teste que liga o Typst tinha ~530 MB com depuração, e a auditoria acrescentou um. Correção:
dependências sem informação de depuração nos perfis de desenvolvimento e teste
(`Cargo.toml`, `[profile.dev.package."*"] debug = false`); os binários caíram para ~170 MB e a
pasta `target/debug` local de 17 GB para 4,7 GB. Nenhum teste mudou; o código do workspace
continua com depuração completa.

### Confirmados como corretos (com teste)

- Injeção de sintaxe Typst no conteúdo (`#let`, `[`, `$`, `*`): vira texto literal.
- Palavras longas sem espaço: o Typst quebra; nada passa da margem (medido: borda máxima
  537,6 pt numa página de 595 pt).
- Caracteres de controle e NUL: não quebram o PDF nem o banco.
- Sintaxe do FTS5 digitada pelo analista: neutralizada (teste existente).
- Links simbólicos e junções não são seguidos; a pasta da extração não muda (hash e datas
  antes/depois nos testes e no E2E).
- Transações de escrita são `IMMEDIATE`: a importação (conexão própria) e a tela não
  disputam a trilha.

### FUTURO (não implementar sem decisão)

- F1 Índice `(extracao_id, app, conversa_id_externo, data_hora_ms)` e ordenação indexável
  (nova migração) para páginas de conversas enormes em milissegundos.
- F2 Comando para reconstruir derivados (conversas, FTS) e conferir consistência.
- F3 Assinatura digital ou carimbo de tempo do PDF; ancoragem externa da trilha (hoje o
  relatório é a âncora).
- F4 `ANALYZE`/`PRAGMA optimize` depois de importações grandes.
- F5 Detectar unidade de rede mapeada (`Z:`), que o caminho não revela.
- F6 Hash em várias threads; retomada de importação interrompida.
- F7 Contêineres (`.tar`, `.zip`, `.ab`), HEIC, vídeo.
- F8 Destaque da busca por posições em vez de marcadores no texto (B6).
- F9 Varredura de PDFs órfãos na pasta `relatorios/` (B7).

## 3. Medições (release, Linux do container, uma thread)

`SIED_VOLUME=200000 cargo test --release -p sied-armazenamento --test desempenho -- --ignored --nocapture`

| Operação (200 mil SMS sintéticos, 5.002 conversas) | Antes | Depois |
|---|---|---|
| Importar (inventário, SHA-256, cópia verificada, parser, gravação) | — | 15,7–16,3 s |
| Banco do caso | — | 210 MB |
| Listar conversas (página de 100) | 3,0 s já com 20 mil (quadrático) | 37 ms (4,7 ms com 20 mil) |
| Abrir conversa curta (20 mensagens) | 145 ms | 0,3 ms |
| Página inicial da conversa de 100 mil | 151 ms | 79 ms |
| Última página da conversa de 100 mil | 277 ms | 185 ms |
| Posição de uma mensagem | 97 ms | 43 ms |
| Localizar na conversa de 100 mil (sem acento) | 570 ms | 488 ms |
| Filtrar conversas por texto / telefone | — | 22–33 ms / 31–44 ms |
| Busca global FTS / por telefone | — | 5–6 ms / 311–328 ms |
| Marcar trecho de 2.000 mensagens (limite) | 603 ms | 418 ms |
| Conferir o hash de todos os 200 mil registros | — | 5,9–6,1 s |

`cargo test --release -p sied-relatorio --test desempenho -- --ignored --nocapture`

| Relatório | Resultado |
|---|---|
| 61 evidências de imagens PNG de 12 MP (limite de 60 figuras) | 9,2 s, PDF de 5,6 MB, pico de memória 458 MB |
| Textos hostis (emojis compostos, NUL, RLO, sintaxe Typst, palavra de 5.000 letras) | 70 páginas, nenhum glifo ausente, nada fora da margem |
| Miniatura de foto 16000×12000 com EXIF girado | 1.118 MB / 4,4 s → 573 MB / 1,4 s |

Disco cheio (ocorreu de verdade durante a medição): a importação terminou com status
`falhou` e a mensagem "falha ao gravar no caso: database or disk is full", sem pânico e sem
corromper o caso.

## 4. Testes

| Conjunto | Antes (main `5b68c8d`) | Agora |
|---|---|---|
| Rust (`cargo test --workspace`) | 172 | 201 (+3 medições ignoradas) |
| Interface (`vitest`) | 41 | 43 |
| E2E no app real (`testes/e2e`) | verde | verde (roteiro inalterado) |
| `cargo fmt --check`, `cargo clippy -D warnings`, typecheck, build da UI, tipos ts-rs | limpos | limpos |

Testes novos (principais): hash recalculado com dados hostis e adulterações; trava do caso;
banco vazio, truncado, de outro tipo e `.bak` existente; parser em pânico; imagem-bomba PNG
60000×60000; orientação EXIF; textos hostis no PDF (diagramação conferida glifo a glifo);
filtros sem acento; caminhos perigosos; WAL ilegível; trecho em importação; trilha truncada e
reescrita; `.parcial`; nomes reservados; pasta na nuvem/rede; coordenadas e datas absurdas;
REAL infinito.

## 5. Como executar e testar

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --exit-code -- app/src/api/tipos
cd app && npm ci && npm run typecheck && npm test && npm run build
cd app && npx tauri build --debug --no-bundle
testes/e2e/executar.sh target/debug/sied-desktop   # Linux: xvfb-run, openbox, wmctrl, tauri-driver, WebKitWebDriver
```

Para atualizar a pasta local depois do merge: `git checkout main && git pull origin main`.

## 6. Limitações que continuam

Ver `docs/HANDOFF-CODEX.md` §22 e `docs/CONTINUIDADE.md` §6.

## 7. Próxima fase

Nenhuma iniciada. Ordem recomendada em `docs/HANDOFF-CODEX.md` §27: importador Avilla (aguarda
a árvore real), conferência do WhatsApp com banco de aparelho de teste, contatos e chamadas
do Android, console de importação (§14.1 da continuidade).

## 8. Commit final da `main`

Preenchido depois do merge do PR #11 (o hash do merge só existe depois dele). Conferir com
`git fetch origin && git log --oneline -1 origin/main`.
