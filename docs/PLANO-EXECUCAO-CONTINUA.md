# SIED — Plano oficial de execução contínua

Atualizado em 08/10/2026. Este documento transforma a visão, o README e o ROADMAP MDFML em uma fila executável. Ele não substitui os invariantes de `VISAO.md`, `ARQUITETURA.md` e `ROADMAP-MDFML.md`.

## Ponteiro atual

**Próxima etapa: E01 — OCR local e documentos pesquisáveis.**

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

**Status: PENDENTE**

Evoluir o dHash existente com classificação/detecção local, regiões e bounding boxes, mecanismo e versão. Manter duplicata exata, near-duplicate e candidato de objeto como conceitos separados. Todo resultado permanece candidato para revisão humana.

**Concluída quando:** imagens sintéticas produzem candidatos e regiões navegáveis, pesquisa por semelhantes funciona e a interface nunca apresenta classificação como conclusão pericial.

### E05 — Motor multimodal unificado

**Status: PENDENTE**

Integrar texto original, documentos, OCR, transcrições, entidades, embeddings, visão e metadados numa única execução. Normalizar, deduplicar e paginar resultados; permitir escolha prévia e filtro posterior de mecanismos.

**Concluída quando:** uma consulta sintética retorna artefatos de modalidades diferentes numa lista coerente e cada resultado explica separadamente todos os sinais que contribuíram.

### E06 — Timeline inteligente, mapa offline e grafo investigativo

**Status: PENDENTE**

Evoluir a timeline e o grafo limitados já existentes. Adicionar mapa realmente offline, filtros temporais/geográficos, agrupamento, expansão progressiva e persistência incremental das relações. Distinguir relação direta de sugerida e nunca inferir causalidade.

**Concluída quando:** eventos e relações abrem seus artefatos originais; mapas funcionam sem rede; grandes casos não carregam o grafo inteiro; toda aresta explica sua origem.

### E07 — Assistente local explicável

**Status: PENDENTE**

Substituir a limitação da gramática atual por consultas naturais locais sobre recuperação seletiva. Produzir respostas estruturadas, fontes clicáveis, filtros usados, scores e “Por que esta resposta?”. Implementar recusa por falta de suporte e isolamento contra instruções contidas na evidência.

**Concluída quando:** perguntas sintéticas recebem respostas verificáveis com artefatos de suporte; o assistente diz que não encontrou suporte quando necessário; nenhuma resposta importante existe sem origem.

### E08 — Revisão humana, avaliação e governança de modelos

**Status: PENDENTE**

Completar AI-7 com filas de revisão para todos os motores, revisão em lote segura, comparação controlada entre versões e métricas apropriadas. Preservar inferência original e decisão humana separadamente. Não treinar silenciosamente.

**Concluída quando:** modelos/versões podem ser avaliados em datasets sintéticos ou autorizados, com precision, recall, falsos positivos/negativos, cobertura, desempenho e limitações reproduzíveis.

### E09 — Relatórios investigativos avançados

**Status: PENDENTE**

Integrar OCR, transcrições, timeline, mapas, relações e explicações multimodais ao relatório. Adicionar modelos institucionais versionados, abertura do PDF pelo aplicativo e geração assíncrona. Estudar assinatura digital sem inventar autoridade certificadora.

**Concluída quando:** relatório sintético inclui essas fontes de forma legível e rastreável, sem bloquear o restante do caso e com SHA-256/auditoria preservados.

### E10 — Expansão de parsers e contêineres, exceto Avilla

**Status: PENDENTE**

Priorizar formatos com documentação e amostras autorizadas: ZIP seguro, iOS, RCS, Telegram, navegadores, e-mail e extensões do WhatsApp (reações, edições, enquetes, chamadas e mapeamentos de identificadores). Não implementar parser por suposição.

**Concluída quando:** cada novo formato possui detecção, fixture sintética identificada, teste de origem/intacto, parser versionado, integração visual e limites explícitos.

### E11 — Administração, providers e segurança operacional

**Status: PENDENTE**

Adicionar configurações editáveis, usuários/perfis/permissões locais, política de acesso ao caso, backup/restauração controlados e gerenciamento de motores. Criar infraestrutura opt-in para providers, incluindo enriquecimento de IMEI, sem envio automático e com auditoria da ação.

**Concluída quando:** permissões são aplicadas no backend; providers ficam desativados por padrão; toda saída externa exige configuração e ação explícita; segredos não entram no banco do caso nem no Git.

### E12 — Desempenho, retomada e preparação de release

**Status: PENDENTE**

Implementar retomada segura de tarefas/importações onde tecnicamente possível, hashing paralelo controlado, limites de recursos, benchmarks e testes de grande volume. Consolidar instalador, atualização, recuperação de falhas e documentação operacional.

**Concluída quando:** cenários grandes possuem métricas reproduzíveis; cancelamento/retomada não falsificam sucesso; testes completos, Clippy, typecheck, build e instalador passam em ambiente suportado.

### E13 — Avilla Forensics

**Status: PENDENTE — ÚLTIMA ETAPA OBRIGATÓRIA**

Implementar somente após todas as etapas anteriores e após receber árvore/amostras de laboratório autorizadas, versão da ferramenta, opções de aquisição e arquivos de manifesto/log aplicáveis. Não presumir nomes, bancos ou caminhos.

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
