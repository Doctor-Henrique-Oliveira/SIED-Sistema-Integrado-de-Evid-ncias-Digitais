# SIED — Passagem de projeto (handoff)

Documento para quem continua o SIED (outro agente, como o Codex, ou uma pessoa). Escrito ao
fim da auditoria pré-produção de 29/09/2026 (PR #11), **a partir do código**, não da
documentação anterior. Onde este documento e o código discordarem, vale o código; corrija o
documento.

Leitura mínima antes de mudar qualquer coisa: §2, §23 (invariantes), §32 (NÃO FAÇA ISSO) e
§28 (arquivos para ler primeiro).

---

## 1. Missão

SIED (Sistema Integrado de Evidências Digitais) é uma plataforma de **análise** forense de
extrações de celulares, **local e 100% offline**, para Windows 11 (compatível com Windows 10).

- **Não faz aquisição.** Importa o resultado de ferramentas de extração (primeiro Avilla
  Forensics; depois IPED, ADB, ALEAPP, iLEAPP, pasta genérica).
- Normaliza tudo para um **modelo interno comum**, guardando a **origem completa** de cada
  registro (arquivo, hash, banco, tabela, id, valores brutos, parser e versão).
- Permite analisar (conversas, busca, mídias, arquivos, contatos), **selecionar evidências** e
  gerar **relatórios PDF** com hashes, metadados e cadeia de custódia.
- Os originais **nunca** são alterados. Tudo o que o SIED produz fica na pasta do caso.

Visão integral e permanente: `docs/VISAO.md`. Arquitetura aprovada (com ajustes):
`docs/ARQUITETURA.md`.

## 2. Estado atual em uma página

- Fase 1 fecha o ciclo **importar → analisar → selecionar → relatar**.
- Na `main`: fundação, importação por pasta com inventário SHA-256 e cópia verificada, parser de
  SMS Android, parser de WhatsApp Android, conversas derivadas, busca FTS5, interface redesenhada
  (12 módulos), interface para volume real (listas virtualizadas, miniaturas, filtros no banco),
  evidências e relatório PDF (Typst embutido), e o hardening desta auditoria.
- **Importador Avilla: só o contrato.** Responde `NaoImplementado`. Aguarda a árvore de uma
  extração real (§26).
- Testes: 284 Rust (+3 medições ignoradas), 62 de interface (vitest), 1 roteiro E2E no app real
  (com o importador ALEAPP/iLEAPP; 257 com o ADB; 248 com o PR #16; 222 e 50 com o PR #15; 201 e 43 após a auditoria).
  CI Linux e Windows.
- Depois do handoff: design system global (PR #14, merge `132a8d5`), **agenda e registro de
  chamadas do Android** (PR #15, merge `9dd1947`, §10.1) e, em construção contínua, **MMS do
  Android** (§10.2), **linha do tempo unificada** (§13.1), **console de importação em tempo
  real** (§9.1) e **importador IPED** (§9.2) (PR #16, merge `b6da666`), e **importador ADB**
  com leitura de `.tar`/`.ab` por dentro (§9.3, PR #17, merge `bcdcd0c`) e **importador ALEAPP/iLEAPP** (§9.4).
  Próximos passos em §27.

## 3. Arquitetura

```
┌──────────────────────── app (Tauri 2) ────────────────────────┐
│  app/src  React + TS (Vite, Tailwind v4, TanStack Query)      │
│     │  invoke() — só comandos do backend; sem fs/shell/rede   │
│  app/src-tauri  comandos.rs → servico.rs (estado: caso aberto)│
└─────┬───────────────┬───────────────┬───────────────┬─────────┘
      │               │               │               │
 sied-ingestao   sied-armazenamento  sied-midia   sied-relatorio
 fonte somente   SQLite global +     miniaturas e  Typst embutido,
 leitura,        SQLite por caso,    imagens do    conferência na
 importadores,   migrações, trilha,  relatório,    geração, PDF +
 parsers,        conversas, busca,   SHA-256       .sha256
 pipeline        evidências          conferido
      └──────────────── sied-dominio (tipos, validação, hash) ──┘
```

Princípios em vigor (todos com teste):

1. **Originais nunca alterados.** A extração é lida por `PastaSomenteLeitura`; nada é gravado
   nela. Bancos da evidência nunca são abertos graváveis: o parser lê uma **cópia de trabalho**
   cujo SHA-256 é conferido com o do inventário.
2. **Importador × parser.** O importador entende a *ferramenta* (quais arquivos, com quais
   papéis); o parser entende o *aplicativo* (como ler o banco). Um parser serve a qualquer
   importador.
3. **Origem completa** de cada artefato (`referencia_origem`).
4. **Somente inclusão.** Inventário, artefatos, origens, auditoria e relatórios têm gatilhos
   que impedem `UPDATE`/`DELETE`. Evidência: alvo e hash imutáveis; remoção só lógica.
5. **Derivados reconstruíveis.** Conversas e índice de busca são mantidos por gatilho a partir
   dos artefatos. Miniaturas ficam fora do banco e podem ser apagadas.
6. **Contratos abertos** a parsers externos futuros (Python, Java), sem introduzi-los agora.

## 4. Tecnologias

| Camada | Tecnologia (versão no repositório) |
|---|---|
| Desktop | Tauri 2.12, `tauri-plugin-dialog` 2.8 |
| Interface | React 19, TypeScript 7, Vite 8, Tailwind 4, TanStack Query 5, TanStack Virtual 3, vitest 5 |
| Núcleo | Rust (edição do workspace), `rusqlite` 0.32 com SQLite embutido (`bundled`, FTS5, funções) |
| Hash | `sha2` (SHA-256); `serde_json` com `float_roundtrip` |
| Mídia | `image` 0.25 |
| Relatório | `typst` 0.15.1, `typst-pdf`, `typst-kit` (fontes embutidas), `typst-layout` |
| Tipos TS | `ts-rs` 10, exportados para `app/src/api/tipos` ao rodar `cargo test` |
| Ids | UUID v7 (`uuid`) para casos; inteiros nos bancos |

## 5. Mapa do repositório

```
Cargo.toml                  workspace e dependências comuns
.cargo/config.toml          TS_RS_EXPORT_DIR (tipos gerados)
.github/workflows/ci.yml    CI (§20)
crates/
  sied-dominio/             caso, dispositivo, artefato (NovoArtefato, hash_registro,
                            normalizar, DataHora::ms_plausivel), ids, validação
  sied-armazenamento/
    migracoes/global|caso/  SQL das migrações (numeradas)
    src/conexao.rs          abrir bancos (Criar/Existente), pragmas, sied_normal()
    src/migracoes.rs        registro e aplicação das migrações, cópia .antes-vN.bak
    src/caso.rs             BancoCaso (API do caso), trava caso.sied.lock, nome de pasta
    src/global.rs           CatalogoGlobal (catálogo de casos, configurações)
    src/importacao.rs       ImportacaoEmCurso: grava inventário e artefatos em lote
    src/extracoes.rs        extrações, arquivos, artefatos, conferir_registro
    src/conversas.rs        conversas derivadas, mensagens, posição, localizar, trecho
    src/busca.rs            busca FTS5 e por telefone
    src/evidencias.rs       evidências (registro, trecho, arquivo), hash_do_trecho
    src/relatorios.rs       tabela relatorio
    src/auditoria.rs        trilha encadeada por hash, verificar, conferir_ancoras
    src/resumo.rs           painel do caso e identificadores (com vínculo à agenda)
    src/agenda.rs           contatos, chamadas, resumo das chamadas e vínculo pela agenda
    src/linha_tempo.rs      linha do tempo do caso (registros com data) e resumo
    tests/                  persistencia, importacao, whatsapp, volume, evidencias,
                            hash_registro, agenda_chamadas, mms, linha_tempo, console,
                            desempenho (ignorado)
  sied-ingestao/
    src/fonte.rs            CaminhoFonte (endereço seguro), FonteSomenteLeitura
    src/fonte/pasta.rs      PastaSomenteLeitura (não segue links, confere cada parte; lista e
                            abre itens de .tar/.ab como "arquivo.ab!/caminho")
    src/fonte/conteiner.rs  leitura de tar (ustar, GNU, pax) e do backup .ab do Android
    src/importador.rs       contrato Importador
    src/importadores/       generico.rs (pasta), iped.rs (caso do IPED), adb.rs (backup .ab),
                            leapp.rs (relatório ALEAPP/iLEAPP),
                            avilla.rs (só contrato)
    src/parser.rs           contrato Parser e SaidaParser
    src/parsers/            android_sms.rs, android_mms.rs, android_contatos.rs, android_chamadas.rs,
                            iped_lista.rs (lista de itens do IPED),
                            leapp_tsv.rs (exportações TSV do ALEAPP/iLEAPP),
                            whatsapp_android/, comum.rs (auxiliares), sqlite.rs (leitura segura)
    src/pipeline.rs         inventário, manifesto, cópias verificadas, execução dos parsers
    src/eventos.rs          eventos do console da importação (fila e releitura do log)
    src/registro.rs         Registro::padrao() (importadores e parsers conhecidos)
    tests/                  contratos, pipeline
  sied-midia/               miniaturas e imagens do relatório (SHA-256 conferido antes)
  sied-relatorio/
    src/montagem.rs         monta os dados do relatório e confere cada evidência
    src/modelo.typ          modelo Typst padrao-v1 (textos entram como dados JSON)
    src/motor.rs            mundo Typst embutido (sem rede, sem pacotes)
    src/lib.rs              gerar: compila, grava .parcial → renomeia, .sha256, registra
    tests/                  relatorio, desempenho (ignorado)
app/
  src/                      interface (rotas/, componentes/, api/, util/)
  src-tauri/src/            comandos.rs (comandos), servico.rs (estado), erro.rs (ErroApp),
                            log.rs (logs técnicos)
  src-tauri/capabilities/   permissões da janela (mínimas)
  src-tauri/tauri.conf.json CSP e empacotamento
testes/
  e2e/                      roteiro ponta a ponta no app real (WebDriver)
  fixtures/                 fixtures SINTÉTICAS (WhatsApp e Android em SQL, imagem, caso do IPED,
                            relatórios ALEAPP/iLEAPP)
  gerador/                  extracao_sintetica.py (--whatsapp, --android, --volume)
  rust/caso_sintetico.rs    caso sintético compartilhado pelos testes Rust (#[path])
docs/                       VISAO, ARQUITETURA, CONTINUIDADE, PREPARACAO-AVILLA-WHATSAPP,
                            checkpoints/ (um por etapa), este HANDOFF
```

## 6. Bancos e pasta do caso

**Banco global** `sied-global.db` (em `%LOCALAPPDATA%\br.sied.desktop\`, ou `SIED_PASTA_DADOS`):
`configuracao` e `caso_catalogo`. Nunca guarda dado de caso.

**Banco do caso** `caso.sied.db`, autocontido, um por caso. Pasta padrão dos casos:
`Documentos\SIED\Casos\<procedimento>` (ou `SIED_PASTA_CASOS`).

```
<pasta do caso>/
  caso.sied.db (+ -wal, -shm enquanto aberto)
  caso.sied.lock                     trava exclusiva enquanto o caso está aberto
  caso.sied.db.antes-vN[-k].bak      cópia de segurança antes de migrar (VACUUM INTO)
  importacoes/<extração>/manifesto.sha256, log-importacao.jsonl
  cache/bancos/<extração>/<arquivo>/ cópias de trabalho conferidas (o parser lê só estas)
  miniaturas/<sha[..2]>/<sha>-v1.jpg derivado; pode ser apagado
  relatorios/AAAA-MM-DD_HHMMSS_relatorio.pdf e .pdf.sha256
```

Conexões (`conexao.rs`):
- `Abertura::Criar` só cria em arquivo inexistente ou vazio; `Abertura::Existente` recusa
  arquivo vazio. O `application_id` é conferido **antes** de qualquer escrita (inclusive
  antes de ativar WAL): arquivo de outro programa não é tocado.
- WAL, `foreign_keys`, `busy_timeout`. Transações de escrita são `IMMEDIATE`.
- Função SQL `sied_normal(texto)`: NFD, sem marcas combinantes, minúsculas Unicode (caminho
  rápido para ASCII). Registrada em toda conexão; usada **só em consultas**, nunca no esquema
  (um banco aberto por outra ferramenta continua legível).
- Erros do SQLite traduzidos para pt-BR com código estável (disco cheio, somente leitura,
  corrompido, ocupado).

**Trava do caso** (`caso.sied.lock`, `File::try_lock`): pega antes de tocar no banco, solta ao
fechar. Segunda abertura (outra janela, outra estação na mesma pasta) recebe `CasoEmUso` →
código `conflito`.

## 7. Migrações

Arquivos em `crates/sied-armazenamento/migracoes/`, registrados em `src/migracoes.rs`, cada uma
conferida por hash (uma migração já aplicada não pode mudar de conteúdo).

| Banco | Versão | Nome | Conteúdo |
|---|---|---|---|
| global | 1 | inicial | `configuracao`, `caso_catalogo` |
| caso | 1 | inicial | `caso` (linha única), `dispositivo`, `auditoria` (somente inclusão) |
| caso | 2 | importacao | `extracao`, `arquivo`, `execucao_parser`, `aviso_parser`, `artefato`, `referencia_origem`; gatilhos de somente inclusão |
| caso | 3 | conversas_busca | `conversa`, `conversa_participante`, FTS5 `busca_artefato`/`busca_arquivo`, gatilhos de derivação, preenchimento |
| caso | 4 | whatsapp | `artefato.anexo_vinculo`, índices `idx_arquivo_nome`, `idx_artefato_id_externo` |
| caso | 5 | evidencias_relatorio | `evidencia` (alvo/hash imutáveis, sem `DELETE`), `relatorio` (somente inclusão) |
| caso | 6 | ordem_conversa | índice `idx_artefato_conversa_ordem` no lugar de `idx_artefato_conversa` (só índice) |

Regras:
- Caso antigo migra ao abrir, **depois** da cópia `.antes-vN.bak` (nome livre: `-2`, `-3`...
  se já existir).
- Banco com versão **maior** que a do programa: recusado antes de migrar (`VersaoMaisNova` →
  código `versao_incompativel`). Migração já aplicada com conteúdo diferente do embutido:
  `MigracaoDivergente`.
- Ao criar a migração N: os testes que simulam casos antigos (`tests/importacao.rs`, v1/v2)
  precisam desfazer também a N (precedente da v3, v4, v5 e v6; os de `whatsapp.rs` e
  `persistencia.rs` também).
- A auditoria não criou migração; a 6 (01/10/2026) só troca um índice.

## 8. Contratos

`crates/sied-ingestao/src/importador.rs`:
```rust
pub trait Importador: Send + Sync {
    fn info(&self) -> InfoComponente;
    fn detectar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao>; // rápido
    fn planejar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao>;
}
```
`Deteccao { confianca 0..1, suporte: Completo|Parcial|NaoImplementado|NaoReconhecido, motivos }`.
`PlanoImportacao { ferramenta_versao, tipo_extracao, atributos_dispositivo (com origem), tarefas:
[TarefaParser { parser_id, arquivos: [{ papel, caminho }] }], observacoes }`.

`crates/sied-ingestao/src/parser.rs`:
```rust
pub trait Parser: Send + Sync {
    fn info(&self) -> InfoComponente;
    fn papeis(&self) -> &'static [&'static str];
    fn processar(&self, entrada: &EntradaParser, saida: &mut dyn SaidaParser)
        -> Resultado<ResumoParser>;
}
```
- `EntradaParser.arquivos`: `ArquivoDeTrabalho { papel, origem, sha256, copia_verificada }`.
  `-wal`/`-shm` são copiados ao lado da cópia com o mesmo nome base.
- Registro ilegível vira `AvisoParser`, não erro. Erro só quando nada pode ser lido.
- `saida.cancelado()` deve ser consultado periodicamente.
- Pânico dentro de `processar` é capturado (`catch_unwind`): a execução fica como falha, o que
  foi emitido antes é gravado, os demais parsers seguem.

`CaminhoFonte` (`fonte.rs`): endereço relativo com `/`, validado. Recusa absoluto, `..`, `.`,
componente só de pontos ou só de espaços, e qualquer componente com unidade (`C:`) — no Windows,
`PathBuf::push("C:")` trocaria o caminho inteiro. `PastaSomenteLeitura` confere de novo cada parte
com as regras da plataforma (`Component::Normal`), não segue links simbólicos nem junções, e
transforma nome perigoso da listagem em **item ignorado com aviso** (a importação continua).

## 9. Importadores

| Id | Arquivo | Situação |
|---|---|---|
| `generico` | `importadores/generico.rs` | Pasta qualquer. Reconhece `mmssms.db` (SMS e MMS), `msgstore.db` + `wa.db` do WhatsApp (pareados pela pasta; `com.whatsapp` e `com.whatsapp.w4b`), `contacts2.db` (agenda; sem `calllog.db` na mesma pasta, também vai ao leitor de chamadas) e `calllog.db` (chamadas); `.crypt*` só inventariados |
| `iped` | `importadores/iped.rs` | Caso do IPED: lista de itens na raiz (6 nomes por idioma) → `iped_lista_arquivos`; fora de `iped/`, o mesmo plano da pasta genérica; §9.2 |
| `adb` | `importadores/adb.rs` | Backup `adb backup` (`.ab`), sozinho ou com arquivos de `adb pull`: observações do cabeçalho e dos `_manifest`; o mesmo plano da pasta genérica, inclusive dentro do backup; §9.3 |
| `leapp` | `importadores/leapp.rs` | Relatório do ALEAPP/iLEAPP: `_TSV Exports/*.tsv` → `aleapp_tsv`, `ileapp_tsv` ou `xleapp_tsv` conforme o `_lava_data.lava`; o resto só no inventário; §9.4 |
| `avilla` | `importadores/avilla.rs` | **Só contrato.** `detectar` → `NaoImplementado`; `planejar` → erro `NaoImplementado` |

Pipeline (`pipeline.rs`), nesta ordem, igual para toda ferramenta:
1. contagem;
2. inventário de **todos** os arquivos com SHA-256, categoria e tamanho, e manifesto
   `manifesto.sha256` (formato `sha256sum`);
3. `planejar` do importador;
4. para cada tarefa: cópia de trabalho de cada arquivo, conferida por SHA-256; auxiliar
   `-wal`/`-journal` ilegível gera **aviso** e execução parcial; parser; gravação em lote (500).

Artefato: `normalizar()` (−0.0 → 0.0; `app` vazio → ausente) e depois `hash_registro()`.

### 9.1 Console da importação

- Cada passo real do pipeline chama `Log::anunciar`: vai à fila do `Controle` (`eventos.rs`, até
  5.000 em memória, consulta pela sequência) e ao `log-importacao.jsonl` com `mensagem` e `tom`.
- Limites: 30 bancos SQLite e 20 avisos por parser anunciados um a um; gravação a cada 5.000
  registros. O banco do caso continua com tudo (inventário, avisos, execuções).
- Comandos `eventos_importacao(desde)` (importação em curso) e `eventos_da_extracao(id)` (relê o
  log; `BancoCaso::eventos_da_importacao`). As duas linhas do fim (gravação do resultado) são só
  ao vivo. Interface: `app/src/componentes/ConsoleEventos.tsx`.
- Mensagem nova de console = texto em pt-BR, frase completa, sem inventar dado.

### 9.2 IPED

- Formato conferido no código-fonte público do IPED (sepinf-inc/IPED: `ExportCSVTask`,
  `ExportFileTask`, `iped-engine-messages*.properties`). Não invente além disso.
- Detecção: `iped/IPEDConfig.txt`, `iped/index`, `iped/conf` e/ou lista com cabeçalho válido
  (15 colunas; `MD5`, `SHA1` e `TrackId` nas posições fixas). `Parcial` sempre: o índice Lucene
  e os bancos internos não são lidos (mudam com a versão).
- Lista: UTF-8 com BOM, CRLF, `;` (pt, de) ou `,`; datas de `java.util.Date.toString()` no fuso
  da máquina do IPED. Só `UTC/GMT/UT`, `GMT±hh:mm` e `BRT/BRST/FNT` são convertidos; o resto fica
  no original com um aviso. O atalho `=HIPERLINK("Exportados\arquivos\E\0\<MD5>.png";"Abrir")`
  é relativo à raiz do caso e liga o item ao inventário pelo caminho exato.
- Fixture: `testes/fixtures/iped/` (`-text` no `.gitattributes`); gerador `--iped`.

### 9.3 ADB e contêineres

- Formato público do Android (AOSP): cabeçalho de texto `ANDROID BACKUP`, versão, compressão
  (`1` = zlib) e cifra (`none` ou `AES-256`), seguido de um tar com `apps/<pacote>/{_manifest,
  db,f,sp,r}` e, se pedido, `shared/`.
- `fonte/conteiner.rs` lê o tar em fluxo (ustar com prefixo, nome longo GNU `L`, pax `x` com
  `path`/`size`, `g`/`K` ignorados, checksum conferido, tamanho em base-256). Nada é extraído.
- `PastaSomenteLeitura` lista os itens de dentro logo depois do contêiner, com o separador `!/`
  (`SEPARADOR_CONTEINER`). Duplicados (vale o primeiro), nomes inseguros, links, itens especiais,
  `.ab` cifrado e erro de leitura no meio viram `ItemIgnorado` com o motivo. Um `.ab` sem o
  cabeçalho é tratado como arquivo comum. Contêiner dentro de contêiner não é aberto.
- A leitura mantém um cursor por contêiner: ler os itens na ordem da listagem descomprime cada
  contêiner uma vez; voltar atrás recomeça do início. `LeitorFonte` deixou de exigir `Seek`.
- `listar_sem_conteineres()` serve à detecção (rápida; não abre contêineres).
- Fixture: gerada nos testes por `testes/rust/backup_sintetico.rs` a partir das fixtures SQL;
  gerador `--adb`. `atributos_dispositivo` (ex.: `api_android`) ainda não é gravado no caso.

### 9.4 ALEAPP e iLEAPP

- Formato conferido no código-fonte público (abrignoni/ALEAPP e abrignoni/iLEAPP:
  `scripts/ilapfuncs.py` função `tsv`, `scripts/lavafuncs.py`). Os dois gravam igual.
- TSV: `csv.writer` do Python com TAB, UTF-8 com BOM, CRLF, aspas só quando preciso; arquivo em
  acréscimo, então o cabeçalho pode se repetir (com outro BOM, no ALEAPP). O nome do arquivo é o
  nome do artefato (pode ter espaço no fim, como "Call logs ").
- Datas: `str(datetime)` do Python. Só com fuso explícito (`+00:00`, `-03:00`, `Z`) entram na
  linha do tempo; sem fuso, ficam no original com um aviso por arquivo.
- Um parser por ferramenta (mesmo código, `ParserLeappTsv(Ferramenta)`), para a execução e o
  `app` dizerem de onde veio. `ferramenta_versao` da extração vem do `_lava_data.lava`.
- O banco `_lava_artifacts.db` não é lido (esquema muda com a versão). Mapear artefatos
  conhecidos (SMS, chamadas...) para os tipos do SIED fica para depois, com conferência.
- Fixtures: `testes/fixtures/leapp/` (gravadas por `montar.py`; `-text`); gerador `--leapp`.

## 10. Parsers

| Id | Papéis | Versão | O que lê |
|---|---|---|---|
| `android_sms` | `mmssms` | 1.0.0 | tabela `sms` de `mmssms.db`, inclusive registros só no `-wal` |
| `android_mms` | `mmssms` | 1.0.0 | tabelas `pdu`, `addr`, `part` do mesmo `mmssms.db`; §10.2 |
| `whatsapp_android` | `msgstore`, `wa` | 1.0.0 | §11 |
| `android_contatos` | `contacts2` | 1.0.0 | agenda (`raw_contacts`, `data`, `mimetypes`, `accounts`, `contacts`); §10.1 |
| `android_chamadas` | `chamadas` | 1.0.0 | tabela `calls` de `calllog.db` (ou de `contacts2.db` antigo); §10.1 |
| `iped_lista_arquivos` | `lista_iped` | 1.0.0 | lista de itens do IPED em CSV; §9.2 |
| `aleapp_tsv`, `ileapp_tsv`, `xleapp_tsv` | `tsv_leapp` | 1.0.0 | exportações TSV do ALEAPP/iLEAPP; §9.4 |

`parsers/sqlite.rs`: abre a **cópia** com `SQLITE_OPEN_READ_ONLY` e lê valores brutos. REAL não
finito vira `{"real_nao_finito": "inf"|"-inf"|"nan"}` (JSON não tem infinito).

Datas: só viram `utc_ms` se `DataHora::ms_plausivel` (0 < ms ≤ 31/12/9999 23:59:59.999 UTC).
Fora disso ficam só no valor original. Regra geral: **"não sei" é melhor que inventar**.

### 10.1 Agenda e chamadas (Android)

- Contato: um artefato `contato` (app `contatos_android`) por **contato bruto**; o agrupamento
  do Android (`contacts`) não é refeito (`contato_id` nos metadados). Telefones em
  `metadados.telefones` (`numero`, `tipo`, `rotulo`, `normalizado`, `registro`), conta em `conta`.
  Cada linha de `data`, a conta e o contato agrupado são origens adicionais; a foto não é copiada
  (só `foto_registro`). Linha de `data` órfã ou mimetype desconhecido geram aviso.
- Chamada: artefato `chamada` (app `telefone`), subtipo pelo `type`; `de_mim` só na efetuada;
  número em `remetente` (ou `destinatarios` na efetuada). Códigos antigos `-1`…`-4` ficam em
  `metadados.numero_especial`, nunca como telefone. `deleted=1` → estado "apagado".
- Vínculo com a agenda (`sied-armazenamento/src/agenda.rs`): mesma extração e igualdade exata dos
  dígitos (`sied_digitos`) do número ou do normalizado do aparelho (`normalized_number` da
  chamada, `data4` do telefone). Calculado na consulta; nada é gravado. Vale para chamadas e para
  identificadores das conversas que são telefone.
- Conferidos só com fixtures sintéticas (`testes/fixtures/android`), pela documentação pública.

### 10.2 MMS (Android)

- Um artefato `mensagem` (app `sms`) por linha de `pdu`, na **mesma conversa** (`thread_id`)
  dos SMS. `date` em **segundos** (`FormatoData::UnixSegundos`).
- `addr`: 137 de, 151 para, 130 cópia, 129 cópia oculta; `insert-address-token` (o próprio
  aparelho) nunca vira telefone. `part`: texto das partes `text/plain` na ordem de `seq` (sem
  texto, o assunto); a primeira mídia vira o anexo (`_data` em `app_parts`, ligado ao inventário
  pelo final do caminho); todas as partes em `metadados.partes`, e a contagem das mídias além da
  primeira em `outras_midias`. Cada linha de `addr` e `part` é origem adicional.
- `m_type` 130 (notificação de MMS não baixado) → subtipo `mms_nao_baixado`. Partes/endereços
  sem `pdu` correspondente geram aviso. Banco sem `pdu`: nada a ler, sem aviso.
- O importador genérico entrega todo `mmssms.db` aos dois leitores (SMS e MMS).

## 11. WhatsApp (Android)

- `msgstore.db` nos esquemas **novo** (`message`) e **antigo** (`messages`), detectados pelo
  conteúdo, não pela versão.
- `wa.db` para contatos.
- Mensagens de texto, mídia, localização, contato, sistema, resposta, encaminhada, apagada;
  situação de envio; grupos e membros.
- Coordenada fora de ±90/±180 ou incompleta: não interpretada, marca `coordenadas_invalidas`,
  valores ficam nos brutos.
- Registro `conversa` do app fica separado das mensagens (sem `conversa_id_externo`).
- Título, tipo e nomes são resolvidos **na consulta**, a partir dos registros da mesma extração,
  nunca gravados na conversa derivada.
- Anexo só é ligado ao arquivo do inventário se o caminho for exato ou o sufixo for único **e**
  o tamanho conferir; o motivo fica em `artefato.anexo_vinculo`.
- Conferido só com bancos **sintéticos** (`testes/fixtures/whatsapp/*.sql`), com nomes da
  referência pública do app. Não lidos: `.crypt*`, `@lid`/`jid_map`, reações, enquetes,
  edições, chamadas do app, dono da conta.

## 12. Conversas

- Tabela `conversa` derivada por gatilho de `(extracao_id, app, conversa_id_externo)` dos
  artefatos. Nada de vínculo fixo gravado na mensagem.
- Consultas da conversa usam `DA_CONVERSA` (`a.app IS nullif(?2, '')`) e
  `artefato a INDEXED BY idx_artefato_conversa`. O `INDEXED BY` é deliberado: sem
  estatísticas (`ANALYZE`), o SQLite escolhia o índice errado e percorria a extração inteira.
- Ordem das mensagens: `data_hora_ms IS NULL, data_hora_ms, id`.
- Filtros no banco (caso inteiro): texto (título, participante, nome no app, telefone com ou sem
  máscara) por `sied_normal`, app, individual/grupo, com anexo.
- `posicao_mensagem`, `localizar_na_conversa` (total + até o limite pedido), trecho até
  `TRECHO_MAXIMO` = 2.000 mensagens.

## 13. Busca

- FTS5 `unicode61 remove_diacritics 2` em registros e nomes de arquivo.
- Texto digitado é neutralizado: cada palavra vira `"palavra"*` (sintaxe do FTS não passa).
- Telefone (6+ dígitos, com separadores): `instr` nos dígitos de remetente e destinatários (e,
  para contatos, também nos telefones da agenda e no normalizado).
- Trecho destacado: o backend pede ao FTS o trecho com e sem marcadores e o separa em pedaços
  (`PedacoTrecho { texto, destaque }`, `busca::pedacos`); a interface só pinta os pedaços. Texto
  do registro com os mesmos caracteres dos marcadores não muda o destaque (B6 resolvido).
- Conversas, participantes e os dois índices FTS são **derivados** (migração 3, gatilhos).
  `derivados.rs` confere cada tabela contra o cálculo a partir de `artefato`/`arquivo` (`EXCEPT`
  nos dois sentidos + `integrity-check` do FTS) e reconstrói numa transação auditada
  (`derivados_reconstruidos`): conversas corrigidas pela chave (o `id` das que continuam
  existindo não muda), participantes refeitos, FTS refeito com `rebuild` e repreenchido. Só
  confirma se a conferência depois não achar diferença. Tela: Dashboard → Auditoria → "Conversas
  e índices de busca" (Conferir; Reconstruir só aparece com diferença). Recusada durante uma
  importação. 200 mil registros (release): conferir ~3 s, reconstruir ~10 s.
  **Ao mudar um gatilho da migração 3 numa migração nova, mude também as consultas de
  `derivados.rs`** (são a mesma regra escrita duas vezes).

### 13.1 Linha do tempo

- `linha_tempo.rs`: registros com `data_hora_ms` (todas as extrações e apps), `INDEXED BY
  idx_artefato_tempo`, ordem por data e id (estável na paginação); filtros de extração, tipos, app,
  período e coordenadas. O tipo `conversa` (cabeçalho do app) não entra.
- `resumo_linha_do_tempo`: por tipo sem o filtro de tipo, apps sem o filtro de app, primeira e
  última data e `sem_data` (sem o filtro de período, excluídos os contatos).
- Interface: `app/src/componentes/LinhaTempo.tsx` (usado em Consultas e, fixo em coordenadas, em
  Localizações); `destinoRegistro` decide onde abrir cada registro (também na busca).

## 14. Mídia e miniaturas

- `sied-midia`: lê da pasta da extração (somente leitura), confere o SHA-256 com o inventário
  **antes** de decodificar, gera JPEG (lado 320 px) em `miniaturas/`. Relatório: lado 1.600 px.
- Arquivos até 256 MB; JPEG, PNG, GIF, WebP, BMP, TIFF. HEIC, vídeo e áudio sem miniatura.
- Limites de memória: até 4 decodificações simultâneas; imagem que decodifica em mais de
  256 MB é exclusiva (uma por vez); redução **antes** de aplicar a orientação EXIF;
  `catch_unwind` na decodificação. Temporário com pid e sequência (sem colisão).
- A imagem chega à tela como `data:` (a CSP só aceita imagens locais).

## 15. Evidências

- Tipos: registro, trecho (início e fim, mesma conversa, até 2.000 mensagens) e arquivo.
- Hash gravado na marcação: `hash_registro`; SHA-256 das linhas `id:hash_registro\n` do trecho
  (`hash_do_trecho`); SHA-256 do inventário para arquivo.
- Alvo e hash imutáveis por gatilho; repetição recusada; remoção lógica com motivo; tudo
  auditado.
- Trecho é recusado enquanto a extração estiver `em_andamento` (a conversa ainda pode crescer).

## 16. Relatório PDF

- `sied-relatorio::gerar`: modelo `padrao-v1`, 10 seções escolhidas na tela (capa, procedimento,
  dispositivos, aquisição e importação, metodologia, evidências, observações, conclusão,
  apêndice técnico com origem e valores brutos, integridade e cadeia de custódia).
- Chamada e contato entram com campos próprios (tipo e duração; telefones, conta, empresa).
- **Conferência na geração**, por evidência:
  - registro e cada mensagem do trecho: o artefato é **remontado das colunas e origens
    gravadas** e o `hash_registro` é **recalculado** (`extracoes::conferir_registro`); nunca
    se confia na coluna `hash_registro` sozinha;
  - arquivo: SHA-256 relido da extração;
  - resultado: `conferida`, `divergente` (quadro de ATENÇÃO, lista do que mudou) ou
    `indisponivel` (pasta da extração ausente).
- Trilha de auditoria quebrada **impede** a geração.
- Textos entram no Typst como **dados JSON**, nunca como marcação. Emojis, controles e
  invisíveis sem glifo nas fontes embutidas viram `[U+XXXX]` (explicado na metodologia).
- No máximo 60 figuras. Typst compila dentro de `catch_unwind`.
- Gravação: `.parcial` → renomeia para `AAAA-MM-DD_HHMMSS_relatorio.pdf` (nunca sobrescreve) →
  `.pdf.sha256` → linha em `relatorio` com SHA-256 e âncora da trilha
  (`auditoria_ate_seq`, `auditoria_hash`). Falha em qualquer passo, inclusive no registro, apaga
  o que já foi gravado (`.parcial`, PDF, `.sha256`). Nunca existe linha em `relatorio` apontando
  para PDF parcial ou ausente; só uma queda do processo no meio deixa um PDF órfão (B7). A tela
  de Relatórios confere a pasta (`BancoCaso::conferir_pasta_relatorios`): cada PDF registrado
  aparece como "PDF confere", "PDF alterado" ou "PDF ausente" (SHA-256 relido), e o que não
  tem registro (PDF, `.parcial`, `.sha256` avulso) é listado com aviso. Só lê; nada é apagado.

## 17. Auditoria

- `auditoria.rs`: cada registro guarda `hash = SHA-256(seq␟ts␟usuario␟maquina␟acao␟detalhes␟anterior)`
  (␟ = `\x1f`), a partir de `HASH_GENESE` (64 zeros). Somente inclusão por gatilho.
- `registrar` é chamado **na mesma transação** da ação auditada.
- `verificar` recalcula a cadeia **e** confere as âncoras gravadas pelos relatórios
  (`conferir_ancoras`): apagar os últimos registros ou reescrever a trilha inteira fora do SIED
  é denunciado se já houve relatório depois desse ponto.
- Usuário e máquina vêm do sistema operacional (`Sessao`).

## 18. Segurança, Tauri e privacidade

- Capacidade da janela: só `core:default` e `dialog:allow-open`. Sem fs, shell, http ou
  processos no frontend: todo disco passa pelos comandos do backend.
- CSP: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
  img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost`. Sem rede.
- Comandos são `async` e rodam em `spawn_blocking` (a janela não congela).
- Extração hostil: caminhos perigosos ignorados com aviso; links não seguidos; bancos lidos só em
  cópia aberta somente leitura; imagem-bomba limitada; pânico capturado nas fronteiras (parser, pipeline,
  imagem, Typst).
- Logs técnicos em `<dados>/logs/sied.log.AAAA-MM-DD` (nível `SIED_LOG`): operações e erros,
  **nunca conteúdo de evidência**. O log da importação (`log-importacao.jsonl`) fica na pasta do
  caso, que é dado de investigação.
- A tela de novo caso avisa quando a pasta está em OneDrive, Dropbox, Google Drive, iCloud ou
  compartilhamento de rede (UNC). Não bloqueia: o analista decide.

## 19. Erros

`app/src-tauri/src/erro.rs`: `ErroApp { codigo, mensagem (pt-BR), campos }`, com
`codigo ∈ validacao | nao_encontrado | nenhum_caso_aberto | conflito | versao_incompativel |
integridade | nao_suportado | interno`. A interface decide pelo código, nunca pelo texto.
Erros de SQLite, disco e trava chegam traduzidos. Nenhum caminho conhecido termina em pânico
não capturado.

## 20. CI

`.github/workflows/ci.yml`:
- **Testes (Linux)**: build da interface (`npm run build` = `tsc` + Vite, o que inclui o
  typecheck), `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, tipos ts-rs
  sem diferença (`git diff --exit-code app/src/api/tipos`) e vitest.
- **Testes (Windows)**: `cargo test --workspace`.
- **Instaladores Windows (NSIS e MSI)**: só em tag `v*`, execução manual (`workflow_dispatch`)
  ou PR que muda o build (`app/src-tauri/`, `Cargo.toml`/`Cargo.lock`, `app/package*.json`,
  `app/vite.config.*`, `app/index.html`, `app/recursos/`, `app/public/`, `.cargo/`,
  `.github/workflows/`).
  Artefato `sied-windows-instaladores`.
- Mudanças só em `docs/**`, `*.md` ou `*.png` não disparam a CI (use `workflow_dispatch` se
  precisar).
- O E2E **não** roda na CI (precisa de janela real); roda localmente (§29).
- Builds de desenvolvimento e de teste compilam as **dependências sem informação de depuração**
  (`[profile.dev.package."*"] debug = false` no `Cargo.toml`). Com depuração, cada binário de
  teste que liga o Typst passava de 500 MB (hoje ~170 MB) e o disco do executor Linux acabava
  no meio da ligação (`rust-lld` terminava com "Bus error").
- Mudar gatilhos da CI ou o que os testes executam exige plano aprovado pelo usuário.

## 21. Testes

| Conjunto | Onde | Quantidade |
|---|---|---|
| Rust | `crates/*/src` (unitários, inclusive os que exportam os tipos TS) e `crates/*/tests` | 284 + 3 medições `#[ignore]` |
| Interface | `app/src/**/*.test.ts(x)` | 57 |
| E2E | `testes/e2e/executar.sh` + `persistencia.mjs` | 1 roteiro (cria caso, importa, analisa, reabre; confere que a extração não mudou: hash e datas antes/depois) |

Medições (não rodam na bateria normal):
```
SIED_VOLUME=200000 cargo test --release -p sied-armazenamento --test desempenho -- --ignored --nocapture
cargo test --release -p sied-relatorio --test desempenho -- --ignored --nocapture
```
Resultados em `docs/checkpoints/2026-09-29-auditoria-hardening.md` §3.

Testes de UI com virtualização: jsdom precisa de `offsetHeight`, `offsetWidth`, `clientHeight`,
`scrollHeight` e `Element.scrollTo` definidos no próprio arquivo de teste (ver
`app/src/rotas/VolumeReal.test.tsx`).

## 22. Limitações conhecidas

Funcionais:
- Avilla sem importador (§26). `.tar` e `.ab` sem cifra lidos por dentro (§9.3); `.zip`, `.ab`
  cifrado e contêiner dentro de contêiner só inventariados.
- Interpretados: SMS e MMS, WhatsApp Android, agenda e chamadas do Android. Sem fotos de
  contato, `deleted_contacts`, chamadas de apps nem junção de contatos repetidos. MMS: só a
  primeira mídia vira anexo (as demais ficam listadas nas partes); RCS não é lido.
- WhatsApp, MMS, agenda e chamadas conferidos só com bancos sintéticos (§10.1, §10.2, §11).
- IPED: só a lista de itens em CSV; conferida só com o caso sintético (§9.2).
- ADB: conferido só com backup sintético; `f/`, `sp/` e `shared/` só inventariados (§9.3).
- ALEAPP/iLEAPP: só `_TSV Exports`; linhas ficam como "outro registro"; conferido só com
  relatórios sintéticos (§9.4).
- Hash numa única thread; sem retomada de importação interrompida; sem exclusão de extração
  (de propósito); sem verificação do "hash declarado" do laudo de aquisição.
- Miniaturas: sem HEIC, vídeo e áudio. Mídias e Arquivos mostram uma extração por vez; listas de
  Arquivos e Contatos paginadas de 100.
- Localizações só em lista (sem mapa offline); Consultas só com a linha do tempo (sem consultas
  salvas); Configurações somente leitura; tema único.
- Relatório: até 60 figuras; a geração segura o caso (outros comandos esperam); sem botão
  "Abrir PDF"; um modelo; sem assinatura digital; usuário é o do sistema operacional.

Da auditoria (baixo risco, documentados):
- ~~**B6**~~ Resolvido em 01/10/2026: o trecho da busca chega em pedaços (§13).
- **B7** Se o SIED cair entre renomear o PDF e registrá-lo, fica um PDF com `.sha256` sem linha
  em `relatorio` (nunca o contrário). Desde 01/10/2026 a tela de Relatórios lista esses arquivos
  como "sem registro no caso" e confere o SHA-256 de cada PDF registrado (§16).
- **B8** Conversa de 100 mil mensagens (200 mil registros, release): desde a migração 6, página
  7–14 ms, posição ~6 ms, localizar ~0,14 s, marcar trecho de 2.000 ~11 ms (eram 58–148 ms,
  34 ms, 0,53 s e 0,5 s). Busca por telefone continua ~0,26 s (varre os registros; um índice
  de dígitos seria nova tabela derivada).
- **B9** A trava impede copiar a pasta **com o caso aberto** no Windows; copiar aberto também
  perderia o `-wal`. Copie o caso **fechado**.
- Caso em compartilhamento de rede ou pasta sincronizada: WAL e trava não são confiáveis fora do
  disco local. A tela avisa (Novo caso e Alertas do Dashboard). Unidade de rede mapeada (`Z:`)
  é reconhecida pelo sistema desde 01/10/2026: comando `pasta_em_rede` (`disco.rs`; Windows
  `GetDriveTypeW` = `DRIVE_REMOTE`, Linux tipo do sistema de arquivos em
  `/proc/self/mountinfo`).

## 23. Invariantes (não quebrar)

1. Nada é gravado na pasta da extração. Bancos da evidência só em cópia conferida, abertos
   somente leitura.
2. `hash_registro` = SHA-256 do JSON compacto de `NovoArtefato` (campos na ordem da declaração,
   chaves de mapa em ordem alfabética, `float_roundtrip`), calculado **depois** de
   `normalizar()`. Mudar a regra, os campos de `NovoArtefato`, a feature `float_roundtrip` ou
   ativar `preserve_order` no `serde_json` faz **todo registro já importado divergir**.
3. O relatório recalcula o hash a partir do banco; nunca confia na coluna.
4. Tabelas de fato são somente inclusão (gatilhos). Evidência: alvo e hash imutáveis.
5. Trilha: registro na mesma transação da ação; cadeia + âncoras dos relatórios.
6. Nunca uma linha em `relatorio` para PDF parcial ou ausente; nunca sobrescrever PDF.
7. Conversas, FTS e miniaturas são derivados: podem ser reconstruídos; não guardar neles nada que
   não exista nos artefatos.
8. `sied_normal` só em consultas, nunca em esquema, gatilho ou índice.
9. Frontend sem acesso direto a disco, shell ou rede.
10. Data implausível não vira data; coordenada inválida não vira coordenada.
11. Migração aplicada nunca muda; cópia `.bak` antes de migrar.
12. Fixtures e dados de teste sempre **sintéticos** e identificados como tais.

## 24. Decisões técnicas

1. Stack Tauri + React/TS + Rust + SQLite/FTS5 + Typst (aprovada pelo usuário).
2. Banco global pequeno + banco autocontido por caso, migrações próprias.
3. Importador (ferramenta) separado de parser (app).
4. Trabalho só em cópia verificada.
5. Conversa derivada (sem vínculo fixo na mensagem); título/nomes resolvidos na consulta.
6. Busca: palavra → `"palavra"*`; telefone por dígitos.
7. Tipos TS gerados do Rust; a CI falha se desatualizados.
8. Evidência guarda o hash do momento da seleção; relatório mostra esse e o conferido.
9. Relatório somente inclusão com âncora da trilha; trilha quebrada impede gerar.
10. CI "Só tags" para instaladores (escolha do usuário).
11. Auditoria de 29/09/2026: hash **recalculado** na geração; trava por arquivo; datas e
    coordenadas plausíveis; `INDEXED BY` + `nullif` nas conversas; `sied_normal`; âncoras da
    trilha; comandos assíncronos; `catch_unwind` nas fronteiras; aviso de nuvem/rede.

Mais detalhes e histórico: `docs/CONTINUIDADE.md` §7.

## 25. Dívida técnica e riscos

| Item | Risco | Proposta |
|---|---|---|
| ~~Conversas enormes sem índice de ordenação~~ | **feito em 01/10/2026 (F1):** migração 6 e leitura em duas partes do índice; números no B8 | — |
| ~~Sem comando de reconstrução dos derivados~~ | **feito em 01/10/2026 (F2):** conferência e reconstrução de conversas, participantes e índices FTS (§13) | — |
| Sem `ANALYZE` | plano de consulta ruim em consultas novas | F4: **medido em 01/10/2026 sem ganho** em nenhuma consulta da medição (as críticas usam `INDEXED BY`); não adotado. Reavaliar se uma consulta nova sem `INDEXED BY` ficar lenta |
| PDF sem assinatura; trilha ancorada só no próprio caso | quem reescreve banco e PDFs juntos não é detectado | F3: assinatura/carimbo de tempo; âncora externa |
| Hash numa thread | importações grandes lentas | F6: **medido em 01/10/2026 sem ganho que justifique**: SHA-256 de uma thread com instruções SHA da CPU passa de 1 GB/s, acima da leitura de um disco de evidência típico (USB/HD); o gargalo é o disco. Reavaliar com SSD NVMe e medição de importação real |
| ~~PDF órfão após queda (B7)~~ | **feito em 01/10/2026 (F9):** conferência da pasta e aviso na tela de Relatórios | — |
| ~~Marcadores de destaque no texto (B6)~~ | **feito em 01/10/2026 (F8):** trecho em pedaços montado no backend | — |
| ~~Unidade mapeada `Z:` não detectada~~ | **feito em 01/10/2026 (F5):** `pasta_em_rede` pelo sistema operacional; aviso no Novo caso e nos Alertas do Dashboard | — |
| WhatsApp sem banco real | esquemas reais podem ter variações | conferir com aparelho de teste |
| `typescript` 7 / `vite` 8 recentes | quebra em atualizações | fixar versões ao atualizar; rodar toda a bateria |

## 26. O que depende do Avilla

Só o **importador Avilla** (`crates/sied-ingestao/src/importadores/avilla.rs`). Falta, do usuário:

1. árvore de pastas de uma extração de teste (`tree /F /A > arvore.txt`, só nomes);
2. versão do Avilla;
3. opções marcadas (backup ADB, downgrade do WhatsApp, cópia de pastas etc.);
4. nome do arquivo de hash/log da aquisição, se houver;
5. de preferência, uma segunda árvore.

Com isso: fixture sintética com os mesmos nomes, `detectar` e `planejar`, testes de detecção,
plano e pasta intacta, e importação completa no app real. Roteiro em
`docs/PREPARACAO-AVILLA-WHATSAPP.md` §2. Formato a documentar em `docs/formatos/avilla.md`.

**Não invente** caminhos, bancos, nomes de arquivos, estrutura ou formatos do Avilla.

## 27. Próximos passos recomendados

Construção contínua (regra do usuário de 30/09/2026): cada fase vai para a `main` com a CI
verde. Feitas: agenda e chamadas (PR #15), MMS, linha do tempo, console de importação e
importador IPED (lista de itens), importador ADB (backup `.ab` e `.tar`), importador
ALEAPP/iLEAPP (exportações TSV), conversas grandes pelo índice (F1), conferência da pasta de
relatórios (F9), destaque da busca em pedaços (F8), conferência/reconstrução dos dados
derivados (F2) e aviso de unidade de rede mapeada (F5). Ordem seguinte:

1. **Importador Avilla**, quando a árvore real chegar (§26).
2. **Conferência do WhatsApp, SMS/MMS, agenda, chamadas, da lista do IPED, do backup ADB e dos
   relatórios ALEAPP/iLEAPP** com dados de teste (não de investigação).
3. Item F3 (§25: assinatura do PDF e âncora externa da trilha), que depende de decisões do
   usuário (certificado, carimbo de tempo, onde ancorar). F1, F2, F5, F8 e F9 feitos; F4 e F6
   medidos sem ganho.

## 28. Arquivos para ler primeiro

1. `docs/VISAO.md`, `docs/ARQUITETURA.md`, este documento.
2. `crates/sied-dominio/src/artefato.rs` (modelo, `hash_registro`, `normalizar`, `DataHora`).
3. `crates/sied-ingestao/src/importador.rs`, `parser.rs`, `fonte.rs`, `pipeline.rs`.
4. `crates/sied-ingestao/src/importadores/generico.rs` e `parsers/android_sms.rs` (o menor
   parser completo).
5. `crates/sied-armazenamento/src/conexao.rs`, `migracoes.rs`, `caso.rs`, `auditoria.rs`.
6. `crates/sied-armazenamento/migracoes/caso/*.sql`.
7. `crates/sied-relatorio/src/montagem.rs` e `lib.rs`.
8. `app/src-tauri/src/comandos.rs`, `servico.rs`, `erro.rs`, `capabilities/default.json`.
9. `app/src/api/cliente.ts` e `consultas.ts`.
10. `docs/checkpoints/` (um por etapa, o mais recente por último).

## 29. Comandos de desenvolvimento e teste

```
# interface e app
cd app && npm ci && npm run tauri dev

# verificações (o mesmo que a CI)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                        # também regenera app/src/api/tipos
git diff --exit-code -- app/src/api/tipos     # tipos gerados versionados
cd app && npm run typecheck && npm test && npm run build

# ponta a ponta no app real
cd app && npx tauri build --debug --no-bundle
testes/e2e/executar.sh target/debug/sied-desktop
#   Linux: xvfb-run, openbox, wmctrl, tauri-driver (cargo install), WebKitWebDriver

# extração sintética para testar à mão
python3 testes/gerador/extracao_sintetica.py --whatsapp --android --volume <pasta>
```

Variáveis: `SIED_PASTA_DADOS`, `SIED_PASTA_CASOS`, `SIED_LOG`.

## 30. Estratégia de Git

- Repositório **único**: `Doctor-Henrique-Oliveira/SIED-Sistema-Integrado-de-Evid-ncias-Digitais`.
  Não criar outro.
- Uma branch por etapa, PR em rascunho, commits coerentes por assunto em pt-BR.
- `main` só recebe **merge commit** de PR, com CI Linux e Windows verde e sem conflito. Desde
  30/09/2026 (17:27) o usuário autorizou a construção contínua: cada fase concluída é mesclada
  assim que a CI fica verde, sem pedir aval a cada merge; a CI da `main` é conferida depois.
- Branches mescladas **não são apagadas** sem confirmação do usuário.
- Cada etapa termina com um checkpoint em `docs/checkpoints/AAAA-MM-DD-<etapa>.md`
  (IMPLEMENTADO, TESTES, ARQUIVOS, DECISÕES, PROBLEMAS, COMO EXECUTAR, COMO TESTAR, LIMITAÇÕES,
  PRÓXIMA FASE) e atualização de `docs/CONTINUIDADE.md`.
- Nunca versionar `.env`, credenciais, bancos, extrações, relatórios reais ou dumps (o
  `.gitignore` cobre; fixtures sintéticas em `testes/fixtures/**` são a exceção explícita).

Histórico de PRs e commits: `docs/CONTINUIDADE.md` §2 e §3.

## 31. Commit final da `main`

A auditoria entrou pelo PR #11 (branch `claude/auditoria-hardening-7p20bx`, sobre `5b68c8d`),
mesclado em 29/09/2026 no commit **`9f87f89`** (merge commit; último commit da branch:
`e1916af`). É o último commit com código: a CI da `main` rodou nele e ficou verde (Linux e
Windows; instaladores pulados na `main`, como previsto em "Só tags"). Os instaladores Windows
(NSIS e MSI) foram gerados e ficaram verdes no PR, sobre o mesmo código (`e1916af`).

Depois dele só entrou o registro deste hash nos documentos (PR #12, só `.md`, que por
configuração não dispara a CI). Para conferir o topo atual:

```
git fetch origin && git log --oneline -1 origin/main
```

## 32. NÃO FAÇA ISSO

- **Não** abra, grave, renomeie ou "conserte" nada na pasta da extração. Nem para teste.
- **Não** abra banco de evidência em modo gravável, nem com `PRAGMA` que escreva (WAL,
  `journal_mode`, `VACUUM`, `wal_checkpoint`). Leia a cópia conferida.
- **Não** enfraqueça a política de somente leitura, os gatilhos de somente inclusão ou a trava
  do caso para "facilitar" um teste ou uma funcionalidade.
- **Não** mude a regra do `hash_registro`, os campos de `NovoArtefato` ou as features do
  `serde_json` sem uma migração de hash planejada e aprovada: todo caso existente divergiria.
- **Não** confie na coluna `hash_registro` para dizer que algo confere: recalcule.
- **Não** edite uma migração já publicada; crie a próxima. Não apague as cópias `.bak`.
- **Não** grave dados derivados (título de conversa, nome de contato) nos artefatos.
- **Não** implemente o importador Avilla sem a árvore real; **não** invente caminhos, bancos,
  nomes de arquivo, estrutura ou formatos.
- **Não** interprete o que não tem certeza: data implausível, coordenada inválida, tipo
  desconhecido ficam como valor bruto com aviso. "Não sei" é melhor que inventar.
- **Não** coloque texto do usuário ou da evidência como marcação Typst; só como dado JSON.
- **Não** dê ao frontend permissões de fs, shell, http ou processo, nem afrouxe a CSP.
- **Não** registre conteúdo de evidência em logs técnicos.
- **Não** versione dados reais, bancos, extrações, relatórios, `.env`, tokens ou dumps. Fixtures
  sempre sintéticas e identificadas.
- **Não** remova, pule, desative ou enfraqueça testes para a CI ficar verde; não mude gatilhos da
  CI sem plano aprovado.
- **Não** faça merge na `main` sem CI Linux e Windows verde nem fora da autorização vigente do
  usuário (§30); não apague
  branches sem confirmação; não reescreva histórico da `main`.
- **Não** crie outro repositório.
- **Não** reative a depuração das dependências no perfil `dev` nem crie binários de teste que
  liguem o Typst à toa: o disco do executor Linux da CI é o limite (§20). Medição nova com
  Typst vai como teste `#[ignore]` num arquivo de teste que já exista, se possível.
- **Não** mostre dado fictício na interface: o que não tem backend fica oculto, desativado ou
  marcado como indisponível.
- **Não** use `lower()` do SQLite para comparar texto do usuário (só ASCII); use `sied_normal`.
- **Não** remova o `INDEXED BY idx_artefato_conversa` das consultas de conversa sem medir com
  200 mil registros (`tests/desempenho.rs`).
