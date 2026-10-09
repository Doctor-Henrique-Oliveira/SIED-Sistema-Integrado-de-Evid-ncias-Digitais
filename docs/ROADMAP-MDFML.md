# SIED — Roadmap do Motor de Descoberta Forense Multimodal Local

Data: 02/10/2026 · Status: **FUTURO — NÃO IMPLEMENTADO** · Nome/sigla provisórios: **MDFML**.

Registro estratégico de produto solicitado pelo responsável pelo SIED. Entrada oficial de
planejamento: [CONTINUIDADE.md §14.2](CONTINUIDADE.md#142-motor-de-descoberta-forense-multimodal-local-mdfml).
Este documento detalha uma capacidade futura; não é autorização para desenvolvimento nem
aprovação de modelos, dependências, fornecedores, esquema de banco ou contratos executáveis.
O grafo é experimental; exemplos de telas, consultas e categorias são conceituais, sem dados
reais de caso e sem resultados produzidos pelo SIED nesta missão.

## 1. Objetivo e fronteira

Reduzir o tempo necessário para o analista encontrar, revisar, correlacionar e documentar
artefatos relevantes em grandes extrações digitais, mantendo integridade, rastreabilidade,
privacidade, explicabilidade, reprodutibilidade e controle humano.

Evolução pretendida: plataforma local de descoberta e análise assistida de evidências digitais,
além da navegação de extrações. Abrange mensagens, contatos, imagens, screenshots, PDFs,
documentos, arquivos, texto OCR, metadados, indicadores técnicos e localizações.

O núcleo deve funcionar sem Internet e sem GPU. Nenhuma extração, mensagem, imagem, documento
ou dado do procedimento será enviado automaticamente para APIs, modelos, serviços de IA,
mecanismos de busca, visão computacional ou terceiros.

O motor propõe **candidatos para revisão humana**. Não determina que um objeto é droga/arma,
que uma pessoa é criminosa ou integrante de grupo, nem que uma conversa comprova crime.
Palavras, emojis, similaridade e correlações são sinais para pesquisa, não essas conclusões.

## 2. Compatibilidade com o estado atual e tensões registradas

Referências: [HANDOFF-CODEX.md](HANDOFF-CODEX.md) §§3, 13, 16–18, 23, 25–27 e 32;
[ARQUITETURA.md](ARQUITETURA.md) (ajustes aprovados e fase 5);
[VISAO.md](VISAO.md) §§8, 14–17 e 21;
[CONTINUIDADE.md](CONTINUIDADE.md) §§1, 7, 13 e 14. O código define o estado efetivo.

| Base/tensão documental | Consequência para o planejamento futuro |
|---|---|
| SIED atual local e 100% offline; frontend sem disco/shell/rede irrestritos | Núcleo MDFML local. Conectores externos exigem decisão técnica e institucional posterior; esta missão não libera permissões nem afrouxa a CSP. |
| Conversas, FTS e miniaturas são derivados dos artefatos existentes | Inferências, OCR e vetores precisam de espaço derivado próprio, versionado e conferível; não podem ser inseridos como fatos nos artefatos ou nas conversas atuais. |
| Arquitetura original propõe tabelas `derivado`/`tarefa`; a nota de estado atual diz que não existem | São propostas históricas, não infraestrutura disponível. Persistência de jobs/resultados e migrações novas dependem de desenho futuro. |
| Importação atual sem retomada automática; processamento assistido deseja pausa/retomada | Fila MDFML será objeto de planejamento separado. Não presumir que o pipeline atual já oferece essas capacidades. |
| Já há FTS/texto, inventário, origem e linha do tempo; OCR, semântica e visão não estão entregues | Reutilizar a base conferida; não apresentar busca multimodal, emojis especializados, timeline assistida ou modelos locais como implementados. |
| Fases gerais 0–5 da arquitetura e prioridades do HANDOFF/CONTINUIDADE | AI-0–AI-7 detalham o eixo futuro de inteligência; não renumeram fases gerais nem antecipam Avilla/validação dos parsers. |

Não resolver divergências inventando comportamento ou mudando o histórico. Na etapa de desenho,
revalidar o código e registrar qualquer incompatibilidade antes de implementar.

## 3. Invariantes que atravessam todas as fases

- Nunca modificar a extração, arquivo, banco, conteúdo, timestamp, metadado ou hash original.
  Bancos de evidência somente em cópias verificadas, abertas somente leitura.
- OCR, transcrições futuras, embeddings, vetores, classificações, índices, thumbnails,
  vocabulários e enriquecimentos externos são derivados; não substituem evidência original.
- Não acrescentar inferências aos campos históricos de `NovoArtefato`, nem alterar casualmente
  a regra de `hash_registro`. A referência ao hash original não é o hash do resultado derivado.
- Preservar as tabelas forenses de somente inclusão, auditoria encadeada/âncoras e a conferência
  de conteúdo/hash na geração de relatório. Nunca registrar PDF parcial ou inexistente.
- Migrações publicadas são imutáveis. Um eventual esquema derivado requer desenho e migração
  nova; nenhum esquema, tabela ou caminho de armazenamento foi escolhido nesta missão.
- Sem permissões amplas de disco, shell ou rede no frontend. Dados implausíveis/indisponíveis
  continuam como desconhecidos ou valores brutos; a inteligência não pode "corrigi-los" em fatos.
- Dados e fixtures de teste continuam sintéticos, identificados como tais. Não versionar casos,
  credenciais, extrações ou datasets investigativos.

## 4. Camadas independentes de descoberta

| Camada | Função futura |
|---|---|
| A — Texto tradicional | Preservar pesquisa textual/exata disponível e FTS, sem regressões. |
| B — Expansão linguística | Dicionários contextualizados e árvore de termos visível ao analista. |
| C — OCR | Extrair texto localmente, com referência à página/região e ao original. |
| D — Semântica | Embeddings locais e recuperação por significado, com limitações explicitadas. |
| E — Visão | Priorizar imagens/regiões candidatas para revisão. |
| F — Multimodal | Combinar texto, OCR, imagem, metadados e contexto de artefatos. |
| G — Correlação | Relações diretas e sugestões separadas; exploração temporal/geográfica. |
| H — Revisão humana | Confirmar, descartar ou manter inconclusivo; aplicável desde o primeiro resultado assistido. |

Nenhuma camada depende obrigatoriamente das posteriores. A revisão humana é transversal;
pesquisa tradicional continua utilizável se OCR ou modelos estiverem ausentes/indisponíveis.

Fluxo conceitual: extração preservada → inventário/artefatos → índices/derivados locais →
pesquisa híbrida e correlações → revisão do analista → seleção explícita de evidências → relatório.

## 5. Pesquisa tradicional, vocabulários e árvore investigativa

Preservar o comportamento da busca atual. Expansão futura é opção explícita, sem transformar
uma consulta literal em expansão escondida. Mostrar consulta original e os termos realmente usados.

O **Dicionário Investigativo Extensível** admite termos, sinônimos, abreviações, grafias
alternativas, erros ortográficos, expressões, emojis/combinações, regionalismos, códigos,
siglas e entradas de usuário/instituição/caso. Cada associação tem origem/proveniência,
autor, versão, data e escopo (caso atual ou perfil institucional).

Exemplos fornecidos pelo responsável: `PA`, `PJL`, `TUDO 3`, `TUDO TREESSSS`, `TRES`, `TUDO`,
`TUDO 2`, `maconha`, `droga`, `skank`, `chá` e `BDL`. Servem apenas para registrar a necessidade
de vocabulários locais contextualizados. Não definem significado criminoso universal, nem
autorizam criar um pacote com associações automáticas. Uma palavra pode ter vários sentidos.

A **Árvore de Pesquisa Investigativa** agrupa sinônimos, abreviações, grafias alternativas,
expressões, emojis, regionalismos e termos do analista. Permitir ligar/desligar ramos,
inspecionar todos os termos, adicionar/remover entradas, importar/exportar vocabulários,
identificar versão/autor/origem/data e aplicar ao caso ou a um perfil institucional.
O snapshot da execução conserva os ramos efetivamente usados.

Descoberta de termos por coocorrência deve exibir ocorrências reais, quantidade de mensagens
e conversas, termos próximos e acesso ao contexto. O analista pode confirmar a associação,
ignorar, adicionar ao caso ou propor ao vocabulário. O motor não declara o significado do
termo e não aprende nem modifica o dicionário silenciosamente.

Emojis: planejar pesquisa de emoji individual, sequência, emoji+palavra e palavra+emoji,
frequência, combinações, períodos, interlocutores/identificadores e mensagens próximas.
Preservar a sequência original, sem deduzir identidade ou significado criminal. A adequação
da indexação Unicode deve ser validada; não presumir que o FTS atual atende a essas pesquisas.

## 6. OCR, PDFs e documentos locais

OCR futuro para prints, fotografias, PDFs digitalizados, imagens de conversas, documentos,
recibos/comprovantes, placas, embalagens e textos parciais. Guardar derivadamente texto,
arquivo/hash de origem, motor/versão, parâmetros, regiões/bounding boxes e score/confiança,
quando disponíveis. Ausência de confiança/região deve ser explícita; não fabricar esses valores.

Pipeline conceitual de documentos: texto nativo → OCR quando necessário → imagens incorporadas
→ metadados → indexação. A seleção do caminho depende do conteúdo/formato suportado.
Resultados devem indicar a página exata e região, quando existente, e abrir o original.
Texto OCR não substitui transcrição literal do original nem modifica o documento.
Pesquisa global poderá incorporar OCR e conteúdo documental como fontes derivadas identificadas.

## 7. Semântica, busca híbrida e pesquisa multimodal

Embeddings e modelos locais poderão combinar correspondência textual/FTS, OCR, similaridade
semântica/visual, metadados e proximidade temporal. O ranking híbrido deve expor a contribuição
dos sinais e seus limites; um score de recuperação não é certeza de fato nem probabilidade de crime.

Consultas naturais futuras incluem "imagens contendo motocicletas", "prints contendo
comprovantes", "documentos mencionando determinado nome", "mensagens relacionadas a
transferências financeiras", "imagens com texto contendo determinado telefone" e "arquivos
relacionados a determinado domínio". São exemplos de descoberta, não garantias de detecção.

**Pesquisa Universal**: mensagens, contatos, nomes, telefones, arquivos, OCR, PDF/documentos,
mídias, metadados, indicadores, localizações e resultados derivados, com filtros de escopo.
**Pesquisa Profunda**: execução explicitamente escolhida pelo analista, que seleciona mecanismos
(mensagens, OCR, documentos, imagens, nomes de arquivo e metadados) e vê qual recuperou cada item.
Itens sem suporte devem aparecer como indisponíveis, sem fingir análise.

Combinação OCR + visão + contexto pode reunir região candidata, texto, metadados e mensagem
associada, mantendo cada sinal identificável e acessível separadamente.

## 8. Visão, regiões, exemplo visual e duplicatas

Visão computacional local serve para **priorizar imagens para revisão**. Conforme modelos
instalados e validados, categorias de pesquisa poderão incluir possíveis armas, substâncias/
materiais de interesse, dinheiro, veículos, documentos, telas de aplicativos, QR codes,
texto e outros objetos. Toda resposta continua sendo candidata.

Avaliar objetos parciais, cortados, pequenos, nos cantos, obstruídos, comprimidos, desfocados,
mal iluminados, vistos em screenshots ou dentro de outra cena. Não prometer identificar
qualquer objeto; registrar falhas, limitações e níveis de score distintos.

Quando o mecanismo permitir, conservar arquivo/hash original, bounding box/coordenadas de
região, categoria sugerida, score, modelo/versão e transformação aplicada à imagem analisada.
Ampliar uma região não deve perder acesso à imagem completa nem alterar o original.

**Pesquisa visual por exemplo**: selecionar imagem/região para buscar semelhantes no próprio
caso (objetos, cenários, documentos fotografados, thumbnails, screenshots e modificações).
Similaridade não significa identidade. Separar duplicata por hash exato de near-duplicate por
perceptual hash ou similaridade visual; explicitar recompressão/redimensionamento e outros limites.
Nenhuma deduplicação elimina automaticamente arquivos ou artefatos de origem.

## 9. Entidades e Central de Indicadores

Entidades assistidas: pessoas mencionadas, telefones, e-mails, endereços, valores monetários,
datas, placas, URLs, domínios e IPs. Resultados são candidatos corrigíveis pelo analista;
nome mencionado não comprova identidade, vínculo ou presença de uma pessoa.

A futura **Central de Indicadores** reúne IPs, domínios, URLs, hashes, coordenadas, e-mails,
telefones e possíveis identificadores técnicos, sempre com ocorrência/origem/contexto.
Separar string encontrada, validação sintática e significado atribuído; números/coords
implausíveis não viram dados válidos. Contagens são reais e devem definir se medem ocorrências
ou valores distintos. Nenhuma consulta externa decorre automaticamente da detecção.

## 10. Correlação, timeline, mapas e grafo experimental

Relações entre mensagem, contato, telefone, imagem, arquivo, localização, URL, domínio, IP
e documento devem distinguir **direta** (sustentada pelo artefato, com origem) de **sugerida**
(inferência de mecanismo versionado). Não deduzir identidade entre pessoas/contas pelo score.

Timeline assistida: janelas temporais de ±5 min, ±30 min, ±1 h e ±24 h, entre mensagens,
imagens, localizações, documentos, ligações e demais eventos com data válida. Preservar
timestamp original, conversão e fuso, indicando ausência/incerteza. Proximidade temporal
não implica causalidade e não cria automaticamente uma relação investigativa.

Mapas futuros offline podem mostrar coordenadas de origem verificável, clusters, sequência
cronológica e filtros por dispositivo, conversa e período. Ponto sem data não vira trajeto;
trajetos precisam de justificativa técnica. A origem de cada coordenada permanece acessível.

**Grafo investigativo: FUTURO/EXPERIMENTAL.** Nós e arestas devem expor origem/mecanismo,
força/limitação do vínculo e revisão humana. Uma inferência fraca não recebe a mesma aparência
de uma relação direta. O planejamento não autoriza correlação automática entre casos distintos.

## 11. Explicabilidade, rastreabilidade e reprodutibilidade

Todo resultado deve responder: o que foi encontrado, onde, por que apareceu, qual mecanismo
o produziu, qual versão/configuração foi usada, qual artefato original e qual hash identifica
o conteúdo analisado. Links diretos devem abrir arquivo, página/região, mensagem, conversa,
extração, metadados e origem técnica quando esses vínculos existirem.

Registro conceitual mínimo, a especificar no desenho técnico (não é uma tabela implementada):

| Grupo | Informação a conservar |
|---|---|
| Origem | Caso/extração/dispositivo, arquivo/hash, artefato/hash do registro quando aplicável, caminho, banco/tabela/id, página ou região e data/fuso originais. |
| Derivação | ID do resultado, tipo de mecanismo, modelo/motor e versão, identificação verificável do pacote/pesos, configuração, entradas efetivas, data e limites/avisos. |
| Recuperação | Consulta original, termos expandidos, ramos/filtros, índice/versão, vocabulário/versão, modelos/versões, parâmetros, score e sinais que justificam o ranking. |
| Revisão | Estado, decisão humana, autor/data e justificativa; separado da inferência histórica. |

**Consultas/resultados salvos**: nome, consulta, termos, filtros, escopo, mecanismos/modelos,
data e resultados/contagens. O snapshot preserva versões do índice, vocabulário e modelo;
registrar também o universo de artefatos considerado para explicar mudanças posteriores no caso.
Reexecução não substitui a anterior. Caso o mecanismo seja não determinístico, conservar
parâmetros/semente quando aplicáveis e declarar os limites da reprodução.

Painel **"Por que este resultado apareceu?"** deve separar, por exemplo, trecho OCR, palavra
na mensagem associada, score semântico, região sugerida e filtro temporal. Exibir ausência de
dados/proveniência como tal; nunca completar lacunas por geração de texto.

## 12. Revisão e controle humano

Fila de Revisão Inteligente: priorização por faixas de score, filtros e contexto. Alta
confiança não significa verdadeiro; baixa não significa falso. Valores devem ser apresentados
como score técnico do mecanismo, não porcentagem de verdade; confiança só será usada quando
o modelo oferecer esse dado com definição e validação adequadas.

Ações conceituais: abrir original, ver contexto, confirmar, descartar e marcar inconclusivo.
Estados: **NÃO REVISADO**, **CONFIRMADO PELO ANALISTA**, **DESCARTADO PELO ANALISTA** e
**INCONCLUSIVO**. A interface deve explicar o que o analista confirmou; confirmar uma categoria
ou associação não equivale a conclusão jurídica ou materialidade pericial.

Inferência original e revisão humana ficam separadas. Revisões e reprocessamentos preservam
o histórico; nunca sobrescrever silenciosamente uma inferência anterior. Nenhum candidato
é automaticamente evidência selecionada ou conteúdo de relatório.

## 13. Providers, hardware, fila e cache

Interfaces futuras, com nomes conceituais `TextModelProvider`, `EmbeddingProvider`,
`VisionProvider` e `OcrProvider`, devem desacoplar o SIED de LLM, visão, OCR, banco vetorial
e fornecedor específicos. Contratos, runtimes e armazenamento ainda precisam de estudo/aprovação.
Não instalar modelos/dependências nem modificar os parsers para acomodar a proposta agora.

Degradação elegante, a validar por benchmarks: máquina básica usa pesquisa tradicional/FTS
e OCR leve quando viável; intermediária pode ter embeddings, semântica e modelos visuais
compatíveis; workstation pode processar modelos maiores, lotes e multimodal local. Não há
requisito mínimo de RAM/GPU definido; sem GPU/modelo, funções básicas do caso continuam disponíveis.

Jobs pesados incrementais/assíncronos devem oferecer pausa, retomada, cancelamento, checkpoint,
falha por item e reprocessamento. Limitar CPU/RAM/GPU/disco e concorrência; não segurar o caso
ou carregar extrações inteiras na memória durante inferência. Mostrar totais reais de analisados,
pendentes e falhas, com categorias de estado e contagem definidas no desenho técnico.
Prever milhares de mensagens/imagens/arquivos, PDFs grandes, múltiplos bancos e dispositivos.

Cache conceitual: hash da entrada/artefato + mecanismo/modelo + versão + configuração relevante
(incluindo região/pré-processamento quando usados). Só reutilizar derivado validado e compatível;
mudanças geram nova versão. Guardar tudo na área de trabalho do caso, separado da extração;
definir no desenho como copiar/arquivar o caso e reconstruir os índices sem perder a história.

## 14. Feedback supervisionado, datasets e robustez

Separar **BASE MODEL**, **MODEL/VOCABULARY PACK** e **CASE-SPECIFIC FEEDBACK**. Marcas humanas
confirmado/descartado/incerto são feedback contextual; não mudam silenciosamente modelos globais
ou vocabulários institucionais. Treinamento é ação explícita, autorizada, versionada e reproduzível.

Datasets curados exigem proveniência, autorização, hash, rótulo, revisor, versão, origem,
licença quando aplicável e retenção. Não incorporar automaticamente imagens de casos reais em
datasets permanentes, nem transferir feedback entre casos. Exemplos de treinamento/avaliação
precisam de autorização específica; fixtures do repositório continuam sintéticas.

Avaliação independente do treinamento e das demonstrações: exemplos sintéticos ou autorizados
com alta/baixa resolução, compressão, desfoque, ruído, oclusão, objeto parcial/pequeno,
iluminações e ângulos diferentes. Medir precisão (precision), recall, falsos positivos,
falsos negativos e desempenho/recursos. Limiar não será escolhido pela aparência da demonstração.

## 15. Segurança, privacidade e processamento externo

Minimização: modelo local recebe apenas o conteúdo necessário à operação. Dados de casos
não se misturam ao catálogo global, aos logs técnicos ou a outros casos sem escopo/autoridade
explicitamente definidos. Índices, OCR e vetores também são sensíveis e precisam de controle
de acesso/retenção equivalente aos dados do caso; offline não significa acesso irrestrito.

Antes de cada fase, desenhar ameaças de documentos/imagens/modelos hostis, esgotamento de
recursos e instruções maliciosas contidas nos artefatos. Conteúdo de evidência é dado, nunca
comando para o assistente executar shell, ler outros casos ou enviar informação a terceiros.
Validar origem/integridade/licença dos pacotes locais e isolar falhas do motor sem enfraquecer
as fronteiras existentes de leitura e permissões. Não há sandbox/runtime novo aprovado aqui.

**Conectores externos: FUTUROS, OPCIONAIS, SEPARADOS.** Enriquecimento de IP, domínio, URL,
hash, coordenada ou IMEI só com serviço autorizado/configurado, governança institucional e
ação explícita. Fluxo: artefato local → usuário solicita → mostrar exatamente dados e destino
→ autorização da operação → serviço externo → resultado armazenado separadamente.

Interface identifica processamento externo, serviço, dados enviados e data; auditoria registra
o evento com minimização. Nunca enviar extração inteira por conveniência nem consultar um
indicador automaticamente. Resultado externo é enriquecimento com proveniência/limites, não
evidência original. Credenciais não vão ao código/log/Git. Núcleo local funciona sem conectores.

## 16. Auditoria e relatórios futuros

Planejar eventos auditáveis de análise/consulta, mecanismo/modelo/versão, configuração,
alteração de vocabulário, enriquecimento externo, confirmação/descarte/revisão e exportação.
Preservar a trilha atual, suas âncoras e transações. Não despejar texto sensível ou prompts
com conteúdo de evidência no log técnico; referências/snapshots sensíveis ficam no caso.

Resultados automatizados só entram no relatório por seleção explícita do analista e após
desenho próprio para derivados/revisões. Distinguir conteúdo original, derivado, resultado
automatizado, revisão humana e consulta externa, com origem e versão.
Revisão humana não dispensa recalcular hashes/conferir conteúdo e cumprir as garantias de
PDF completo/registrado. Não usar campos de observação como caminho silencioso para inserir
inferências, nem alterar o relatório existente nesta missão.

## 17. Interface futura

Módulo conceitual **Análise Inteligente**, sem implementação agora: pesquisa no caso, filtros
por mensagens/imagens/documentos/OCR/indicadores/tudo, resultados paginados, contexto/origem,
painel "Por que este resultado?" e fila de revisão. Contagens e estados reais; mecanismos
ausentes explicitamente indisponíveis. Preservar acesso rápido ao original e aos detalhes técnicos.

O assistente local poderá apoiar consultas naturais, sempre com referências e limites;
o módulo deve continuar sendo ambiente de pesquisa/revisão, não apenas chatbot. Não desenhar
novas telas nem modificar o trabalho visual/premium atual como parte deste registro.

## 18. Faseamento proposto AI-0 → AI-7

Todas as fases abaixo são **FUTURAS, NÃO IMPLEMENTADAS como capacidade MDFML**, sem datas
ou autorização de execução. A existência de FTS/linha do tempo hoje é base, não conclusão da fase.

| Fase | Escopo planejado | Entrega a validar antes de avançar |
|---|---|---|
| AI-0 | Pesquisa avançada tradicional, vocabulários, árvore de pesquisa, emojis e Central de Indicadores. | Expansão transparente, snapshot de termos/filtros, pesquisa exata preservada e revisão dos indicadores sem rede. |
| AI-1 | OCR local, PDF/documentos e entidades. | Origem/hash/página/região conferíveis, erros visíveis, busca em derivados e entidades candidatas corrigíveis. |
| AI-2 | Embeddings locais, pesquisa semântica e ranking híbrido. | Índices versionados/reconstruíveis, justificativa por sinal e desempenho comparado à pesquisa tradicional. |
| AI-3 | Visão/classificação, objetos/regiões e near-duplicates. | Scores/limitações avaliados, acesso à imagem completa, similaridade distinta de identidade e fila de revisão. |
| AI-4 | Pesquisa multimodal: imagem + texto + OCR + contexto. | Combinação de sinais preserva proveniência individual e usa apenas capacidades instaladas/validadas. |
| AI-5 | Correlação, timeline assistida e grafo experimental. | Relação direta/sugerida visualmente distinta, fuso/origem preservados, sem inferir causalidade ou identidade. |
| AI-6 | Assistente local explicável e consultas naturais sobre o caso. | Respostas referenciadas, "não sei" quando necessário, dados não executam instruções e controle humano das ações. |
| AI-7 | Feedback supervisionado, datasets internos autorizados e avaliação/melhoria de modelos. | Treinamento explícito/versionado, governança/retenção, avaliação independente e nenhum aprendizado global silencioso. |

Portão obrigatório de **cada fase**, antes de merge: escopo/autorização próprios; design
técnico; modelo de ameaças; testes; benchmarks; dataset sintético; avaliação de falsos
positivos e negativos; auditoria; revisão de privacidade; checkpoint revisável. Só avançar
com critérios mensuráveis aprovados e evidência de que as funções básicas continuam utilizáveis.
Dados autorizados de avaliação complementam, não substituem, as fixtures sintéticas exigidas.

## 19. Decisões ainda abertas e critério de sucesso

Ainda dependem de estudo e aprovação: contratos/providers; motores/modelos e licenças;
armazenamento/versão dos derivados; distribuição offline e integridade dos pacotes;
recursos mínimos/benchmarks; validação/calibração de scores; critérios de aceitação;
retenção/controle de acesso; representação de derivados em evidências/relatórios;
governança de vocabulários/datasets e eventual autorização de conectores externos.

O sucesso será medido pelo tempo para encontrar, revisar, correlacionar e documentar resultados
úteis, pela qualidade da recuperação e pelos erros/recursos medidos. Esses ganhos só contam
se preservarem integridade, rastreabilidade, privacidade, explicabilidade, reprodutibilidade
e decisão humana. Esta missão registra o roadmap; não altera código, parsers, migrations,
backend, banco, dependências, modelos, datasets, relatórios ou a experiência visual.
