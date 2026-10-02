# SIED — Documento de continuidade

Estado em 30/09/2026: depois da auditoria, a `main` recebeu o saneamento dos documentos (PR #13) e o
**design system global com a casca do workspace** (PR #14, merge `132a8d5`) e a **agenda de
contatos e registro de chamadas do Android** (PR #15, merge `9dd1947`; checkpoint
`docs/checkpoints/2026-09-30-contatos-chamadas-android.md`). Desde a regra de construção contínua
do usuário (30/09, 17:27), cada fase vai para a `main` quando a CI Linux e Windows está verde. Em
seguida vieram os **MMS do Android**, a **linha do tempo unificada** (Consultas e Localizações) e
o **console de importação em tempo real** (§14.1) e o **importador IPED** (lista de itens em CSV),
no PR #16 (merge `b6da666`; checkpoints `2026-09-30-mms-android.md`,
`2026-09-30-linha-do-tempo.md`, `2026-09-30-console-importacao.md` e
`2026-09-30-importador-iped.md`). Depois, o **importador ADB**: leitura de backup `.ab` e de `.tar`
sem extrair para o disco (PR #17, checkpoint `2026-09-30-importador-adb.md`), e o **importador
ALEAPP/iLEAPP** (exportações TSV do relatório; checkpoint `2026-09-30-importador-leapp.md`).

Estado em 29/09/2026: a fase 1 fecha o ciclo **importar → analisar → selecionar → relatar**. Na
`main` estão a interface para volume real (PR #9, merge `24791ff`), as **evidências selecionadas
com o relatório PDF** em Typst (PR #10, merge `5b68c8d`) e a **auditoria pré-produção com
hardening** (PR #11, merge `9f87f89`, último commit com código; CI da `main` verde nele, Linux e
Windows). Quem for continuar o projeto
deve começar por `docs/HANDOFF-CODEX.md`. O fechamento da auditoria está em
`docs/checkpoints/2026-09-29-auditoria-hardening.md`; o das evidências e relatório, em
`docs/checkpoints/2026-09-29-fase1-evidencias-relatorio.md`; o do volume real, em
`docs/checkpoints/2026-09-28-fase1-visual-volume.md`; o do WhatsApp, em
`docs/checkpoints/2026-09-28-fase1-whatsapp.md`; a interface do redesign, no §15. Este documento permite retomar o projeto do
zero: o que existe, onde está, por que foi feito assim e o que vem a seguir. Os detalhes de cada
etapa ficam nos checkpoints em `docs/checkpoints/` e na preparação em
`docs/PREPARACAO-AVILLA-WHATSAPP.md`.

Documentos de referência: `docs/VISAO.md` (visão integral do projeto, permanente) e
`docs/ARQUITETURA.md` (arquitetura aprovada com ajustes).

---

## 1. Arquitetura atual

SIED é uma plataforma de **análise** forense de extrações de celulares, local e 100% offline. Não
faz aquisição: importa o resultado de ferramentas (Avilla, depois IPED, ADB, ALEAPP, iLEAPP, pasta
genérica), normaliza para um modelo comum com origem completa de cada registro e gera relatórios
PDF das evidências selecionadas.

| Camada | Tecnologia | Onde |
|---|---|---|
| Interface | React + TypeScript + Vite + Tailwind v4 + TanStack Query; tema escuro próprio, ícones embutidos (§15) | `app/src` |
| Aplicativo desktop | Tauri 2 | `app/src-tauri` (biblioteca `sied_desktop_lib`) |
| Domínio | Rust | `crates/sied-dominio` (caso, dispositivo, artefato, ids, validação) |
| Armazenamento | Rust + SQLite (rusqlite, FTS5) | `crates/sied-armazenamento` |
| Ingestão | Rust | `crates/sied-ingestao` (fontes, importadores, parsers, pipeline) |
| Mídia | Rust (`image`) | `crates/sied-midia` (miniaturas e imagens do relatório, SHA-256 conferido) |
| Relatórios | Typst 0.15 embutido (`typst`, `typst-pdf`, fontes embutidas) | `crates/sied-relatorio` (montagem, conferência, modelo `modelo.typ`) |

Plataforma alvo: Windows 11 (compatível com Windows 10). A CI gera instaladores NSIS e MSI em tag de
release, execução manual ou PR que muda o build (opção "Só tags", PR #4).

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
| `fase1-preparacao` | `5bad447` | #3 | mesclado (`1f9a0b4`) |
| `ci-otimizacao` | `ee2738c` | #4 | mesclado (`65eae8d`) |
| `roadmap-console-importacao` | `df83890` | #5 | mesclado (`5258374`) |
| `fase1-redesign` | `9bbad7a` | #6 | mesclado (`5e8e734`) |
| `fase1-redesign-registro` | — | #7 | mesclado (`85273d1`) |
| `fase1-whatsapp` | `baee700` | #8 | mesclado (`dc402cf`) |
| `fase1-visual-volume` | `181f2c5` | #9 | mesclado (`24791ff`) |
| `fase1-evidencias-relatorio` | — | #10 | mesclado (`5b68c8d`) |
| `claude/auditoria-hardening-7p20bx` | `e1916af` | #11 | mesclado (`9f87f89`) |
| `claude/auditoria-hardening-7p20bx` (reiniciada da `main`) | — | #12 | só documentos: registra o commit final |
| — | `951365e` | #13 | mesclado (`6f5d5c1`): saneamento dos documentos do handoff |
| `claude/project-thread-dycfuw` | `24e44fe` | #14 | mesclado (`132a8d5`): design system global e casca do workspace |
| `claude/contatos-chamadas-android-z7m48s` | `c9762ec` | #15 | mesclado (`9dd1947`): agenda e chamadas do Android |
| `claude/contatos-chamadas-android-z7m48s` (reiniciada da `main`) | `0c19bca` | #16 | mesclado (`b6da666`): MMS do Android, linha do tempo unificada, console de importação e importador IPED |
| `claude/contatos-chamadas-android-z7m48s` (reiniciada da `main`) | `6dfadb6` | #17 | mesclado (`bcdcd0c`): importador ADB (backup `.ab` e `.tar`) |
| `claude/contatos-chamadas-android-z7m48s` (reiniciada da `main`) | `0b641e3` | #18 | mesclado (`eb7bdfe`): importador ALEAPP/iLEAPP |
| `claude/contatos-chamadas-android-z7m48s` (reiniciada da `main`) | `2012ac7` | #19 | mesclado (`e92b830`): conversas grandes pelo índice (migração 6) |
| `claude/contatos-chamadas-android-z7m48s` (reiniciada da `main`) | — | novo PR | conferência da pasta de relatórios (F9) e destaque da busca em pedaços (F8) |

As branches mescladas **não foram apagadas**. Podem ser removidas depois que a `main` for
confirmada íntegra e com CI verde, com aval do usuário.

Regras de trabalho: push e PR em rascunho da branch de trabalho são autorizados. Desde 30/09/2026
(17:27) vale a construção contínua pedida pelo usuário: cada fase concluída é mesclada na `main`
com a CI completa verde (Linux e Windows) e sem conflito, e a CI da `main` é conferida depois.
Branches não são apagadas.

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
| `5bad447` | Documento de continuidade e reforço do `.gitignore` |
| `1f9a0b4` | Merge do PR #3 na `main` |
| `65eae8d` | Merge do PR #4: CI sem execuções duplicadas, cache, instaladores só em tag/manual/PR de build |
| `5258374` | Merge do PR #5: roadmap do console de importação (§14) |
| `0317a3e` | Backend: resumo do caso e identificadores das conversas para o painel |
| `ecc02ed` | Backend: categoria "midia" no inventário para a galeria de mídias |
| `64f6d27` | Interface: tema escuro, casca com 12 módulos, Dashboard do caso e telas novas |
| `af482fb` | Testes da interface do redesign |
| `9bbad7a` | Documentação do redesign (§15) e README |
| `5e8e734` | Merge do PR #6 na `main`: redesign visual e de navegação |
| `85273d1` | Merge do PR #7 na `main`: checkpoint do redesign |
| `314b258` | `fase1-whatsapp`: parser WhatsApp Android, pareamento `msgstore.db`/`wa.db`, anexos ligados ao inventário, migração 4 |
| `1ebe173` | `fase1-whatsapp`: conversas do WhatsApp na interface e gerador `--whatsapp` |
| `dc402cf` | Merge do PR #8 na `main`: leitor WhatsApp Android |
| `65a7af2` | `fase1-visual-volume`: miniaturas (crate `sied-midia`), filtros de conversa no banco, posição e localizar, progresso detalhado |
| `f440877` | `fase1-visual-volume`: interface para volume real e gerador `--volume` |
| `181f2c5` | `fase1-visual-volume`: ajustes das telas e checkpoint |
| `24791ff` | Merge do PR #9 na `main`: interface para volume real |
| `64b3387` | `fase1-evidencias-relatorio`: crate `sied-relatorio` com Typst embutido |
| (seguinte) | `fase1-evidencias-relatorio`: evidências (migração 5), relatório PDF, telas, testes e checkpoint |
| `5b68c8d` | Merge do PR #10 na `main`: evidências e relatório PDF |
| `97f59f2` | Auditoria: relatório recalcula o `hash_registro` a partir do conteúdo gravado (crítico) |
| `f24df36` | Auditoria: trava exclusiva do caso (`caso.sied.lock`) |
| `ac63ae0` | Auditoria: tipo do banco conferido antes de escrever; cópia de migração sem colisão; erros SQLite em pt-BR |
| `ec0d609` | Auditoria: pânico capturado nas fronteiras, imagem grande sem estourar memória, emojis e invisíveis legíveis no PDF |
| `506af8a` | Auditoria: comandos Tauri fora da thread da janela |
| `dac7b39` | Auditoria: filtros de texto sem diferença de maiúsculas e acentos (`sied_normal`) |
| `dab6953` | Auditoria: caminhos da extração com unidade, `.` e nomes só de pontos recusados em qualquer posição |
| `06d960e`, `5636248` | Auditoria: WAL ilegível gera aviso; trecho em importação recusado |
| `cd1af15` | Auditoria: verificação da trilha confere as âncoras dos relatórios |
| `37afac8`, `72685f1`, `5e9e196` | Auditoria: temporário do PDF, nomes reservados do Windows, texto da trilha no PDF |
| `985e20c`, `6ca06ef` | Auditoria: aviso de pasta na nuvem ou em compartilhamento de rede |
| `1a49ab0`, `004ac16` | Auditoria: índice certo nas consultas da conversa; medições com volume |
| `eb89f48`, `72a4672`, `2e48156` | Auditoria: `.gitignore`, coordenadas e REAL infinito, datas absurdas |
| `6324102`, `e1916af` | Auditoria: HANDOFF-CODEX, continuidade e checkpoint; dependências sem depuração nos builds de teste (CI Linux sem disco) |
| `9f87f89` | Merge do PR #11 na `main`: auditoria pré-produção e hardening |

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
- Progresso real na tela (PR #9): as quatro etapas (contagem, inventário e SHA-256, bancos
  conhecidos, cópia verificada e leitura), barra da etapa atual, tempo decorrido, taxa e tempo
  restante do inventário medidos desde o início da etapa, bytes da cópia de trabalho sendo
  conferida. Resumo "etapa n/4 · %" no cabeçalho.
- Importador genérico (pasta); importador Avilla registrado **só como contrato**.
- Parser de SMS Android (`mmssms.db`), incluindo registros que só existem no `-wal`.
- Parser **WhatsApp Android** (PR #8, na `main`):
  - `msgstore.db` nos esquemas novo (`message`) e antigo (`messages`), detectados pelo conteúdo;
  - `wa.db` (contatos);
  - mensagens de texto, mídia, localização, contato, sistema, resposta, encaminhada e apagada;
  - situação de envio, grupos e membros;
  - backups `.crypt*` só inventariados.
- Anexos que o app cita são ligados ao arquivo do inventário (caminho exato ou sufixo único com
  tamanho conferido). O motivo de não ligar fica gravado.

**Extração**
- Tela da extração com resumo, inventário, registros e visão técnica da origem de cada registro.

**Conversas** (migração 3)
- Conversas reconstruídas por app e id da conversa, com participantes, contagens e período.
- Visualização estilo chat: lados enviado/recebido, separadores por dia, hora no fuso do caso.
- Alternância para "Detalhes técnicos" com o painel de origem do registro selecionado.
- WhatsApp:
  - título (assunto do grupo ou nome do contato), tipo e nomes dos participantes, vindos dos
    registros do próprio app na mesma extração;
  - mensagem de sistema centralizada;
  - anexo com situação no inventário, coordenadas, trecho respondido e apagada.

**Interface** (redesign, §15)
- Casca fixa com barra lateral de 12 módulos, cabeçalho com o caso aberto, busca global (Ctrl+K),
  importação em curso e estação/usuário.
- Dashboard do caso com cabeçalho do procedimento, atalhos, 8 indicadores reais, dispositivos,
  extrações, alertas e auditoria.
- Conversas em três colunas; busca com resultados agrupados; Mídias, Arquivos e Contatos com dados
  reais; Consultas com a linha do tempo do caso e Localizações com a lista cronológica dos
  registros com coordenadas (mapa e consultas salvas ainda indisponíveis).

**Volume real** (PR #9)
- Miniaturas das imagens na galeria de Mídias, no detalhe do arquivo e nos anexos das conversas.
  O arquivo é lido da pasta da extração em modo somente leitura, o SHA-256 é conferido com o
  inventário antes de decodificar, e a miniatura (JPEG, lado maior 320 px) fica em
  `<pasta do caso>/miniaturas/`. Sem miniatura, a tela mostra o ícone e o motivo (origem
  indisponível, arquivo difere do inventário, imagem ilegível, formato sem suporte).
- Listas virtualizadas: conversas (acima de 100), mensagens (acima de 500) e grade de mídias
  (acima de 100). Só as linhas visíveis existem na tela; as páginas vêm do banco conforme a
  rolagem. Botões "Primeira" e "Última" na conversa longa.
- Filtros de conversa no banco, valendo para o caso inteiro: texto (título, participante, nome no
  app, telefone com ou sem máscara), app (com contagem), individuais ou grupos, com anexo.
- Localizar em conversa longa pergunta ao banco (total de achados e posição de cada um) e rola até
  o achado; vindo da busca, a conversa rola até o registro mesmo fora da primeira página.

**Evidências e relatório** (PR #10, migração 5)
- Marcação nas telas de análise: mensagem ou trecho (início e fim, na ordem da conversa, até 2.000
  mensagens) pela barra "Evidência" do chat; arquivo pelo detalhe em Mídias e Arquivos. Selo
  "E-n" no chat e no detalhe.
- Hash gravado na marcação: `hash_registro`, SHA-256 do inventário ou SHA-256 das linhas
  `id:hash_registro` do trecho. Alvo e hash imutáveis por gatilho; repetição recusada.
- Módulo Relatórios: lista com reordenar, editar textos (título, categoria, descrição contextual,
  observação), ver na origem e remover com motivo (remoção lógica). Tudo auditado.
- Relatório PDF (Typst embutido, modelo `padrao-v1`): 10 seções escolhidas na tela; capa,
  procedimento, dispositivos, aquisição e importação, metodologia, evidências (chat, trecho,
  arquivo com a imagem conferida), observações, conclusão, apêndice técnico com a origem completa e
  os valores brutos, integridade e cadeia de custódia.
- Na geração, cada evidência é conferida de novo ("conferida", "divergente" com quadro de ATENÇÃO,
  "indisponível" sem a pasta da extração). Trilha de auditoria quebrada impede a geração.
- Saída `<caso>/relatorios/AAAA-MM-DD_HHMMSS_relatorio.pdf` + `.pdf.sha256`, nunca sobrescrita,
  registrada na tabela `relatorio` (somente inclusão) com o SHA-256 e a posição da trilha.

**Agenda de contatos e registro de chamadas do Android** (branch `claude/contatos-chamadas-android-z7m48s`)
- Parser `android_contatos` (`contacts2.db`): um registro por contato bruto (`raw_contacts`), com a
  conta que sincroniza (Google, WhatsApp, aparelho), telefones (tipo, rótulo, número normalizado
  pelo aparelho), e-mails, nome estruturado, apelido, empresa, observação, endereço, site e outros
  tipos de dado (inclusive de apps) com o mimetype. Cada linha de `data`, a conta e o contato
  agrupado entram como origens adicionais. Apagados (`deleted=1`) ficam com estado "apagado".
- Parser `android_chamadas` (tabela `calls` de `calllog.db`, ou de `contacts2.db` nas versões
  antigas): tipo (recebida, efetuada, perdida, correio de voz, rejeitada, bloqueada, atendida em
  outro aparelho, desconhecido), duração, apresentação, nome gravado junto da chamada, recursos
  (vídeo, Wi-Fi, HD...), conta/chip, número normalizado e demais colunas nos metadados. Códigos
  antigos no lugar do número (`-1`…`-4`) não viram telefone.
- O importador genérico reconhece `contacts2.db` e `calllog.db`; `contacts2.db` sem `calllog.db`
  na mesma pasta também passa pelo leitor de chamadas (tabela ausente vira aviso, execução
  "parcial").
- Vínculo com a agenda **calculado na consulta, nunca gravado**: mesma extração e igualdade exata
  dos dígitos do número (ou do número normalizado pelo aparelho). Aparece nas chamadas e nos
  identificadores das conversas. Não é identificação de quem usava a linha.
- Tela Contatos com três abas: "Agenda e contatos" (filtro por origem e por nome/telefone,
  detalhe com telefones e "Ver chamadas"), "Chamadas" (resumo, filtro por tipo e por número ou
  nome, detalhe) e "Identificadores das conversas" (a tabela anterior, com a coluna "Na agenda").
  Contato e chamada podem virar evidência e têm painel técnico com a origem até a linha da tabela.
  A barra lateral continua com 12 módulos (Chamadas é uma aba de Contatos).
- Busca por telefone também acha contatos pelos telefones da agenda; resultado de contato ou
  chamada abre direto em Contatos.
- Relatório PDF com campos próprios para chamada (tipo, número, duração, nome gravado, recursos,
  conta) e contato (telefones com tipo e rótulo, e-mails, conta, empresa).
- Sem migração nova (índices existentes bastam).

**MMS do Android** (branch `claude/contatos-chamadas-android-z7m48s`, reiniciada da `main`
depois do PR #15)
- Parser `android_mms` (tabelas `pdu`, `addr` e `part` do `mmssms.db`): uma mensagem por MMS,
  na mesma conversa dos SMS (`thread_id`), com remetente e destinatários (o próprio aparelho,
  `insert-address-token`, nunca vira telefone), texto das partes, assunto, primeira mídia como
  anexo ligado ao arquivo de `app_parts` no inventário, todas as partes nos metadados e cada
  linha de `addr`/`part` como origem adicional. Data em segundos. MMS não baixado aparece como
  tal. Entra em conversas, busca, evidências e relatório como qualquer mensagem.

**Linha do tempo unificada** (mesma branch, depois do MMS)
- Todos os registros com data do caso (todas as extrações e apps) em ordem cronológica, pelo
  índice `idx_artefato_tempo`; filtros de tipo, app, extração, período e coordenadas; ordem
  crescente ou decrescente; paginação de 100.
- Resumo por tipo e por app, primeiro e último registro, e contagem dos registros **sem data
  utilizável** (ficam fora da lista, nunca com data inventada). O cabeçalho de conversa não entra.
- Tela em Consultas (aba "Linha do tempo"; "Consultas salvas" continua indisponível), agrupada por
  dia no fuso do caso; cada linha abre a conversa com o registro destacado, a chamada em Contatos
  ou a extração. Localizações usa a mesma lista só com coordenadas (latitude e longitude como
  lidas); o mapa offline continua indisponível. Sem migração.

**Console de importação em tempo real** (§14.1; mesma branch)
- O pipeline anuncia cada passo real (contagem, SHA-256 e inventário, bancos localizados,
  manifesto, plano, cópia conferida, parser iniciado e concluído, gravação, avisos, erros, fim)
  numa fila em memória consultada pela sequência e no `log-importacao.jsonl` com a mesma mensagem.
- Console sob a barra de progresso (horário no fuso do caso, decorrido, nível, recolhível, "ao
  vivo"), fechado no resultado e reaberto na aba Processamento da extração a partir do log.

**Importador IPED** (mesma branch; formato conferido no código-fonte público do IPED)
- Reconhece a pasta de um caso do IPED (`iped/IPEDConfig.txt`, `iped/index`, `iped/conf`) e a
  lista de itens na raiz (`FileList.csv`, "Lista de Arquivos.csv" e as outras 4 traduções).
- Parser `iped_lista_arquivos`: um registro "arquivo" por linha (nome, tamanho, MD5/SHA1 do IPED,
  caminho na evidência, marcadores, categorias, apagado, recuperado, datas com fuso inequívoco),
  a linha inteira em `valores_brutos`; o atalho `=HIPERLINK(...)` liga o item ao arquivo exportado
  no inventário. Fora de `iped/`, os bancos com nome padrão seguem para os leitores Android e
  WhatsApp. O índice Lucene não é lido. Sem migração.

**Importador ADB e contêineres** (mesma branch; formato público do Android, AOSP)
- A fonte lê por dentro `.tar` e backup `.ab` (sem cifra, comprimido ou não) sem extrair nada
  para o disco: os itens de dentro entram no inventário com SHA-256 e caminho
  `backup.ab!/apps/<pacote>/db/...`, e os parsers recebem a cópia verificada desses itens.
- Importador `adb`: reconhece o `.ab` pelo cabeçalho, descreve cada backup e cada `_manifest`
  (pacote, versão do app, API do Android) nas observações e manda os bancos de nome padrão para
  os mesmos leitores da pasta genérica. Backup cifrado (`AES-256`) é inventariado com hash, mas
  não aberto. A pasta genérica também passa a abrir `.tar` e `.ab`. Sem migração.

**Importador ALEAPP/iLEAPP** (mesma branch; formato conferido no código-fonte público dos dois)
- Reconhece a pasta do relatório pelo `_lava_data.lava` (nome e versão da ferramenta), `_HTML`,
  `_lava_artifacts.db`, `_Timeline` e a pasta `_TSV Exports`.
- Um parser por ferramenta (`aleapp_tsv`, `ileapp_tsv`, `xleapp_tsv`): cada linha de cada
  `_TSV Exports/<artefato>.tsv` vira um registro "outro" com o nome do artefato, um resumo
  "coluna: valor" (busca e listas), a linha inteira em `valores_brutos`, a data (primeira
  coluna, ou a primeira com "date"/"time" no nome) convertida só com fuso explícito, e
  latitude/longitude pelas colunas `Latitude`/`Longitude`. O banco LAVA, o HTML e as cópias da
  evidência em `data/` e `media/` entram só no inventário. Sem migração.

**Busca global** (migração 3)
- FTS5 em registros e nomes de arquivo, sem diferenciar acentos e maiúsculas, com prefixo.
- Texto digitado protegido contra sintaxe do FTS (cada palavra vira frase com prefixo).
- Telefone em qualquer formato (com ou sem DDI/DDD e separadores).
- Filtros por extração e período (backend também aceita app e tipo).
- "Abrir na conversa" a partir do resultado, com o registro destacado e a origem aberta.

**Conferência e reconstrução dos dados derivados** (01/10/2026, sem migração)
- Dashboard → Auditoria → "Conversas e índices de busca": **Conferir** compara conversas,
  participantes e os índices de busca com o que os registros importados produzem (e o índice FTS
  com o texto guardado). Com diferença, lista por tabela e oferece **Reconstruir**, que recalcula
  tudo numa transação registrada na auditoria; os registros de origem não são tocados.

## 5. Testes e resultados

| Conjunto | Quantidade | Resultado |
|---|---|---|
| Rust (`cargo test --workspace`, soma de todos os `test result`) | 284 + 3 medições ignoradas com o aviso de unidade de rede mapeada (282 com a conferência e reconstrução dos derivados; 276 com a conferência da pasta de relatórios e o destaque em pedaços; 267 com as conversas grandes; 265 com o ALEAPP/iLEAPP; 257 com o ADB; 248 com o PR #16; 222 com o PR #15; 201 após a auditoria) | todos passam |
| `cargo clippy -D warnings` e `cargo fmt --check` | — | limpos |
| Interface (`vitest`) | 62 com o aviso de unidade de rede mapeada (59 com a conferência dos derivados; 57 com a conferência da pasta de relatórios; 56 com a linha do tempo e o console; 50 com o PR #15; 43 após a auditoria) | todos passam |
| Build da interface (`npm run build`) e do app (`tauri build --debug --no-bundle`) | — | ok |
| Typecheck TypeScript | — | limpo |
| Tipos gerados (ts-rs) atualizados | — | conferido pela CI |
| Ponta a ponta no app real (`testes/e2e`) | 1 roteiro, vários passos | passa; pasta da extração intacta (hash e datas antes/depois) |
| CI Linux e Windows | — | verde em todos os PRs e na `main` (instaladores conforme "Só tags") |

O que os testes cobrem, em resumo: migrações (incluindo caso antigo que migra com cópia de
segurança `.antes-vN.bak`), gatilhos de somente inclusão, inventário e hash, manifesto, detecção
e plano, parser de SMS (inclusive WAL), conversas, busca (texto, prefixo sem acento, telefone,
nomes de arquivo, sintaxe maliciosa, filtros), resumo do caso e identificadores, filtro de mídias,
WhatsApp (dois esquemas, `wa.db`, WAL, avisos, anexos, títulos e nomes, migração 3→4), e
miniaturas (redução, cache, hash divergente, origem ausente, formatos, pedidos simultâneos),
filtros de conversa, posição e localizar sobre 1.200 mensagens sintéticas, progresso final, e
a interface: listas virtualizadas, filtros enviados ao banco, localizar em conversa longa,
miniaturas na galeria e nos anexos, etapas do progresso, evidências (hash por tipo, recusas,
gatilhos, marcação no chat e no arquivo, lista, geração) e o relatório (PDF, `.sha256`, registro,
auditoria, conteúdo, divergente, indisponível, trilha adulterada), chat, busca, importação, formulário de dispositivo, barra lateral (12 módulos, módulos
desativados sem caso), indicador sem dado, seção recolhível, Contatos e Mídias. O ponta a ponta
continua o mesmo roteiro, sem nenhuma verificação removida.

A auditoria (PR #11) acrescentou: hash recalculado com dados hostis e cinco adulterações; trava
do caso; banco vazio, truncado, de outro tipo e `.bak` já existente; parser em pânico;
imagem-bomba; orientação EXIF; textos hostis no PDF (conferidos glifo a glifo); filtros sem
acento; caminhos perigosos; WAL ilegível; trecho em importação; trilha truncada e reescrita;
temporário do PDF; nomes reservados; pasta na nuvem ou na rede; datas e coordenadas absurdas;
REAL infinito. Medições com volume em `tests/desempenho.rs` (ignoradas; ver o checkpoint).

## 6. Limitações conhecidas

- **Avilla sem importador**: falta a árvore de uma extração real (ver §10).
- **Contêineres**: `.tar` e `.ab` sem cifra são lidos por dentro; `.zip` e outros continuam só
  inventariados com hash. Backup `.ab` cifrado não é aberto (exige a senha). Ler itens fora de
  ordem num contêiner grande comprimido é lento (a leitura recomeça do início).
- **ADB**: conferido só com backup sintético; `apps/<pacote>/f/` e `sp/` só inventariados.
- **ALEAPP/iLEAPP**: só as exportações TSV (relatório gerado sem TSV não tem registros); cada
  linha fica como "outro registro", sem virar mensagem, contato ou chamada do SIED; datas sem
  fuso ficam só no original. Conferido só com relatórios sintéticos.
- **IPED**: só a lista de itens exportada em CSV é lida (o índice Lucene e os bancos internos
  não); conferido só com um caso sintético montado pelo código-fonte público. Fuso ambíguo
  (ex.: `CET`) fica só no valor original, com aviso.
- São interpretados **SMS e MMS**, **WhatsApp Android**, **agenda** e **registro de chamadas**
  do Android. MMS, agenda e chamadas foram conferidos só com fixtures sintéticas montadas pela
  documentação pública (`Telephony.Mms`, `ContactsContract`, `CallLog.Calls`), não com um banco
  de aparelho de teste. Fotos dos contatos, `deleted_contacts`, chamadas de apps (WhatsApp etc.)
  e RCS não são lidos. Contatos repetidos em contas diferentes não são juntados. No MMS, só a
  primeira mídia vira anexo; as demais ficam listadas nas partes da mensagem.
- WhatsApp:
  - conferido só com bancos sintéticos, com nomes da referência pública do app;
  - `.crypt*` não decifrados;
  - `@lid`/`jid_map`, reações, enquetes, edições e chamadas do app não lidos;
  - dono da conta não identificado;
  - detalhes no checkpoint da fase.
- Hash calculado numa única thread. Sem retomada de importação interrompida (importa-se de novo; a
  interrompida fica registrada).
- Sem exclusão de extração (somente inclusão, de propósito).
- Sem verificação do "hash declarado" do laudo de aquisição.
- Localizar dentro da conversa longa conta todas as ocorrências, mas navega só pelas 5.000
  primeiras (ignora maiúsculas e acentos desde a auditoria).
- Busca ordenada só por relevância; a tela expõe filtros de extração, app e período (tipo só no
  backend).
- Miniaturas só de imagens JPEG, PNG, GIF, WebP, BMP e TIFF até 256 MB. HEIC/HEIF, vídeos e
  áudios aparecem sem miniatura e sem reprodução. Mídias e Arquivos mostram uma extração por vez.
- A lista de Arquivos (tabela) continua paginada de 100 em 100; a de Contatos também.
- A taxa e o tempo restante aparecem só no inventário (etapa medida em bytes). Na leitura dos
  bancos, a barra avança por banco concluído; o tempo de cada parser não é estimado.
- Localizações só em lista (sem mapa offline); Consultas só com a linha do tempo (sem consultas
  salvas); linha do tempo paginada de 100 e com período por dia inteiro; Configurações somente
  leitura; tema único (escuro). O ícone do instalador continua o anterior.
- A imagem de referência citada no pedido do redesign não chegou ao projeto; o redesign seguiu a
  especificação escrita.
- Relatório: no máximo 60 figuras; a geração segura a trava do caso (outros comandos esperam); sem
  botão "Abrir PDF" (a tela mostra o caminho); usuário é o do sistema operacional; um único modelo;
  sem assinatura digital do PDF; erro de imagem ilegível vem do decodificador em inglês.
- Da auditoria (baixo risco): queda do processo entre gravar e registrar o PDF deixa um PDF órfão
  (B7; desde 01/10/2026 a tela de Relatórios o lista como "sem registro no caso"; B6, destaque da
  busca, resolvido); conversa de 100 mil
  mensagens: página 7–14 ms e localizar ~0,14 s desde a migração 6 (eram 79–185 ms e ~0,5 s, B8); copie o caso **fechado** (a trava impede copiar com ele
  aberto no Windows, e o `-wal` ficaria para trás) (B9).
- Caso em compartilhamento de rede ou pasta sincronizada (OneDrive e afins): WAL e trava não são
  confiáveis fora do disco local. A tela avisa, sem bloquear: no Novo caso e nos Alertas do
  Dashboard do caso aberto. Desde 01/10/2026 a unidade mapeada (`Z:`) também é reconhecida,
  pelo sistema operacional (`app/src-tauri/src/disco.rs`).

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
11. WhatsApp: esquema detectado pelo conteúdo. O registro `conversa` do app fica separado das
    mensagens (sem `conversa_id_externo`).
12. Título, tipo e nomes são resolvidos **na consulta** a partir dos registros da mesma extração,
    nunca gravados na conversa derivada.
13. Anexo só é ligado quando o arquivo é único e o tamanho confere. O motivo fica em
    `artefato.anexo_vinculo`.
14. Miniatura é dado **derivado**, fora do banco: arquivo em `<caso>/miniaturas/<sha[..2]>/<sha>-v1.jpg`,
    gerado sob demanda depois de conferir o SHA-256 do arquivo na extração, reconstruível a qualquer
    momento. Chega à tela como `data:` (a política de conteúdo do app só aceita imagens locais).
    No máximo 4 gerações ao mesmo tempo.
15. Listas longas: a primeira página vem como antes; acima do tamanho da página, a lista é
    virtualizada (`@tanstack/react-virtual`) e as demais páginas são buscadas conforme a rolagem.
    Filtros, posição e localizar são consultas SQL sobre o caso inteiro.
16. Progresso: o backend informa início da importação e da etapa, fase da tarefa e bytes da cópia
    conferida; a tela só calcula frações e taxas a partir desses números, sem simulação.
17. Evidência guarda o hash do alvo no momento da seleção; o relatório mostra esse hash e o
    conferido na geração. Trecho identificado pelas pontas e pelo hash das linhas
    `id:hash_registro` na ordem da conversa. Remoção lógica com motivo.
18. Relatório somente inclusão, com SHA-256 e posição da trilha de auditoria; trilha quebrada
    impede a geração. Typst e fontes embutidos; textos entram como dados JSON, nunca como
    marcação. Datas no fuso do caso com nome e deslocamento.
19. (Auditoria) Na geração do relatório o `hash_registro` é **recalculado** a partir das colunas
    e origens gravadas (`extracoes::conferir_registro`); a coluna sozinha não prova nada. Para o
    recálculo ser exato: `serde_json` com `float_roundtrip` e `NovoArtefato::normalizar()` antes
    do hash (−0.0 → 0.0; `app` vazio → ausente).
20. (Auditoria) Trava exclusiva por arquivo `caso.sied.lock` enquanto o caso está aberto.
21. (Auditoria) Datas só entre 1970 e 31/12/9999 (`DataHora::ms_plausivel`); coordenadas só
    dentro de ±90/±180. Fora disso, o valor fica só no original.
22. (Auditoria) Consultas da conversa com `a.app IS nullif(?, '')` e
    `INDEXED BY idx_artefato_conversa`; comparação de texto do usuário com `sied_normal()`
    (só em consultas).
23. (Auditoria) A verificação da trilha confere as âncoras gravadas por cada relatório.
24. (Auditoria) Comandos Tauri assíncronos (`spawn_blocking`); `catch_unwind` no parser, no
    pipeline, na decodificação de imagem e no Typst.
25. (Auditoria) Dependências sem informação de depuração nos builds de desenvolvimento e teste
    (`[profile.dev.package."*"] debug = false`): os binários de teste com Typst caíram de ~530
    para ~170 MB e a CI Linux deixou de ficar sem disco ao ligá-los.

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
| 4 | whatsapp | coluna `artefato.anexo_vinculo`; índices `idx_arquivo_nome` e `idx_artefato_id_externo` |
| 5 | evidencias_relatorio | `evidencia` (alvo e hash imutáveis, sem `DELETE`, removida não muda), `relatorio` (somente inclusão) |
| 6 | ordem_conversa | índice `idx_artefato_conversa_ordem (extracao_id, app, conversa_id_externo, data_hora_ms)` no lugar de `idx_artefato_conversa`; nenhum dado muda |

Um caso criado em versão anterior é migrado ao abrir, depois da cópia de segurança.

## 9. Estado da importação, conversas e busca

- **Importação**: completa para pasta genérica com SMS Android e WhatsApp Android (`msgstore.db` + `wa.db`). Vai do início (seleção) ao fim (artefatos com
  origem), em segundo plano e com auditoria.
- **Conversas**: funcionam para qualquer parser que preencha `conversa_id_externo`. Título, tipo
  e nomes aparecem quando o app grava registros `conversa` e `contato` (hoje, o WhatsApp); sem
  eles, os participantes aparecem como identificadores brutos (SMS).
- **Busca**: funciona em tudo que for importado, sem trabalho extra por parser.
- **Evidências e relatório**: valem para qualquer registro, trecho de conversa ou arquivo do
  inventário, de qualquer parser.

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

Não dependem do Avilla: agenda de contatos e chamadas (evidências e relatório já estão prontos). O parser do
WhatsApp já está pronto (PR #8) e vale para qualquer importador.

## 11. Proteção do repositório

O `.gitignore` (seção SIED) impede o envio de:
- bancos (`*.db`, `*.sqlite*`, `*.sied.db*` e arquivos `-wal`/`-shm`);
- pastas de casos, extrações e relatórios; `.ab`, `.tar`, `.avilla`, `.pdf`;
- contêineres e imagens forenses (`.zip`, `.7z`, `.rar`, `.ufdr`, `.e01`, `.dd`, `.img` e afins);
- backups cifrados do WhatsApp (`*.crypt*`);
- credenciais (`.env*` exceto `.env.example`, `*.pem`, `*.key`, `*.pfx`, `*.p12`, `secrets.*`);
- arquivos de trabalho de um caso (`*.bak`, `*.parcial`, `*.sied.lock`, `log-importacao.jsonl`,
  `manifesto.sha256`).

Exceção explícita: `testes/fixtures/**` (fixtures sintéticas, identificadas como tais).

## 12. Como executar e testar

```
cd app && npm install && npm run tauri dev      # app em desenvolvimento
cargo test --workspace                           # testes Rust (também regenera app/src/api/tipos)
cargo clippy --workspace --all-targets -- -D warnings
cd app && npm run typecheck && npm test          # tipos e testes da interface
cd app && npx tauri build --debug --no-bundle    # binário para o ponta a ponta
testes/e2e/executar.sh target/debug/sied-desktop # ponta a ponta no app real (Linux: xvfb-run, openbox, wmctrl, tauri-driver, WebKitWebDriver)
python3 testes/gerador/extracao_sintetica.py --whatsapp --volume <pasta>  # extração sintética com volume
```

Detalhes no `README.md`.

## 13. Próximo passo recomendado

**Fase 1: ciclo completo na `main`, auditado.** Volume real (PR #9) e evidências com relatório PDF
(PR #10) estão descritos no §4 e nos checkpoints `2026-09-28-fase1-visual-volume.md` e
`2026-09-29-fase1-evidencias-relatorio.md`. A auditoria pré-produção (PR #11) está em
`2026-09-29-auditoria-hardening.md`; a passagem para outro agente, em `docs/HANDOFF-CODEX.md`.
Depois vieram, em construção contínua, a agenda e as chamadas (PR #15), os MMS, a linha do
tempo unificada, o console de importação, o importador IPED (PR #16), o importador ADB (PR #17), o importador
ALEAPP/iLEAPP (PR #18, checkpoints de 30/09/2026) e as conversas grandes servidas pelo índice
(F1 da dívida técnica, checkpoint `2026-10-01-conversas-grandes.md`), a conferência da pasta de
relatórios e o destaque da busca em pedaços (F9 e F8, checkpoint `2026-10-01-relatorios-busca.md`).

A seguir (ordem da construção contínua):
1. importador Avilla (quando a árvore chegar, §10);
2. conferência do WhatsApp, SMS/MMS, agenda, chamadas, da lista do IPED, do backup ADB e dos
   relatórios ALEAPP/iLEAPP com dados de teste
   (não de investigação);
3. demais itens de dívida técnica medidos (HANDOFF §25; F1, F8 e F9 feitos, F4 medido sem ganho).

---

## 14. Roadmap: itens futuros registrados

Itens pedidos pelo usuário para depois. **Não implementar antes do momento indicado.**

### 14.1 Console de importação em tempo real

**Feito em 30/09/2026** (checkpoint `2026-09-30-console-importacao.md`). O registro original
segue abaixo como referência.

**Quando:** depois que o fluxo real de importação estiver estabilizado (importador Avilla e
parsers principais funcionando com extrações reais). Registrado em 28/09/2026.

**O que é:** um painel com aparência de terminal técnico/forense dentro do SIED que complementa
(ou substitui) a barra de progresso simples durante importações de extrações, relatórios e
resultados do Avilla/IPED. Exemplo do que deve aparecer:

```
[10:42:11] Iniciando importação...
[10:42:12] Fonte selecionada: avilla_export.zip
[10:42:14] Calculando SHA-256...
[10:42:21] Hash confirmado.
[10:42:22] Inventariando arquivos...
[10:42:30] Banco do WhatsApp localizado.
[10:42:31] Executando parser de mensagens...
[10:42:39] 12.483 mensagens processadas.
[10:42:41] 1.284 mídias catalogadas.
[10:42:45] Indexando busca global...
[10:42:51] Importação concluída com sucesso.
```

**Requisitos:**
- Mostrar etapas, progresso, quantidade de itens processados, avisos, erros e tempo decorrido.
- **Eventos reais do backend, não animação.** Cada linha corresponde a algo que aconteceu:
  leitura de arquivos, cálculo e conferência de hashes, validação da cópia de trabalho, detecção e
  execução de importadores e parsers, gravação de artefatos, indexação da busca.
- Barra ou percentual de progresso resumido acima do console.
- Console recolhível/expansível.
- Aparência coerente com o restante do SIED.

**Ponto de partida no código atual:** a importação já roda em segundo plano e expõe um
*retrato* do progresso (`pipeline::Progresso`: etapa, arquivos, bytes e tarefas feitos/total),
que a interface consulta a cada 500 ms. O console exige um **registro de eventos** com horário
(etapa iniciada/concluída, arquivo, hash conferido, parser iniciado com contagem, aviso, erro),
guardado em ordem durante a execução e entregue à interface de forma incremental. O protocolo de
parsers externos (`protocolo.rs`) já tem mensagens de `progresso`, `aviso`, `fim` e `falha`, que
entram nesse mesmo registro. Vale avaliar gravar os eventos também na auditoria ou no manifesto
da importação, para que o console de uma importação passada possa ser reaberto.

---

### 14.2 Motor de Descoberta Forense Multimodal Local (MDFML)

**FUTURO — não implementado.** Registrado pelo responsável pelo produto em 02/10/2026 como
capacidade estratégica de diferenciação do SIED. Nome e sigla provisórios. Este registro é
contexto de produto e planejamento; não autoriza implementação, instalação de modelos ou
alteração da arquitetura vigente. Especificação: [ROADMAP-MDFML.md](ROADMAP-MDFML.md).

**Objetivo:** reduzir o tempo para encontrar, revisar, correlacionar e documentar artefatos
relevantes em grandes extrações, com pesquisa local sobre mensagens, imagens, prints, OCR,
PDFs, documentos, arquivos, metadados, indicadores e localizações. Preservar a pesquisa
textual/FTS existente; resultados assistidos serão candidatos para revisão humana, nunca
conclusões periciais automáticas.

**Diretrizes:** offline-first; original intocado; derivados separados; explicação do mecanismo
e da origem de cada resultado; modelos/vocabulários/configurações versionados; decisão humana
separada da inferência; processamento incremental que não bloqueie o caso. Integração externa,
se futuramente aprovada, será opcional, institucionalmente autorizada, identificada,
explicitamente solicitada e auditável; não integra o núcleo local nem modifica as permissões
atuais do frontend.

**Faseamento proposto, ainda sem autorização de execução:** AI-0 (pesquisa tradicional,
vocabulários, árvore de pesquisa, emojis e indicadores); AI-1 (OCR, PDFs/documentos e entidades);
AI-2 (embeddings locais, semântica e ranking híbrido); AI-3 (visão, regiões e near-duplicates);
AI-4 (multimodal); AI-5 (correlação, timeline assistida e grafo **experimental**); AI-6
(assistente local explicável); AI-7 (feedback supervisionado, datasets autorizados e avaliação).
Cada fase exige desenho técnico, modelo de ameaças, testes/benchmarks, avaliação de erros,
privacidade, auditoria e checkpoint antes de qualquer merge.

O detalhe documenta limites e tensões com o estado atual. Não substitui a ordem de continuidade
do §13, não desbloqueia o importador Avilla e não altera o trabalho visual/premium em andamento.

---

## 15. Interface: estrutura visual (redesign de 28/09/2026)

Evolução visual e de navegação sobre a base existente: backend, Tauri, bancos, migrações, auditoria,
contratos e fluxos continuam os mesmos. Nenhum número é inventado; sem dado, o componente mostra
"—" ou "Ainda não disponível".

**Tema** (`app/src/estilos.css`, tokens no `@theme` do Tailwind v4): fundo azul quase preto em
níveis (`fundo`, `lateral`, `sup`, `sup-2`, `sup-3`), bordas discretas, dourado (`ouro`) só para
marca, item ativo e ação principal; verde `ok`, azul `info`, laranja `atencao`, vermelho `erro` só
para erro. Fontes do sistema (Segoe UI / Cascadia Mono), nada baixado. Foco visível dourado em tudo,
barras de rolagem finas, `prefers-reduced-motion` respeitado.

**Design system global (30/09/2026)**: tudo em `app/src/estilos.css`, só na interface.
- Tokens: cores navy/preto + dourado (`ouro`, `ouro-claro`, `ouro-escuro`, `ouro-tinta` = texto sobre
  dourado; `texto-4` = desabilitado; `*-claro` = texto dentro de avisos), escala tipográfica fixa
  (`text-3xs` 10 px a `text-2xl` 22 px, `text-sm` = 13 px, o corpo das tabelas), cantos pequenos
  (`rounded-md` 4 px, `rounded-xl` 6 px; `rounded-2xl` 12 px só nas bolhas de conversa) e
  `--tracking-sobre`.
- Receitas (`@utility`): `sobrelinha` (rótulo técnico em caixa alta), `botao` + `botao-p` +
  `botao-primario|secundario|fantasma|perigo`, `entrada` (foco, hover, desabilitado e
  `aria-invalid`), `painel` e `tecla`. `classeBotao()` (`Botao.tsx`) monta as classes de botão para
  `Botao`, `LinkBotao` e links soltos. Nenhuma cor hexadecimal nas páginas.
- Hierarquia por tipografia e linhas finas: `Cabecalho` sem caixa de ícone (ícone dourado junto ao
  título, linha abaixo), `FaixaIndicadores` (células separadas por 1 px, sem um cartão por número),
  `Aviso` com filete lateral, `Selo` retangular de 20 px.
- Casca: barra lateral de 232 px com filete dourado no item ativo; cabeçalho de 48 px com trilho
  "caso › módulo"; a área útil volta ao topo a cada troca de rota.

**Casca** (`rotas/Layout.tsx`): barra lateral fixa (`componentes/BarraLateral.tsx`) com marca,
caso aberto, 12 módulos com ícone em 4 grupos, status offline e versão; cabeçalho
(`componentes/BarraSuperior.tsx`) com o caso (trocar/fechar), busca global do caso (Ctrl+K),
indicador de importação em curso com percentual real, selo "Local" e usuário @ estação. Botão
"Pular para o conteúdo" para teclado.

**Componentes reutilizáveis**
| Componente | Onde | Papel |
|---|---|---|
| `Icone` | `componentes/Icone.tsx` | ícones de traço embutidos (SVG, `currentColor`) |
| `Cabecalho`, `Cartao`, `Definicoes`, `Aviso`, `Carregando`, `Vazio`, `Selo`, `Hash`, `HashCurto`, `BarraProgresso`, `Paginacao` | `componentes/Estrutura.tsx` | blocos básicos (Selo = StatusBadge; Vazio = EmptyState) |
| `Indicador` | idem | StatCard; `valor` nulo vira "—" |
| `SecaoRecolhivel` | idem | blocos técnicos recolhíveis (`aria-expanded`) |
| `Indisponivel` | idem | módulo ainda não disponível, com a estrutura prevista |
| `Abas`, `Filtros`, `LinkBotao`, `tabela`, `entrada` | idem | abas, pílulas de filtro, links-botão, classes de tabela e campo |
| `Botao`, `Campo`, `AreaTexto`, `Selecao` | `Botao.tsx`, `Campo.tsx` | mesmas props de antes, tema novo |
| `SecaoDispositivos`, `SecaoExtracoes`, `SecaoAtividades` | `SecoesCaso.tsx` | cartões no Dashboard, tabela nas páginas próprias |
| `InventarioArquivos`, `SeletorExtracao` | `Arquivos.tsx` | tabela (Arquivos) ou grade (Mídias) com painel de detalhes |
| `PainelTecnico` | `PainelTecnico.tsx` | Origem (arquivo, SHA-256, tabela, registro, consulta, valores brutos), Data e hora (timestamp e formato originais), Processamento (parser, importador, estado, confiança, manifesto, hash do registro), Metadados |
| `ConsoleImportacao` | `Importacao.tsx` | espaço do console (§14.1): hoje só o estado real que o backend já informa |

**Rotas** (`main.tsx`, hash router)
| Módulo | Rota | Situação |
|---|---|---|
| Dashboard | `/casos/:id` | pronto: cabeçalho do procedimento, atalhos, indicadores (`resumo_caso`), dispositivos, extrações, alertas, auditoria |
| Casos | `/` e `/casos/novo` | pronto (lista com filtro) |
| Dispositivos | `/casos/:id/dispositivos` | pronto (tabela completa, cadastro) |
| Extrações | `/casos/:id/extracoes` e `/extracoes/:x` | pronto; a extração tem blocos técnicos (dispositivo, ferramenta, contagens, tempo, origem, integridade) e abas |
| Conversas | `/casos/:id/conversas[/:c]` | pronto: lista (filtro, app, grupo, anexos), chat (separador fixo por dia, localizar na conversa, destaque da busca), contexto ou detalhes técnicos |
| Busca | `/casos/:id/busca?q=` | pronta: campo grande, filtros de extração, app e período, resultados agrupados |
| Mídias | `/casos/:id/midias` | pronta com o inventário (filtro `midia`), grade virtualizada e miniaturas de imagens conferidas por SHA-256; vídeos e áudios ainda sem miniatura ou reprodução |
| Arquivos | `/casos/:id/arquivos` | pronta: busca, tipo, extensão, caminho, tamanho, SHA-256, data, parser |
| Contatos | `/casos/:id/contatos` | identificadores das conversas (`listar_identificadores`), sem deduzir identidade |
| Localizações | `/casos/:id/localizacoes` | lista cronológica dos registros com coordenadas (linha do tempo filtrada); mapa offline indisponível |
| Consultas | `/casos/:id/consultas` | aba "Linha do tempo" (todos os registros com data, filtros, resumo, sem data à parte); "Consultas salvas" indisponível |
| Relatórios | `/casos/:id/relatorios` | lista de evidências (ordem, textos, origem, remoção), gerar PDF e relatórios gerados (PR #10) |
| Configurações | `/configuracoes` | somente leitura (versão, usuário, estação, pastas) |

**Backend novo desta etapa** (somente leitura, sem migração): `resumo_caso`,
`listar_identificadores` e a categoria agrupadora `midia` em `listar_arquivos`.

**Resoluções conferidas**: 1366×768 e 1920×1080 (capturas do app real sobre o caso sintético do
ponta a ponta).

### 15.1 Premium Experience (01/10/2026)

Em validação na branch `feat/premium-experience-splash-console`: o aplicativo passa por splash
SIED e welcome sem sidebar; criação/abertura bem-sucedida mostra um console de inicialização com
eventos reais antes de liberar o workspace. A janela é contida na entrada e libera
resize/maximização no workspace usando somente permissões específicas de janela. Dashboard e
Conversas receberam ajustes de hierarquia sem mudar dados ou regras. Decisões, testes e
limitações: `docs/checkpoints/2026-10-01-premium-experience.md`.
