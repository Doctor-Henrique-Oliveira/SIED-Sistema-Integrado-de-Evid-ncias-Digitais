# AI-1 — PDF nativo por página

Data: 02/10/2026. Branch: `feat/mdfml-ai1-documentos-locais`. PR #25 em rascunho,
sem merge. Continuação do leitor documental, não conclusão da fase AI-1.

## IMPLEMENTADO

- Leitura sob demanda de texto nativo de PDF no detalhe de **Arquivos**.
- Página inicial 1; Anterior/Próxima usam a contagem observada na árvore de páginas,
  conferida com a enumeração do motor. Não se confia somente em `/Count`.
- Página sem texto mantém a origem/paginação conferidas e permite seguir a leitura.
  Isso não representa reconhecimento de imagem/OCR.
- SHA-256 do PDF e tamanho conferidos contra o inventário **antes** do parser.
  SHA-256 separado do texto derivado da página; motor/versão, horário e identidade
  de caso/extração/arquivo acompanham o resultado.
- A fronteira desktop recebe somente IDs e página opcional. Caminho/hash/tamanho
  continuam resolvidos no inventário do caso aberto, sem escrita documental no banco.
- A UI descarta respostas tardias/incompatíveis, limpa o conteúdo ao mudar página,
  mantém texto inerte e distingue texto derivado de reprodução visual do PDF.
- UTF-8 continua fiel aos bytes, sem página/contagem inventadas.

## DECISÕES

Parser publicado **lopdf 0.45.0**, versão fixada, Rust puro, licença MIT, sem
features padrão. Não há parser PDF artesanal, renderizador, processo externo,
OCR, modelo, senha, execução de JavaScript ou rede na leitura.

Envelope inicial explícito: fontes Type1 Latin Base14 (Courier/Helvetica/Times e
variantes), StandardEncoding/WinAnsiEncoding/MacRomanEncoding, sem ToUnicode,
CMap/Type0, fonte incorporada, Form XObject ou imagem inline. A revisão final
deve usar tabela xref clássica; revisão final em xref stream é recusada.
Imagens XObject não são decodificadas nem reconhecidas.
Fontes, glifos, operadores de texto e estados gráficos fora desse envelope são
recusados, não convertidos com perdas. Texto nativo é derivado da estrutura do
PDF; não certifica a ordem visual, composição, aparência ou conteúdo das imagens.

São admitidos conteúdos sem filtro ou com FlateDecode, sem parâmetros de
decodificação. A validação zlib estrita e limitada usa o codec publicado `flate2`
antes da extração oficial, recusando filtros malformados, checksum inválido,
prefixos parciais e bytes residuais. Essa conferência é necessária porque o
motor tem caminhos lenientes de recuperação de conteúdo.

Fontes primárias: [LoadOptions](https://docs.rs/lopdf/0.45.0/lopdf/struct.LoadOptions.html),
[Stream](https://docs.rs/lopdf/0.45.0/lopdf/struct.Stream.html),
[código publicado](https://docs.rs/crate/lopdf/0.45.0/source/src/).
A licença MIT do pacote fixado foi incluída em `app/src-tauri/licenses/` e nos
recursos do bundle; a presença no instalador final depende da validação nativa.

## LIMITAÇÕES / AMEAÇAS

- Origem e texto: até 2 MiB; até 512 páginas, 4096 objetos e 64 fontes por página.
- Conteúdo descomprimido agregado da página: 256 KiB; expansão de cada stream
  de carga: 64 KiB. ObjStm é filtrado antes da descompressão; objetos comprimidos
  não são admitidos. Limites de objetos/páginas são conferidos **após** a carga.
- O prazo de 5 segundos e cancelamento são **cooperativos**, não preemptivos.
  O frontend não possui comando de cancelamento; ocultar apenas fecha a apresentação.
- Esses limites não são quota de RSS nem isolamento de processo. Pânico capturável
  é tratado sem publicar texto; OOM e bloqueio dentro de uma chamada não são contidos
  por `catch_unwind`. Isolamento/quota OS e corpus hostil mais amplo continuam
  necessários antes de alegar disponibilidade robusta para PDF arbitrário.
- Sem indexação documental, busca em PDF, persistência de derivados, entidades,
  OCR de páginas escaneadas ou alteração de relatórios/evidências.
- O confinamento de caminho do adapter não promete eliminar toda corrida de
  filesystem; links/reparse são recusados, inclusive possíveis placeholders OneDrive.
- O tipo xref observado é o da **revisão final**. O loader pode ter processado
  referências anteriores/híbridas antes dessa checagem, com limite por stream,
  mas sem validação estrita agregada dessa cadeia. Não alegar ausência de todo
  xref stream histórico nem suporte robusto a qualquer PDF incremental. Esta
  ressalva permanece na revisão de ameaça/isolamento anterior à integração.
- O SIED usa mensagens fixas e não registra texto documental; o parser de
  terceiros pode emitir warnings/erros. Revisar a configuração de logging e
  comportamento do motor antes de afirmar que todo erro é livre de dados de origem.
- Recursos inline herdados de nós Pages podem ser recusados pelo envelope atual
  de resolução da biblioteca. Isso não é indicação de PDF sem texto; não houve
  ampliação artesanal do resolvedor nesta entrega.
- O modelo de ameaças da fundação era anterior à fronteira desktop/PDF. A revisão
  técnica deste incremento não substitui o portão formal de ameaça/privacidade,
  desempenho e validação nativa antes de merge.

## TESTES

- Interface completa: `npm test -- --maxWorkers=2` — **113/113 aprovados**, 16 arquivos.
- Direcionados leitor/API: **43/43 aprovados** (39 leitor + 4 API).
- **29 testes PDF Rust normais + 1 medição opcional preparados**: writer oficial, páginas reais/WinAnsi, página vazia,
  fonte/hash/tamanho divergentes, pedidos inválidos, cancelamento, limites,
  compressão, filtros/checksum, imagem inline, fonte sem suporte, operações inválidas e árvore
  inconsistente. Fixtures exclusivamente sintéticas, criadas antes da importação.
- Teste integrado de origem/mtime, inventário e trilha intactos, sem escrita direta
  no banco. Medição opcional: 20 leituras/hash/parses reais de fonte em memória;
  não é medição de disco, RSS nem um SLO aprovado.
- `cargo fmt --all --check`, Cargo metadata/lockfile e `git diff --check` locais.
- Snapshot da CI #68 no commit `168cc1f44160d50da4ea408904785ed92006180f`:
  compilação Rust e exportação genuína de contratos aprovadas (4 testes da lib),
  **13/13 UTF-8 aprovados**; **20/21 PDF aprovados**, 1 falha de regressão em
  `filtro_comprimido_corrompido_nao_faz_fallback_para_bytes_crus`. A biblioteca
  aceitava o filtro inválido como recuperação sem texto, e não a falha esperada.
  Essa execução não valida o motor estrito final nem os novos testes adicionais.
- Correção: checksum/término/consumo zlib estritos, filtros e operandos válidos,
  imagem inline recusada; o mesmo teste de regressão permanece exigindo `Falhou`
  e nenhum texto, sem enfraquecimento. Rust/Clippy/medição **finais pendentes na CI**.
  Não houve polling/espera de jobs; a consulta com SHA abreviado foi corrigida para
  o SHA completo da mesma execução, sem acompanhar mudanças de estado.

## BUILD / PROBLEMAS

Build TypeScript/Vite **aprovado localmente**: `npm run build` (tsc + Vite,
116 módulos). Aviso de chunk principal >500 kB permanece não bloqueante;
sem refatoração ou alteração de configuração para mascará-lo.

`TextoDocumento` com `total_paginas` foi recuperado **sem autoria manual** da
exportação efetiva Rust/ts-rs, CI #68/job `110987307756`, commit `168cc1f`, antes
da falha de integração. O módulo exportador passou: a falha posterior não foi
tratada como aprovação da bateria PDF. O job de contratos testa também a biblioteca
e publica a exportação no log. Nenhum contrato foi criado manualmente.
Todos os gates Linux/Windows e instaladores oficiais continuam obrigatórios.

Toolchain nativo MSVC/Windows SDK indisponível nesta máquina; não houve nova
tentativa de build Tauri nem instalação de ferramentas/configuração corporativa.
Aplicativo Windows instalado não deve ser anunciado como contendo esta alteração
sem instalador correspondente. Validação nativa ainda pendente.

## ARQUIVOS

`crates/sied-documentos` (motor PDF, despacho, DTO, testes e dependências),
`Cargo.lock`, comando/adapter documental desktop, cliente API, leitor/testes da
interface, prévia sintética de teste, CI e recurso de licença do bundle. Banco, migrações, ingestão histórica,
hash_registro, auditoria, evidências e relatórios preservados. `.claude/` intocada.

## COMO EXECUTAR / TESTAR

Na versão compilada deste PR, abrir um caso sintético normalmente, entrar em
Arquivos e selecionar um PDF nativo inventariado com SHA-256 e até 2 MiB.
Clicar **Ler documento**, navegar nas páginas e conferir proveniência, hashes e
recusas. Não usar documentos reais nem criar/adulterar arquivos na extração atual.

A entrada `src/testes/previa-documentos.html` permite inspecionar a apresentação
sintética, explicitamente **backend não conectado**; não é validação do parser,
caso real ou execução Tauri. A automação de browser estava bloqueada por política;
não foi contornada, não houve screenshot nem confirmação visual automatizada.

## PRÓXIMA FASE

Validar motor/contratos na CI e fluxo nativo com instalador correspondente, sem
aguardar/pollar jobs nesta missão. Completar revisão de ameaça/privacidade e
avaliação de formatos/limites antes de ampliar o envelope. AI-1 permanece parcial;
OCR/entidades/indexação documental são incrementos posteriores. Não iniciar AI-2.
