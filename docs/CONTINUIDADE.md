# SIED — Documento de continuidade

Estado em 28/09/2026, ao mesclar o PR #3 na `main`. Este documento permite retomar o projeto do
zero: o que existe, onde está, por que foi feito assim e o que vem a seguir. Os detalhes de cada
etapa ficam nos checkpoints em `docs/checkpoints/` e na preparação em
`docs/PREPARACAO-AVILLA-WHATSAPP.md`.

Documentos de referência: `docs/VISAO.md` (visão integral do projeto, permanente) e
`docs/ARQUITETURA.md` (arquitetura aprovada com ajustes).

---

## 1. Arquitetura atual

SIED é uma plataforma de **análise** forense de extrações de celulares, local e 100% offline. Não
faz aquisição: importa o resultado de ferramentas (Avilla, depois IPED, ADB, ALEAPP, iLEAPP, pasta
genérica), normaliza para um modelo comum com origem completa de cada registro e, na etapa
seguinte, gera relatórios PDF.

| Camada | Tecnologia | Onde |
|---|---|---|
| Interface | React + TypeScript + Vite + Tailwind + TanStack Query | `app/src` |
| Aplicativo desktop | Tauri 2 | `app/src-tauri` (biblioteca `sied_desktop_lib`) |
| Domínio | Rust | `crates/sied-dominio` (caso, dispositivo, artefato, ids, validação) |
| Armazenamento | Rust + SQLite (rusqlite, FTS5) | `crates/sied-armazenamento` |
| Ingestão | Rust | `crates/sied-ingestao` (fontes, importadores, parsers, pipeline) |
| Relatórios | Typst (previsto) | ainda não iniciado |

Plataforma alvo: Windows 11 (compatível com Windows 10). A CI gera instaladores NSIS e MSI.

### Princípios em vigor

1. **Originais nunca alterados.** A pasta da extração é lida por uma fonte somente leitura; nada é
   gravado nela. Os bancos da evidência nunca são abertos em modo gravável: o parser trabalha numa
   cópia de trabalho cujo SHA-256 é conferido contra o do inventário.
2. **Importador × parser.** O importador entende a *ferramenta* (Avilla, IPED...) e diz quais
   arquivos têm quais papéis; o parser entende o *aplicativo* (SMS, WhatsApp...) e lê os bancos.
   Um parser vale para qualquer importador que encontre os arquivos dele.
3. **Origem completa.** Cada artefato guarda: caminho original, ferramenta, arquivo e hash, banco,
   tabela, id do registro, valores brutos, parser e versão (`referencia_origem`).
4. **Somente inclusão.** Inventário, artefatos, origens e auditoria têm gatilhos que impedem
   `UPDATE`/`DELETE`.
5. **Derivados reconstruíveis.** Conversas e índice de busca são mantidos por gatilho a partir dos
   artefatos e podem ser refeitos a qualquer momento.
6. **Contratos abertos.** Os contratos de importador e parser não impedem parsers externos futuros
   (Python, Java), sem introduzi-los agora.

### Bancos

- **Global** (pequeno): configurações e catálogo de casos (`configuracao`, `caso_catalogo`).
- **Um SQLite autocontido por caso** (`<caso>.sied.db`): pode ser arquivado, copiado ou
  transferido sozinho. Cada banco tem versão de esquema e migrações próprias.

---

## 2. Branches e PRs

| Branch | Commit | PR | Situação |
|---|---|---|---|
| `main` | ver §3 | — | recebe tudo por merge commit |
| `fase1-fundacao` | `50c2363` | #1 | mesclado (`b193d99`) |
| `fase1-importacao` | `2e4ad5d` | #2 | mesclado (`005615c`) |
| `fase1-preparacao` | ver §3 | #3 | mesclado nesta etapa |
| `fase1-whatsapp` | — | — | só local, sem commits próprios; será recriada a partir da nova `main` |

As branches mescladas **não foram apagadas**. Podem ser removidas depois que a `main` for
confirmada íntegra e com CI verde, com aval do usuário.

Regras de trabalho: push e PR em rascunho da branch de trabalho são autorizados; **merge só com aval
explícito** e CI completa verde (Linux e Windows).

## 3. Commits principais

| Commit | Descrição |
|---|---|
| `d767370` | Commit inicial do repositório |
| `944f65b` | Fundação: workspace, bancos, domínio, contratos e app Tauri |
| `50c2363` | Correção do clippy apontada pela CI |
| `05709b5` | Importação: fonte pasta, inventário SHA-256, manifesto, parser de SMS, gravação no caso |
| `1e59378` | Importação em segundo plano, telas de importação e extração, teste ponta a ponta |
| `c464b62`, `2e4ad5d` | Checkpoint da importação e situação da CI Windows |
| `b193d99` | Merge do PR #1 na `main` |
| `005615c` | Merge do PR #2 na `main` |
| `53eeaed` | Conversas reconstruídas, busca global e roteiro do Avilla/WhatsApp |
| (este) | Documento de continuidade e reforço do `.gitignore` |
| (merge) | Merge do PR #3 na `main` |

---

## 4. Funcionalidades concluídas

**Casos**
- Criar, abrir, listar e reabrir casos; catálogo global; banco autocontido por caso.
- Fuso horário do caso usado em todas as datas exibidas.
- Trilha de auditoria somente inclusão.

**Importação** (fonte: pasta)
- Seleção da pasta, detecção do importador com confiança e motivos, plano com papéis.
- Inventário completo com SHA-256, categoria e tamanho; manifesto da importação com hashes.
- Cópia de trabalho verificada; parsers leem só a cópia.
- Execução em segundo plano com progresso, cancelamento e registro de interrupção.
- Importador genérico (pasta); importador Avilla registrado **só como contrato**.
- Parser de SMS Android (`mmssms.db`), incluindo registros que só existem no `-wal`.

**Extração**
- Tela da extração com resumo, inventário, registros e visão técnica da origem de cada registro.

**Conversas** (migração 3)
- Conversas reconstruídas por app e id da conversa, com participantes, contagens e período.
- Visualização estilo chat: lados enviado/recebido, separadores por dia, hora no fuso do caso.
- Alternância para "Detalhes técnicos" com o painel de origem do registro selecionado.

**Busca global** (migração 3)
- FTS5 em registros e nomes de arquivo, sem diferenciar acentos e maiúsculas, com prefixo.
- Texto digitado protegido contra sintaxe do FTS (cada palavra vira frase com prefixo).
- Telefone em qualquer formato (com ou sem DDI/DDD e separadores).
- Filtros por extração e período (backend também aceita app e tipo).
- "Abrir na conversa" a partir do resultado, com o registro destacado e a origem aberta.

## 5. Testes e resultados

| Conjunto | Quantidade | Resultado |
|---|---|---|
| Rust (`cargo test --workspace`) | 110 | todos passam |
| `cargo clippy -D warnings` e `cargo fmt --check` | — | limpos |
| Interface (`vitest`) | 15 | todos passam |
| Typecheck TypeScript | — | limpo |
| Tipos gerados (ts-rs) atualizados | — | conferido pela CI |
| Ponta a ponta no app real (`testes/e2e`) | 1 roteiro, vários passos | passa; pasta da extração intacta (hash e datas antes/depois) |
| CI Linux (testes) e Windows (build + instaladores NSIS/MSI) | — | verde nos PRs #1, #2, #3 e na `main` |

O que os testes cobrem, em resumo: migrações (incluindo caso antigo que migra com cópia de
segurança `.antes-vN.bak`), gatilhos de somente inclusão, inventário e hash, manifesto, detecção
e plano, parser de SMS (inclusive WAL), conversas, busca (texto, prefixo sem acento, telefone,
nomes de arquivo, sintaxe maliciosa, filtros) e a interface de chat e busca.

## 6. Limitações conhecidas

- **Avilla sem importador**: falta a árvore de uma extração real (ver §10).
- **Contêineres** (`.tar`, `.zip`, `.ab`) são inventariados com hash, mas não abertos.
- Só **SMS** é interpretado (sem MMS). WhatsApp, contatos e chamadas vêm a seguir.
- Hash calculado numa única thread. Sem retomada de importação interrompida (importa-se de novo; a
  interrompida fica registrada).
- Sem exclusão de extração (somente inclusão, de propósito).
- Sem verificação do "hash declarado" do laudo de aquisição.
- Conversa carrega 500 mensagens por página; vindo da busca, o registro só é destacado se estiver
  na página carregada.
- Busca ordenada só por relevância; a tela expõe filtros de extração e período, não de app e tipo.
- Sem evidências selecionadas nem relatório PDF (fecham o critério de pronto da fase 1).

## 7. Decisões técnicas

1. Stack Tauri + React/TS + Rust + SQLite/FTS5 + Typst (aprovada).
2. Banco global pequeno + banco autocontido por caso, cada um com migrações versionadas.
3. Migrações conferidas por hash; antes de migrar um caso existente, cópia de segurança com
   `VACUUM INTO` em `<banco>.antes-v<N>.bak`.
4. Importador (ferramenta) separado de parser (app).
5. Trabalho só em cópia verificada; nada gravado na pasta da evidência.
6. Conversa **derivada**: o artefato guarda `conversa_id_externo` (id da conversa no app) e a tabela
   `conversa` é agregação mantida por gatilho, sem vínculo fixo gravado na mensagem.
7. Busca: cada palavra vira `"palavra"*`; tokenizador `unicode61 remove_diacritics 2`; telefone
   (só dígitos e separadores, 6+ dígitos) buscado por `instr` nos dígitos de remetente e
   destinatários.
8. Tipos TypeScript gerados a partir do Rust (ts-rs) em `app/src/api/tipos`; a CI falha se estiverem
   desatualizados.
9. Avilla: nada presumido; só contrato e fixtures sintéticas identificadas.
10. Nunca versionar `.env`, credenciais ou dados reais (ver §11).

## 8. Estrutura do banco e migrações

Arquivos em `crates/sied-armazenamento/migracoes/`, registrados em `src/migracoes.rs`.

**Global**
| Versão | Nome | Objetos |
|---|---|---|
| 1 | inicial | `configuracao`, `caso_catalogo` |

**Caso**
| Versão | Nome | Objetos |
|---|---|---|
| 1 | inicial | `caso` (linha única), `dispositivo`, `auditoria` (somente inclusão) |
| 2 | importacao | `extracao`, `arquivo` (inventário), `execucao_parser`, `aviso_parser`, `artefato`, `referencia_origem`; gatilhos de somente inclusão em arquivo, artefato e origem |
| 3 | conversas_busca | `conversa`, `conversa_participante`, FTS5 `busca_artefato` e `busca_arquivo`; gatilhos `artefato_indexa_busca`, `artefato_agrega_conversa`, `arquivo_indexa_busca`; preenchimento dos dados já importados |

Um caso criado em versão anterior é migrado ao abrir, depois da cópia de segurança.

## 9. Estado da importação, conversas e busca

- **Importação**: completa para pasta genérica com SMS Android, do início (seleção) ao fim
  (artefatos com origem), em segundo plano e com auditoria.
- **Conversas**: funcionam para qualquer parser que preencha `conversa_id_externo`; hoje, SMS.
  Participantes aparecem como identificadores brutos (nomes virão com o WhatsApp e contatos).
- **Busca**: funciona em tudo que for importado, sem trabalho extra por parser.

## 10. O que ainda depende do Avilla

Só o **importador Avilla** (`crates/sied-ingestao/src/importadores/avilla.rs`, hoje responde
`NaoImplementado`). Precisamos do usuário:

1. Árvore de pastas de uma extração de teste: `tree /F /A > arvore.txt` (só nomes, sem conteúdo).
2. Versão do Avilla usada.
3. Opções marcadas na extração (backup ADB, downgrade do WhatsApp, cópia de pastas etc.).
4. Nome do arquivo de hash ou log da aquisição, se houver.
5. Se possível, uma segunda árvore de outra extração.

Com isso: fixture sintética com os mesmos nomes, `detectar` e `planejar`, testes de detecção,
plano e pasta intacta, e importação completa no app real. O roteiro está em
`docs/PREPARACAO-AVILLA-WHATSAPP.md` §2.

Não dependem do Avilla: parser WhatsApp, contatos, chamadas, evidências e relatório.

## 11. Proteção do repositório

O `.gitignore` (seção SIED) impede o envio de:
- bancos (`*.db`, `*.sqlite*`, `*.sied.db*` e arquivos `-wal`/`-shm`);
- pastas de casos, extrações e relatórios; `.ab`, `.tar`, `.avilla`, `.pdf`;
- contêineres e imagens forenses (`.zip`, `.7z`, `.rar`, `.ufdr`, `.e01`, `.dd`, `.img` e afins);
- backups cifrados do WhatsApp (`*.crypt*`);
- credenciais (`.env*` exceto `.env.example`, `*.pem`, `*.key`, `*.pfx`, `*.p12`, `secrets.*`).

Exceção explícita: `testes/fixtures/**` (fixtures sintéticas, identificadas como tais).

## 12. Como executar e testar

```
cd app && npm install && npm run tauri dev      # app em desenvolvimento
cargo test --workspace                           # testes Rust (também regenera app/src/api/tipos)
cargo clippy --workspace --all-targets -- -D warnings
cd app && npm run typecheck && npm test          # tipos e testes da interface
cd app && npx tauri build --debug --no-bundle    # binário para o ponta a ponta
testes/e2e/executar.sh target/debug/sied-desktop # ponta a ponta no app real
```

Detalhes no `README.md`.

## 13. Próximo passo recomendado

**Parser WhatsApp Android** numa branch própria criada a partir da nova `main` (não depende do
Avilla). Plano já definido:

- Fixtures SQL sintéticas em `testes/fixtures/whatsapp` para os dois esquemas do `msgstore.db`
  (antigo: `messages`; novo: `message`, `chat`, `jid` e auxiliares) e para o `wa.db`.
- Detecção pelo esquema presente, nunca pela versão do app; valores brutos sempre preservados.
- Mensagens (texto, mídia, localização, contato, sistema, resposta, encaminhamento, apagada),
  estados de envio, grupos e participantes, contatos do `wa.db`.
- Anexos ligados ao inventário (caminho exato; depois sufixo único com conferência de tamanho).
- Conversas ganham título, tipo (individual/grupo) e nomes dos participantes.
- Checkpoint com testes, limitações e amostras reconstruídas antes de qualquer merge.

Depois: importador Avilla (quando a árvore chegar), evidências selecionadas e relatório PDF.
