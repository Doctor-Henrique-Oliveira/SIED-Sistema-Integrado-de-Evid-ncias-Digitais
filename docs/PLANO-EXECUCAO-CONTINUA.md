# SIED — Plano oficial de execução contínua

Atualizado em 09/10/2026. Este documento transforma a visão, o README e o ROADMAP MDFML em uma fila executável. Ele não substitui os invariantes de `VISAO.md`, `ARQUITETURA.md` e `ROADMAP-MDFML.md`.

## Ponteiro atual

**Próxima etapa: E12 — Desempenho, retomada e preparação de release.**

O código incorporado à `main` até `87b10e8` é a linha de base. AI-0–AI-7 possuem fundações e interfaces, porém vários motores continuam conservadores ou ausentes. As etapas abaixo aprofundam essas fundações sem duplicá-las.

## Como manter o plano

Estados permitidos:

- `CONCLUÍDA`: fluxo real, integrado, testado e utilizável;
- `EM ANDAMENTO`: existe uma branch ativa com implementação incompleta;
- `PENDENTE`: ainda não iniciado de forma funcional;
- `BLOQUEADA`: depende de insumo indispensável indisponível, com evidência do bloqueio.

Uma etapa só muda para `CONCLUÍDA` quando seus critérios forem atendidos no código. Documento, interface isolada, contrato, mock ou fixture não bastam.

## Linha de base já disponível

- casos, dispositivos, extrações, inventário e manifesto SHA-256;
- importadores genérico, ADB, IPED e ALEAPP/iLEAPP, com limites documentados;
- SMS, MMS, WhatsApp Android, contatos e chamadas cobertos nos formatos atuais;
- conversas, mídias, arquivos, contatos, localizações e timeline básica;
- FTS, pesquisa literal, vocabulários versionados e indicadores locais;
- leitura conservadora de UTF-8/PDF nativo e entidades sintáticas;
- dHash e similaridade visual estrutural limitada;
- ranking integrado, contexto temporal, grafo limitado, gramática de consulta e datasets supervisionados;
- evidências selecionadas, relatório PDF e Auditoria em módulo próprio;
- shell premium e preparação do caso antes da liberação do workspace.

## Ordem obrigatória

### E01 — OCR local e documentos pesquisáveis

**Status: EM ANDAMENTO**

Branch de trabalho: `feat/e01-ocr-local-documentos`. O motor local Windows,
persistência append-only, pesquisa, salto da ocorrência encontrada para sua
região, mapa de bounding boxes, paginação real das regiões e cancelamento
cooperativo visível durante leitura/indexação estão integrados. Ao sair da
tela, a operação documental ativa também recebe pedido de cancelamento.
Fixtures Windows de imagem e PDF sintéticos exigem o
motor real e falham explicitamente sem idioma OCR; a etapa só será marcada como
concluída após sua execução, contratos Rust, testes multiplataforma e validação
visual do fluxo imagem/PDF.

Entregar OCR totalmente local para imagens e PDFs digitalizados, com página, região/bounding box, mecanismo, versão, confiança técnica, hash da origem, cache versionado e busca navegável. Ampliar formatos documentais somente quando houver parser seguro e origem conferível. O processamento deve ser cancelável e incremental, sem alterar a evidência.

**Concluída quando:** imagem e PDF sintéticos geram texto OCR pesquisável; cada ocorrência abre página/região e origem; reprocessamento seguro reutiliza cache; erros e ausência de motor ficam visíveis; testes Rust, contratos TypeScript, testes de interface e build passam.

### E02 — Transcrição local de áudio e vídeo

**Status: EM ANDAMENTO (núcleo persistente, pesquisa, playback por timestamp e adaptador offline configurável implementados; engine de fala continua dependente de instalação local)**

Adicionar motor offline, extração segura de áudio, segmentos com timestamps, idioma/motor/versão, cancelamento, cache e pesquisa dentro da transcrição. Integrar resultado à origem, mídia, busca, timeline, evidências e relatório.

**Concluída quando:** mídia sintética é transcrita localmente; clicar num trecho abre o instante correspondente; busca e relatório preservam timestamps e proveniência; nenhuma rede é usada. O SIED agora aceita sidecars SRT/VTT conferidos e um comando local opt-in (`SIED_TRANSCRITOR_LOCAL`) que recebe a mídia regular e devolve SRT/VTT em stdout, com cancelamento e limite de saída. A instalação/configuração de Whisper/Vosk e a validação Rust/MSVC continuam externas; contêineres permanecem sem execução de processo.

### E03 — Embeddings locais e busca híbrida real

**Status: EM ANDAMENTO (índice local e busca híbrida funcional; modelo neural local ainda pendente)**

Foi adicionado o índice reconstruível `embedding_derivado` e a aba de pesquisa híbrida local. O motor atual `hash-embedding-local/tfidf-hash-v1` é determinístico, offline e explicitamente lexical: combina FTS5 com similaridade vetorial, pagina, permite cancelamento cooperativo e mostra a contribuição de cada mecanismo. Ele ainda não substitui um modelo neural local autorizado; OCR e demais derivados entram quando já existem no banco.

**Concluída quando:** mensagens, documentos, OCR e transcrições sintéticas semanticamente relacionadas são recuperadas; índices podem ser reconstruídos; ranking é explicável e comparado à busca tradicional.

### E04 — Visão computacional e regiões

**Status: EM ANDAMENTO (dHash/near-duplicates funcionais e adaptador local de candidatos com regiões implementados; modelo visual continua opt-in)**

Evoluir o dHash existente com classificação/detecção local, regiões e bounding boxes, mecanismo e versão. O adaptador `SIED_VISAO_LOCAL` recebe uma imagem regular e devolve JSON limitado com rótulo, confiança técnica e caixa normalizada; a interface mostra cada item como candidato e preserva o mecanismo/versão. Duplicata exata, near-duplicate e candidato de objeto continuam conceitos separados. Todo resultado permanece candidato para revisão humana.

**Concluída quando:** imagens sintéticas produzem candidatos e regiões navegáveis, pesquisa por semelhantes funciona e a interface nunca apresenta classificação como conclusão pericial. A instalação/configuração de um modelo offline concreto permanece externa e não há rede ou provider automático.

### E05 — Motor multimodal unificado

**Status: IMPLEMENTADA LOCALMENTE (E05)**

Integrado o motor `pesquisar_multimodal` ao cliente Tauri e à aba Pesquisa profunda. A execução reutiliza os índices locais de mensagens, documentos/OCR, transcrições com timestamps, entidades documentais, embeddings, descritores visuais e metadados. Os candidatos são deduplicados pela origem, ranqueados por RRF com pesos explícitos, paginados e filtráveis por mecanismo após a consulta. Cada sinal preserva motivo, motor/versão e hash derivado quando aplicável; a interface mantém navegação para o artefato original e destaca que ranking não é confiança pericial.

Limites deliberados: a visão continua dependente de descritores/provider local opt-in da E04; OCR, transcrição e embeddings só retornam dados previamente indexados; não há envio para rede nem fabricação de sinais ausentes.

**Concluída quando:** uma consulta sintética retorna artefatos de modalidades diferentes numa lista coerente e cada resultado explica separadamente todos os sinais que contribuíram.

### E06 — Timeline inteligente, mapa offline e grafo investigativo

**Status: IMPLEMENTADA LOCALMENTE (E06)**

Evoluída a linha do tempo e o contexto relacional existentes sem duplicar os índices. A tela de Localizações agora oferece um mapa esquemático offline, com projeção local limitada a 200 pontos, pontos navegáveis para a origem e aviso explícito de que proximidade espacial não prova trajeto ou causalidade. O contexto relacional ganhou grafo SVG interativo, expansão sob demanda por registro, limite de 30 eventos/60 referências e seleção de arestas com explicação de mecanismo, origem e classe direta/sugerida.

Limites deliberados: o mapa não usa tiles nem rede; o grafo continua experimental e carrega apenas a vizinhança solicitada, evitando materializar o caso inteiro.

**Concluída quando:** eventos e relações abrem seus artefatos originais; mapas funcionam sem rede; grandes casos não carregam o grafo inteiro; toda aresta explica sua origem.

### E07 — Assistente local explicável

**Status: IMPLEMENTADA LOCALMENTE (E07)**

Adicionada a aba Assistente local à Busca. A consulta natural é interpretada por regras determinísticas locais, executa recuperação seletiva no motor multimodal, mostra os mecanismos escolhidos antes da resposta e apresenta suporte, origem navegável, score, motor/versão, hashes e “Por que esta resposta?”. O assistente informa explicitamente quando não há suporte suficiente, não executa ações, não acessa outros casos e trata o texto recuperado como dado inerte. Mensagens permitem abrir o contexto temporal/grafo já existente.

Limites deliberados: esta primeira camada não é chatbot nem modelo generativo; perguntas complexas exigem termos literais ou a Pesquisa profunda para controlar os mecanismos com precisão.

**Concluída quando:** perguntas sintéticas recebem respostas verificáveis com artefatos de suporte; o assistente diz que não encontrou suporte quando necessário; nenhuma resposta importante existe sem origem.

### E08 — Revisão humana, avaliação e governança de modelos

**Status: IMPLEMENTADA LOCALMENTE (E08)**

AI-7 recebeu uma fila paginada de revisão humana para candidatos de entidades documentais, com filtros por estado e versão do motor, proveniência/hash/offsets e navegação até o arquivo original. A decisão continua append-only em `revisao_entidade_documental`, separada da inferência automática. Os datasets autorizados já existentes preservam autorização, retenção, versões, hashes, revisor e métricas de precision/recall/F1/acurácia/cobertura; continuam sem treinamento silencioso ou transferência entre casos. A fila confere novamente o snapshot textual antes de apresentar qualquer candidato.

**Concluída quando:** modelos/versões podem ser avaliados em datasets sintéticos ou autorizados, com precision, recall, falsos positivos/negativos, cobertura, desempenho e limitações reproduzíveis. O comparativo controlado entre versões e revisão em lote permanecem como extensão posterior, sem bloquear a fila segura individual.

### E09 — Relatórios investigativos avançados

**Status: IMPLEMENTADA LOCALMENTE (E09)**

O PDF agora inclui, dentro da metodologia, um bloco de derivados investigativos com metadados dos snapshots documentais/OCR (motor, versão, regiões, entidades e hashes), resumo real da linha do tempo e relações experimentais derivadas das primeiras evidências de registro. Cada relação preserva classe direta/sugerida, mecanismo, motivo e score técnico quando existente. Transcrições locais e imagens já selecionadas continuam com origem, timestamps, motor e hash. O bloco é limitado e explicita o que não foi persistido, sem inventar resultados multimodais/visuais.

**Concluída quando:** relatório sintético inclui essas fontes de forma legível e rastreável, sem bloquear o restante do caso e com SHA-256/auditoria preservados. Modelos institucionais versionados, abertura automática do PDF pelo aplicativo, geração assíncrona e assinatura digital continuam extensões da fase.

### E10 — Expansão de parsers e contêineres, exceto Avilla

**Status: EM PROGRESSO — ZIP SEGURO E CONTATOS iOS CONSERVADORES IMPLEMENTADOS**

O ingestão agora abre o diretório central de ZIPs sem extração automática, rejeita nomes
inseguros/duplicados, limita a 200.000 entradas e limita a materialização de cada item a
512 MiB em temporário derivado descartável. Itens internos recebem o mesmo endereço estável
`arquivo.zip!/caminho`, entram no inventário/hash e podem ser entregues a parsers existentes;
fixtures sintéticas cobrem leitura, preservação da origem e traversal. Os demais formatos com
documentação e amostras autorizadas foram iniciados com parsers versionados para SMS do iOS,
histórico Chrome/Firefox/Safari, exportação JSON oficial do Telegram e e-mail RFC 5322 (`.eml`).
RCS, formatos proprietários de e-mail e extensões avançadas do WhatsApp continuam pendentes;
não implementar parser por suposição. `AddressBook.sqlitedb` agora é encaminhado ao parser
`ios_contatos`, que exige a tabela `ABPerson`, preserva as colunas brutas e recusa esquemas
desconhecidos. O plano IPED agora preserva
também um manifesto limitado dos arquivos de configuração conhecidos (`IPEDConfig.txt`,
`LocalConfig.txt`, OCR e transcrição), além da versão apenas quando declarada no próprio caso;
chaves fora da lista permitida e conteúdo do índice Lucene continuam sem leitura.

**Concluída quando:** cada novo formato possui detecção, fixture sintética identificada, teste de origem/intacto, parser versionado, integração visual e limites explícitos.

### E11 — Administração, providers e segurança operacional

**Status: IMPLEMENTADA LOCALMENTE — CONTROLE DA ESTAÇÃO**

O banco global mantém o manifesto de providers e agora também permissões explícitas por usuário/ação,
trilha encadeada própria da estação e comandos Tauri para execução controlada. Um provider local habilitado
só pode ser verificado com ação autorizada e recebe exclusivamente `--version`, sem shell, rede ou conteúdo
de caso, com limite de dois segundos. O enriquecimento de IMEI usa somente arquivo JSON/CSV local configurado,
com limite de 4 MiB e resultado marcado como derivado. Backup usa `VACUUM INTO` e SHA-256; restauração lê um
backup global compatível e faz merge validado de providers/permissões sem apagar casos. A tela de Configurações
expõe concessão/revogação por ação, verificação, IMEI, backup e conferência da trilha. Provedores externos
continuam recusados e segredos não são aceitos.

**Concluída quando:** permissões são aplicadas no backend; providers ficam desativados por padrão; toda saída externa exige configuração e ação explícita; segredos não entram no banco do caso nem no Git. Os quatro critérios agora têm cobertura local; autenticação institucional e provider externo continuam fora desta etapa.

### E12 — Desempenho, retomada e preparação de release

**Status: EM ANDAMENTO — limites, hashing controlado e checkpoint de inventário implementados localmente**

O pipeline agora resolve limites locais por execução (`SIED_IMPORT_MAX_FILES`,
`SIED_IMPORT_MAX_BYTES` e `SIED_IMPORT_HASH_THREADS`, com tetos internos de segurança),
recusa a importação antes de exceder arquivos/bytes e registra os limites no log da
operação. O inventário pode usar uma janela limitada de workers (1–8) para calcular
SHA-256 em paralelo sem carregar a extração inteira; cancelamento é cooperativo e a
origem continua somente leitura. Cada importação grava `checkpoint-retomada.json` por
substituição via arquivo temporário, com fonte, estado, progresso, limites e versão do
formato. O avaliador `avaliar_retomada` compara a fonte registrada com a fonte atual e
sempre devolve conferência obrigatória; fonte divergente não pode ser reaproveitada.
Estados interrompidos nunca são tratados como sucesso; a retomada automática da mesma
transação ainda não é habilitada para evitar duplicação de registros. O armazenamento
também recusa uma segunda importação da mesma origem, dispositivo e importador enquanto
a primeira permanece `em_andamento`, inclusive após reinício do aplicativo. Execuções
canceladas, falhas ou interrompidas continuam exigindo uma nova decisão explícita.
Na tela da extração, a ação “Marcar interrompida” fecha somente uma execução órfã,
registra auditoria e não retoma, apaga ou reinterpreta registros.

Fixtures sintéticas cobrem hashing paralelo, cancelamento e leitura de lote. O benchmark
`cargo bench -p sied-ingestao --bench inventario` mede o núcleo de hash com bytes sintéticos
e permite comparar execuções pelo mesmo ambiente. Ele deve ser executado em ambiente Rust/MSVC
suportado. Consolidação
do instalador, atualização e recuperação operacional ainda dependem do pipeline de CI e
da validação Windows correspondente.

**Concluída quando:** cenários grandes possuem métricas reproduzíveis; cancelamento/retomada não falsificam sucesso; testes completos, Clippy, typecheck, build e instalador passam em ambiente suportado.

### E13 — Avilla Forensics

**Status: EM ANDAMENTO — RECONHECIMENTO LOCAL; PARSER BLOQUEADO POR AUSÊNCIA DE EXTRAÇÃO**

O reconhecimento local confirmou a instalação/suíte Avilla, as ferramentas chamadas pelo módulo iOS
e a existência de diretórios de aquisição, mas não encontrou arquivos em uma extração real. O
importador continua `NaoImplementado` por segurança. Implementar somente após receber árvore/amostras
de laboratório autorizadas, versão da ferramenta, opções de aquisição e arquivos de manifesto/log
aplicáveis. Não presumir nomes, bancos ou caminhos. Evidência do reconhecimento:
`docs/checkpoints/2026-10-09-e13-avilla-reconhecimento.md`.

**Concluída quando:** detecção e planejamento são baseados em estrutura real documentada; fixture sintética reproduz apenas a estrutura necessária; origem permanece intacta; importação, parsers, testes e interface funcionam de ponta a ponta.

## Checklist obrigatório ao concluir qualquer etapa

1. funcionalidade real no backend e persistência quando aplicável;
2. interface utilizável e navegação até o artefato original;
3. origem, mecanismo, versão, score/confiança e limitações preservados;
4. processamento local/offline por padrão;
5. cancelamento, paginação e incrementalidade quando a operação puder ser longa;
6. testes direcionados e regressão das áreas afetadas;
7. contratos TypeScript regenerados pelo Rust, nunca escritos manualmente;
8. typecheck e build da interface;
9. validação Rust/Clippy ou registro exato da dependência indisponível;
10. documentação e este ponteiro atualizados com fatos;
11. commit e push na branch da etapa, sem merge automático.
