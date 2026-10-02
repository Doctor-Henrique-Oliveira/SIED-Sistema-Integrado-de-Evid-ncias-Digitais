# Checkpoint — AI-0: descoberta local explicável

Data: 02/10/2026. **EM IMPLEMENTAÇÃO; VALIDAÇÃO PENDENTE. NÃO LIBERADO PARA USO PERICIAL.**

Branch: `feat/mdfml-ai0-busca-local`. Base oficial conferida:
`f3123f548852ec6962c34f1c01e97e8ce09cef5c`. Registro do roadmap: `7f79835`.
Após a missão documental, o responsável autorizou no chat iniciar a implementação.
Este checkpoint não conclui a AI-0 nem autoriza AI-1–AI-7. Não há merge.

## Recorte implementado, ainda não validado operacionalmente

- A tela Busca mantém o mecanismo tradicional como padrão e ganha modos explícitos
  Expansão, Literal / emojis e Indicadores; digitar ou abrir o histórico não executa
  consultas auditadas. Reutiliza componentes e tokens atuais, sem redesign do shell.
- Expansão combina por OU o texto original e os termos habilitados pelo analista.
  Palavras usam o FTS existente; telefones usam dígitos nos identificadores;
  símbolos/emojis usam comparação literal. Não há vocabulário criminal pré-carregado.
- Literal preserva caixa e sequência Unicode, incluindo ZWJ/VS e modificadores.
  **Espaços nas bordas são desconsiderados**, conforme aviso da interface.
- Indicadores são candidatos sintáticos a IP, e-mail, URL HTTP(S), domínio, hash,
  telefone e coordenadas nos campos já importados. Não consulta DNS/rede nem lê
  conteúdo de arquivos. Domínios/e-mails ASCII e reconhecimento conservador podem
  gerar falsos positivos e falsos negativos; não se promete descoberta exaustiva.
- Resultados informam termos efetivos, mecanismo, motivo e origem do registro,
  com navegação para o contexto técnico existente. Nada vira evidência automaticamente.
- Vocabulários têm ramos habilitáveis, edição e importação/exportação JSON manual.
  A importação lê somente a configuração; autor/hash/versão externos não são
  aceitos como proveniência local. Salvar cria nova versão, nunca altera a anterior.
  Alternar um ramo exige salvar nova versão antes de pesquisar com ela.
- Tudo fica no **caso atual**. Perfis institucionais e revisão persistida de
  indicadores ainda não estão implementados. O histórico está na Busca, não na
  futura aba genérica de Consultas salvas.

## Persistência, contratos e invariantes

Migração **nova** `0007_pesquisa_local.sql`: `vocabulario_pesquisa` e
`consulta_descoberta`, derivados com triggers somente inclusão. Migrações 1–6
não foram editadas. O mecanismo existente faz backup antes de atualizar casos.
Fixtures de rollback antigas removem também as tabelas/registro v7 para simular
as versões históricas corretamente, exclusivamente em bancos temporários sintéticos.

Gravação/exportação do vocabulário e execução de consulta usam a auditoria existente,
na mesma transação IMMEDIATE da persistência. Detalhes desses eventos contêm IDs,
hashes, versão/mecanismo e contagens, não texto dos termos. Configuração sensível fica
no caso; não há novo log técnico de conteúdo, catálogo global de pesquisas ou acesso
irrestrito do frontend a disco, shell ou rede.

O SHA-256 do vocabulário inclui configuração, versão, data e autor; sua leitura
recalcula esse hash. Snapshot confere bytes serializados e metadados ao ler.
São hashes de **derivados**, sem mudar a regra histórica de `hash_registro`.
Consulta ligada a vocabulário exige exatamente os termos dos ramos habilitados
na versão referenciada. Parsers, Avilla, ingestão, fatos forenses, evidências,
relatórios, auditoria/âncoras existentes e extração original não foram alterados.

**Snapshot de configuração, não coleção congelada de resultados:** registra pedido,
termos, filtros, algoritmo `mdfml-ai0-local-v1`, representação FTS v3, máximos de IDs
e contagens. Paginação rejeita versão incompatível, novos IDs ou mudança em termos,
algoritmo/contagens. Isso não certifica igualdade integral da composição dos resultados
se alguém adulterar dados/índices fora das proteções mantendo os mesmos IDs/contagens.

Novos DTOs devem ser gerados oficialmente pelo `ts-rs`, nunca escritos manualmente.
A CI gera contratos do armazenamento antes do build da interface/instalador e oferece
artifact `sied-contratos-typescript`. A conferência mantém o bloqueio de contratos
alterados e agora também detecta contratos novos não versionados. O primeiro checkpoint
precisa incorporar esses arquivos gerados antes de ter essa conferência verde.

## Limites e riscos pendentes

- Até 8 ramos, 31 expansões e 32 termos efetivos, 200 caracteres por termo; páginas
  até 200 itens (UI: 50), até 64 candidatos exibidos por artefato. Totais de indicadores
  contam artefatos, não valores distintos ou todas as ocorrências.
- Indicadores e trechos literais varrem o escopo filtrado. Paginação limita resultados,
  **não o tempo total da consulta**. Não há deadline/cancelamento da pesquisa nesta
  entrega inicial; o serviço fica ocupado durante a chamada. Medir CPU/RAM/latência
  em volume antes de liberar; não apresentar essa implementação como motor escalável.
- Execução/paginação são recusadas durante importação ativa; não muda o cancelamento
  existente da ingestão. Não foram executadas migrações em casos reais ou no caso
  demonstrativo instalado; não abrir casos de produção com esta branch pendente.
- Filtros app/tipo/período excluem inventário de arquivos, que não tem esses atributos.
  Extração restringe ambos; o inventário pesquisa caminhos, não conteúdo de documentos.
- Sem OCR, PDFs/documentos, embeddings, GPU, modelo de IA, inferência de significado,
  correlação, conectores, painel institucional ou alteração dos módulos premium.
- Modelo de ameaças específico da fase tem armazenamento gerenciado pelo Codex Security,
  com revisão independente de arquitetura. Não é auditoria de segurança concluída.

## Validação do checkpoint

- `npm.cmd test`: 92/93 passaram; o teste existente de transição do Layout excedeu
  5 segundos. `npm.cmd test -- src/rotas/Layout.test.tsx`: 2/2 passaram, sem editar timeout.
- `npm.cmd test -- --maxWorkers=2`: execução final passou **96/96 testes em 16 arquivos**,
  incluindo regressões de edição durante gravação pendente e limpeza de status/JSON antigos.
- Novas regressões Rust sintéticas cobrem expansão/união, filtros e paginação,
  Unicode/emojis, destinatários JSON escapados, acentos NFD, entradas hostis,
  indicadores plausíveis, original/hash preservados, histórico, ramos/versionamento,
  proveniência e adulteração de derivados. **Não executaram localmente.**
- Tentativa de teste Rust interrompida pelo ambiente MSVC sem `link.exe`, antes
  de compilar/executar o código da fase. Não foi instalado toolchain, alterada
  política corporativa nem tentada compilação Tauri local.
- `npm.cmd run build`: bloqueado pelos novos contratos TypeScript ainda não gerados
  (e erros de tipagem derivados das importações faltantes). Build não aprovado.
- `cargo fmt --all --check` e `git diff --check`: aprovados. CI consultada no máximo
  uma vez, sem polling, sleeps ou espera por instaladores.

Antes de concluir: incorporar contratos realmente gerados, executar testes Rust e
Clippy nas plataformas suportadas, repetir build/interface, medir desempenho e FP/FN,
validar manualmente com caso **SINTÉTICO** pelo fluxo oficial e revisar os riscos acima.
Não iniciar a próxima fase ou fazer merge automaticamente.
