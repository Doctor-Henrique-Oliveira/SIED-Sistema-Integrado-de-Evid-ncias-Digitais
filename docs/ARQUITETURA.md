# SIED — Proposta de arquitetura (v0.1, para aprovação)

Data: 28/09/2026 · Status: **aprovada com ajustes em 28/09/2026** · Visão de referência: [`VISAO.md`](VISAO.md)

Este documento respondeu aos 9 pontos pedidos antes da implementação. Cada decisão traz o motivo, para que possa ser contestada.

## Ajustes aprovados (28/09/2026)

Estes ajustes prevalecem sobre o texto original quando houver diferença.

1. **Stack aprovada**: Tauri, React, TypeScript, Rust (núcleo e parsers), SQLite, FTS5, Typst.
   O sistema de importadores/parsers deve permitir, no futuro, parsers em Python, Java ou outras
   linguagens, sem introduzir essas dependências agora. Implementado como contrato: entrada e saída
   de parser são serializáveis e há um protocolo JSON por linha definido em
   `crates/sied-ingestao/src/protocolo.rs` para parsers em processo separado.
2. **Bancos**: um banco **global** pequeno (catálogo de casos, configurações e, futuramente,
   usuários) e um banco SQLite **autocontido por caso**, cada um com versão de esquema e
   migrações próprias (`crates/sied-armazenamento/migracoes/`). Casos e dispositivos usam UUID v7,
   para que bancos de casos diferentes possam ser anexados e correlacionados sem colisão de chaves.
3. **Importer ≠ Parser** é princípio fundamental. O parser (ex.: WhatsApp) processa os dados
   independentemente de terem vindo do Avilla, IPED ou outra fonte.
4. **Rastreabilidade**: nenhum banco da evidência é aberto de forma que possa ser alterado; só
   cópias de trabalho verificadas. Preservar caminho original, ferramenta, arquivo, hash, banco,
   tabela, ID do registro, valores brutos, parser e versão. Gerar manifesto da importação.
5. **Windows 11** como alvo principal, mantendo Windows 10 quando possível. O instalador embute o
   instalador offline do WebView2 (estações sem internet).
6. **GitHub**: usar exclusivamente o repositório privado SIED já existente.
7. **Avilla**: não presumir a estrutura. Até receber uma extração real, o `AvillaImporter` existe só
   como contrato; testes usam fixtures sintéticas identificadas como tal.
8. **Fase 1 incremental**: primeiro a fundação (workspace, Tauri, React/TS, comunicação, banco global,
   banco por caso, migrações, Caso, Dispositivo, contratos de Artefato/Importer/Parser, logs, erros).

---

## 1. Análise da visão

O SIED não é um visualizador nem uma ferramenta de aquisição. É a **camada de análise** que fica depois das ferramentas de extração:

```
Avilla / IPED / ADB / ALEAPP / iLEAPP / pasta
        │  (importadores: entendem o formato de cada ferramenta)
        ▼
  Inventário + hashes  ──►  Parsers por aplicativo  ──►  Modelo universal de artefatos
                                                               │
                         Índice de busca ◄─────────────────────┤
                                                               ▼
      UI: conversas, contatos, chamadas, mídia, mapa, linha do tempo, busca
                                                               │
                                     Evidências selecionadas ◄─┘
                                                               ▼
                                                     Relatório PDF + auditoria
```

Quatro requisitos definem a arquitetura mais que todos os outros:

| Requisito | Consequência técnica |
|---|---|
| **Rastreabilidade total** (todo dado aponta para arquivo, banco, tabela e registro de origem) | A referência de origem é parte obrigatória do modelo, não um campo opcional. Um parser não consegue gravar um artefato sem ela (ver §7). |
| **Original intocado** | Acesso à extração só por uma camada de leitura que não tem métodos de escrita. Bancos SQLite nunca são abertos no lugar; abrimos uma cópia verificada por hash. |
| **Milhões de artefatos sem travar** | Tudo paginado no banco; nada é carregado inteiro na memória; processamento em jobs no backend Rust; listas virtualizadas na UI. |
| **Independência de ferramenta** | Separar *importador* (conhece a ferramenta) de *parser* (conhece o aplicativo). O parser do WhatsApp é o mesmo para Avilla, ADB ou pasta genérica. |

Pontos de atenção identificados na visão:

- **Estrutura do Avilla não é documentada publicamente.** O README do projeto (github.com/AvillaDaniel/AvillaForensics) cita backups `.ab`, arquivos `.tar`, dumps de apps e logs cifrados `.avilla`, mas não descreve a árvore de saída. O importador Avilla depende de uma amostra (ver "Pendências" no fim).
- **"Calcular hashes quando apropriado"**: proponho hash SHA-256 de **todos** os arquivos por padrão (em disco local SSD, cerca de 9 min para 256 GB), com opção de adiar para depois. Arquivos que viram artefato ou evidência são sempre hasheados antes de entrar no relatório.
- **Mapa offline**: mapa sem internet exige um pacote de mapas local (arquivo PMTiles do Brasil ou do estado). Isso é um item de instalação na delegacia, não só código.
- **Signal e Telegram**: o Signal guarda o banco cifrado; só será possível quando a extração trouxer a chave ou o banco já decifrado. O Telegram (`cache4.db`) guarda mensagens serializadas em formato binário próprio. Ambos ficam para depois do WhatsApp.

## 2. Repositório atual

Não existe repositório ainda. A pasta do projeto contém só esta documentação (`docs/VISAO.md` e `docs/ARQUITETURA.md`). Proposta: criar um repositório **privado** no GitHub quando a arquitetura for aprovada, com `.gitignore` bloqueando casos, bancos, extrações e `.env` desde o primeiro commit.

## 3. Arquitetura proposta

### 3.1 Visão geral

```
┌──────────────────────────── Aplicativo desktop (Tauri 2) ────────────────────────────┐
│  UI  React + TypeScript (WebView)                                                     │
│    telas · componentes (ChatBolha, PainelTecnico, ListaVirtual...) · cliente tipado  │
│                     │ comandos tipados (invoke)        ▲ eventos de progresso         │
│                     ▼                                  │ protocolo sied:// (mídia)    │
│  Camada Tauri (fina): valida entrada, checa permissão, chama os serviços             │
├───────────────────────────────────── Rust ───────────────────────────────────────────┤
│  sied-ingestao      fontes somente leitura · importadores · parsers · fila de jobs    │
│  sied-armazenamento SQLite + FTS5 · migrações · repositórios · auditoria encadeada   │
│  sied-relatorio     modelos Typst → PDF                                              │
│  sied-dominio       tipos do modelo universal (sem E/S)                               │
└──────────────────────────────────────────────────────────────────────────────────────┘
   Extração (somente leitura)                      Pasta do caso (leitura e escrita)
   D:\Extracoes\...                                 E:\SIED\Casos\IP-123-2026\
```

### 3.2 Escolhas e motivos

| Camada | Escolha | Por quê | Alternativa descartada |
|---|---|---|---|
| Desktop | **Tauri 2** | Binário pequeno, usa o WebView2 do Windows, backend em Rust com acesso direto ao disco. Sem Node no runtime. | Electron: ~150 MB, Node com acesso amplo ao sistema. |
| UI | **React + TypeScript + Vite** | Pedido do usuário; ecossistema maduro. | — |
| Componentes UI | **Radix UI** (acessível, sem estilo) + **Tailwind CSS**; **TanStack Query** (cache e paginação de dados), **TanStack Virtual** (listas com milhões de itens), **React Router** | Bibliotecas estáveis, pequenas e sem dependência de rede. | Frameworks pesados de UI (MUI) dificultam o visual "app de mensagens". |
| Backend e parsers | **Rust** | Um único executável offline, rápido para hash e varredura de milhões de arquivos, tipagem forte, sem runtime Python para instalar em cada máquina. | Python: ótimo para parsers, mas exige empacotar interpretador e bibliotecas; ALEAPP/iLEAPP serão lidos pelo que **produzem** (TSV/SQLite), sem executar Python. Um protocolo de parser externo (processo separado, JSON) fica previsto para a fase 5 se algo só existir em Python. |
| Banco | **SQLite**, um arquivo por caso | Transacional, arquivo único (fácil de arquivar e entregar), suporta centenas de GB, sem servidor. Um banco por caso isola casos e facilita bloqueio de acesso. | PostgreSQL: exige serviço instalado; faz sentido só na edição servidor. |
| Busca | **SQLite FTS5** com tokenizador `unicode61 remove_diacritics` | Busca por palavra sem acento, prefixo e frase, no mesmo arquivo e na mesma transação dos dados. Aguenta milhões de linhas. A busca fica atrás de uma interface, então trocar por Tantivy é possível se um dia for gargalo. | Elasticsearch/OpenSearch: servidor Java, pesado para estação offline. |
| Relatório | **Typst** embutido como biblioteca Rust | Layout de alta qualidade (bolhas de chat, imagens com legenda, tabelas), resultado determinístico, gera PDF/A, roda 100% offline e sem navegador. | Imprimir HTML pelo WebView: resultado varia por máquina. Chromium embutido: +150 MB. Bibliotecas de PDF de baixo nível: relatório "tabela feia". |
| Mapa (fase 3) | **MapLibre GL** + arquivo **PMTiles** local | Mapa vetorial rápido lendo um arquivo no disco, sem internet. | Leaflet com tiles online: viola o requisito offline. |
| Tipos Rust ↔ TS | **tauri-specta** | Gera os tipos TypeScript a partir do Rust; UI e backend não divergem. | Tipos escritos à mão nos dois lados. |

### 3.3 Princípios de execução

- **Jobs em background**: importação, hash, parsing, miniaturas e relatório rodam numa fila de tarefas em threads do Rust. A tabela `tarefa` guarda o estado, então uma importação interrompida retoma do ponto onde parou (inventário é idempotente por caminho). A UI recebe progresso por eventos e nunca espera uma chamada longa.
- **Paginação por cursor** em todas as listas (conversas, mensagens, galeria, linha do tempo), sempre por índice (`data_hora_ms, id`). Nenhuma tela carrega uma tabela inteira.
- **Mídia servida por protocolo próprio** `sied://arquivo/<id>` e `sied://miniatura/<sha256>`: o backend confere o id no banco, abre o original só para leitura e entrega com suporte a *range* (necessário para avançar áudio e vídeo). A UI nunca recebe caminhos do disco.
- **Sem rede**: o app não inclui plugin HTTP, a política de segurança do WebView (`CSP`) permite só `self` e `sied:`, sem atualizador automático e sem telemetria.

## 4. Estrutura de pastas

### 4.1 Repositório

```
sied/
├─ README.md
├─ docs/
│  ├─ VISAO.md                  visão permanente (texto do usuário)
│  ├─ ARQUITETURA.md            este documento
│  ├─ adr/                      decisões arquiteturais numeradas (0001-sqlite-por-caso.md ...)
│  └─ formatos/avilla.md        estrutura documentada das extrações Avilla
├─ crates/
│  ├─ sied-dominio/             tipos do modelo universal, enums, validações (sem E/S)
│  ├─ sied-armazenamento/       conexão, migrações SQL, repositórios, FTS, auditoria
│  │  └─ migracoes/0001_inicial.sql ...
│  ├─ sied-ingestao/
│  │  ├─ fontes/                leitura somente leitura: pasta, tar, zip, backup .ab
│  │  ├─ inventario/            varredura, classificação por assinatura, hash, manifesto
│  │  ├─ importadores/          avilla, generico (depois iped, adb, aleapp, ileapp)
│  │  ├─ parsers/               whatsapp_android, android_contatos, android_chamadas, android_sms, midia_exif ...
│  │  └─ tarefas/               fila de jobs, progresso, cancelamento, retomada
│  ├─ sied-relatorio/           montagem do relatório + modelos Typst + fontes embutidas
│  └─ sied-cli/                 linha de comando: importar, verificar hashes, verificar auditoria
├─ app/
│  ├─ src-tauri/                camada fina: comandos, eventos, protocolo sied://, permissões
│  └─ src/                      React
│     ├─ rotas/                 uma pasta por área (visao-geral, conversas, contatos, ...)
│     ├─ componentes/           ChatBolha, SeparadorDia, PainelTecnico, ListaVirtual, CartaoEvidencia ...
│     ├─ api/                   cliente gerado (tauri-specta) + hooks TanStack Query
│     └─ util/                  datas e fuso, números de telefone, formatação
├─ testes/
│  ├─ gerador/                  cria extrações SINTÉTICAS (bancos WhatsApp falsos etc.)
│  └─ fixtures/                 somente dados sintéticos, nunca reais
└─ .gitignore                   bloqueia casos/, *.db, extrações, .env, relatórios
```

Motivo de 4 crates e não 15: cada crate corresponde a uma fronteira que a visão pede (domínio, armazenamento, parsers, relatório, UI). Dentro de cada um, a organização é por módulo. Cresce por arquivos, não por novos crates.

### 4.2 Pasta de um caso (área de trabalho, separada da extração)

```
E:\SIED\Casos\IP-0123-2026\
├─ caso.sied.db              banco da análise (artefatos, índice, evidências, auditoria)
├─ cache\
│  ├─ bancos\<sha256>\       cópias verificadas dos SQLite da extração (+ -wal, -shm)
│  ├─ miniaturas\<sha256>.webp
│  └─ derivados\             previews, futuros OCR e transcrições
├─ importacoes\<id>\
│  ├─ manifesto.sha256       formato sha256sum: verificável com ferramentas externas
│  └─ log-importacao.jsonl   log detalhado, incluindo erros de parsing
└─ relatorios\
   ├─ 2026-09-30_relatorio.pdf
   └─ 2026-09-30_relatorio.pdf.sha256
```

O SIED recusa criar a pasta do caso dentro da pasta da extração (e vice-versa).

## 5. Modelo inicial do banco de dados

Um banco por caso. Um banco pequeno do aplicativo (`sied-app.db`) guarda usuários, lista de casos conhecidos e configurações.

### 5.1 Diagrama

```
caso 1─N dispositivo 1─N extracao 1─N arquivo
                               │            ▲ ▲
                               │            │ └──────────── referencia_origem N─1 artefato
                               1─N execucao_parser 1─N artefato ─┐
                                                                  ├─ N─1 conversa N─N pessoa
                                                                  └─ 1─0..1 evidencia
auditoria (encadeada)   tarefa   relatorio   derivado(arquivo → miniatura/preview)
```

### 5.2 Tabelas (SQL resumido)

```sql
-- Caso / procedimento
CREATE TABLE caso (
  id INTEGER PRIMARY KEY CHECK (id = 1),       -- um caso por arquivo
  procedimento TEXT, numero_procedimento TEXT, numero_bo TEXT,
  investigado TEXT, vitima TEXT, delegado TEXT,
  responsavel_analise TEXT NOT NULL, unidade TEXT NOT NULL,
  observacoes TEXT, fuso_horario TEXT NOT NULL DEFAULT 'America/Sao_Paulo',
  criado_em TEXT NOT NULL
);

CREATE TABLE dispositivo (
  id INTEGER PRIMARY KEY,
  apelido TEXT NOT NULL,                        -- "Celular 1 – investigado"
  fabricante TEXT, modelo TEXT, nome_comercial TEXT,
  sistema TEXT, versao_sistema TEXT,            -- android / ios
  imei1 TEXT, imei2 TEXT, serial TEXT, numero TEXT, operadora TEXT,
  capacidade_bytes INTEGER, observacoes TEXT,
  criado_em TEXT NOT NULL
);
-- Valores detectados automaticamente, com a fonte de cada um.
-- O analista confirma e copia para `dispositivo`; o detectado nunca é sobrescrito.
CREATE TABLE dispositivo_atributo (
  id INTEGER PRIMARY KEY,
  dispositivo_id INTEGER NOT NULL REFERENCES dispositivo(id),
  chave TEXT NOT NULL,                          -- imei1, modelo, conta_google ...
  valor TEXT NOT NULL,
  arquivo_id INTEGER REFERENCES arquivo(id),    -- ex.: build.prop
  execucao_parser_id INTEGER REFERENCES execucao_parser(id)
);

CREATE TABLE extracao (
  id INTEGER PRIMARY KEY,
  dispositivo_id INTEGER NOT NULL REFERENCES dispositivo(id),
  ferramenta TEXT NOT NULL,                     -- avilla, iped, adb, aleapp, generico
  ferramenta_versao TEXT,
  tipo_extracao TEXT,                           -- backup_adb, logica, sistema_arquivos ...
  data_aquisicao TEXT,                          -- informada pelo analista ou detectada
  origem_caminho TEXT NOT NULL,                 -- caminho absoluto no momento da importação
  origem_volume TEXT,                           -- rótulo/serial do volume
  hash_declarado TEXT,                          -- hash informado no laudo de aquisição
  hash_declarado_confere INTEGER,
  importador TEXT NOT NULL, importador_versao TEXT NOT NULL,
  sied_versao TEXT NOT NULL,
  usuario TEXT NOT NULL, maquina TEXT NOT NULL,
  iniciada_em TEXT NOT NULL, concluida_em TEXT,
  total_arquivos INTEGER, total_bytes INTEGER,
  hash_manifesto TEXT,                          -- SHA-256 do manifesto.sha256
  status TEXT NOT NULL                          -- inventariando, processando, concluida, falhou, cancelada
);

-- Inventário: um registro por arquivo da extração (inclusive dentro de .tar/.ab/.zip)
CREATE TABLE arquivo (
  id INTEGER PRIMARY KEY,
  extracao_id INTEGER NOT NULL REFERENCES extracao(id),
  caminho TEXT NOT NULL,                        -- relativo à raiz; contêiner com '!' → "backup.ab!/apps/com.whatsapp/db/msgstore.db"
  nome TEXT NOT NULL, extensao TEXT,
  tamanho INTEGER NOT NULL,
  modificado_em TEXT, criado_em TEXT, acessado_em TEXT,   -- como gravados na extração
  sha256 TEXT, md5 TEXT,
  assinatura TEXT,                              -- tipo real pelo conteúdo (magic bytes)
  categoria TEXT NOT NULL,                      -- imagem, video, audio, documento, banco_sqlite, ...
  status TEXT NOT NULL,                         -- ok, erro_leitura, hash_pendente
  erro TEXT,
  UNIQUE (extracao_id, caminho)
);

-- Cada vez que um parser roda sobre um arquivo
CREATE TABLE execucao_parser (
  id INTEGER PRIMARY KEY,
  extracao_id INTEGER NOT NULL REFERENCES extracao(id),
  parser TEXT NOT NULL, parser_versao TEXT NOT NULL,
  arquivo_id INTEGER REFERENCES arquivo(id),    -- entrada principal
  copia_sha256 TEXT,                            -- hash da cópia efetivamente lida (= original)
  iniciada_em TEXT NOT NULL, concluida_em TEXT,
  status TEXT NOT NULL,                         -- ok, parcial, falhou
  artefatos INTEGER, avisos INTEGER, erros_json TEXT,
  substituida_por INTEGER REFERENCES execucao_parser(id)  -- reprocessamento
);

-- Pessoas/identidades vistas nos apps (normaliza remetentes e contatos)
CREATE TABLE pessoa (
  id INTEGER PRIMARY KEY,
  app TEXT NOT NULL, identificador TEXT NOT NULL,   -- jid, telefone E.164, e-mail
  nome_exibicao TEXT, telefone_normalizado TEXT,
  foto_arquivo_id INTEGER REFERENCES arquivo(id),
  UNIQUE (app, identificador)
);

CREATE TABLE conversa (
  id INTEGER PRIMARY KEY,
  extracao_id INTEGER NOT NULL REFERENCES extracao(id),
  app TEXT NOT NULL, id_externo TEXT NOT NULL,
  titulo TEXT, tipo TEXT NOT NULL,              -- individual, grupo, transmissao, sms
  foto_arquivo_id INTEGER REFERENCES arquivo(id),
  total_mensagens INTEGER, primeira_ms INTEGER, ultima_ms INTEGER,
  UNIQUE (extracao_id, app, id_externo)
);
CREATE TABLE conversa_participante (
  conversa_id INTEGER NOT NULL REFERENCES conversa(id),
  pessoa_id INTEGER NOT NULL REFERENCES pessoa(id),
  papel TEXT,                                   -- admin, membro, ex_membro
  PRIMARY KEY (conversa_id, pessoa_id)
);

-- MODELO UNIVERSAL DE ARTEFATOS
CREATE TABLE artefato (
  id INTEGER PRIMARY KEY,
  extracao_id INTEGER NOT NULL REFERENCES extracao(id),
  dispositivo_id INTEGER NOT NULL REFERENCES dispositivo(id),
  execucao_parser_id INTEGER NOT NULL REFERENCES execucao_parser(id),
  tipo TEXT NOT NULL,        -- mensagem, contato, chamada, midia, documento, localizacao, evento_navegador, conta, arquivo ...
  subtipo TEXT,              -- texto, imagem, audio, video, documento, localizacao, contato_compartilhado, sistema ...
  app TEXT,                  -- whatsapp, telegram, sms, android_telefone ...
  conta TEXT,                -- conta local do app, quando houver
  conversa_id INTEGER REFERENCES conversa(id),
  remetente_id INTEGER REFERENCES pessoa(id),
  destinatarios TEXT,        -- JSON [pessoa_id...]
  de_mim INTEGER,            -- 1 enviado pelo titular do aparelho
  data_hora_ms INTEGER,      -- UTC em ms (ordenação e filtros)
  data_hora_original TEXT,   -- valor bruto como estava na fonte
  formato_data TEXT,         -- unix_ms, unix_s, webkit, cocoa, texto
  conteudo TEXT,
  resposta_a_id INTEGER REFERENCES artefato(id),
  encaminhada INTEGER,
  anexo_arquivo_id INTEGER REFERENCES arquivo(id),
  mime TEXT, nome_arquivo TEXT, tamanho INTEGER, duracao_s INTEGER,
  latitude REAL, longitude REAL, precisao_m REAL,
  estado_registro TEXT NOT NULL DEFAULT 'ativo',  -- ativo, apagado, recuperado_wal, desconhecido
  confianca TEXT NOT NULL DEFAULT 'alta',          -- alta, media, baixa (+ motivo em metadados)
  metadados TEXT,            -- JSON com campos específicos do app
  hash_registro TEXT NOT NULL -- SHA-256 do artefato normalizado (canônico)
);
CREATE INDEX idx_art_conversa ON artefato(conversa_id, data_hora_ms, id);
CREATE INDEX idx_art_tempo    ON artefato(data_hora_ms, id);
CREATE INDEX idx_art_tipo     ON artefato(tipo, subtipo, data_hora_ms);
CREATE INDEX idx_art_geo      ON artefato(latitude, longitude) WHERE latitude IS NOT NULL;

-- REFERÊNCIA À FONTE ORIGINAL (ver §7). Um artefato tem 1..N referências.
CREATE TABLE referencia_origem (
  id INTEGER PRIMARY KEY,
  artefato_id INTEGER NOT NULL REFERENCES artefato(id),
  papel TEXT NOT NULL,           -- registro, anexo, miniatura_app, contato_resolvido, wal
  arquivo_id INTEGER NOT NULL REFERENCES arquivo(id),
  arquivo_sha256 TEXT NOT NULL,  -- hash do arquivo no momento da leitura
  tabela TEXT,                   -- ex.: message
  chave_registro TEXT,           -- ex.: _id=48213 (ou rowid)
  consulta TEXT,                 -- SQL usada para ler o registro
  deslocamento INTEGER,          -- para dados recuperados em páginas/WAL
  valores_brutos TEXT            -- JSON: a linha original, coluna a coluna
);
CREATE INDEX idx_ref_artefato ON referencia_origem(artefato_id);
CREATE INDEX idx_ref_arquivo  ON referencia_origem(arquivo_id);

-- Busca global (índice sobre artefatos e sobre nomes de arquivos)
CREATE VIRTUAL TABLE busca_artefato USING fts5(
  conteudo, remetente, destinatarios, nome_arquivo, metadados_texto, telefones,
  content='', tokenize='unicode61 remove_diacritics 2'
);
CREATE VIRTUAL TABLE busca_arquivo USING fts5(caminho, nome, tokenize='unicode61 remove_diacritics 2');

-- Evidências selecionadas
CREATE TABLE evidencia (
  id INTEGER PRIMARY KEY,
  artefato_id INTEGER REFERENCES artefato(id),     -- item único
  conversa_id INTEGER REFERENCES conversa(id),     -- ou trecho de conversa:
  trecho_inicio_id INTEGER REFERENCES artefato(id),
  trecho_fim_id INTEGER REFERENCES artefato(id),
  arquivo_id INTEGER REFERENCES arquivo(id),       -- ou arquivo sem artefato
  categoria TEXT, ordem INTEGER NOT NULL,
  titulo TEXT, observacao TEXT, descricao_contextual TEXT,
  hash_no_momento TEXT NOT NULL,   -- hash_registro/sha256 quando marcado
  criada_por TEXT NOT NULL, criada_em TEXT NOT NULL,
  removida_em TEXT, removida_por TEXT              -- remoção lógica, histórico preservado
);

CREATE TABLE relatorio (
  id INTEGER PRIMARY KEY,
  modelo TEXT NOT NULL, secoes TEXT NOT NULL,      -- JSON das seções escolhidas
  gerado_por TEXT NOT NULL, gerado_em TEXT NOT NULL,
  caminho TEXT NOT NULL, sha256 TEXT NOT NULL,
  sied_versao TEXT NOT NULL, auditoria_ate_seq INTEGER NOT NULL
);

CREATE TABLE derivado (           -- tudo que o SIED gera a partir de um arquivo
  id INTEGER PRIMARY KEY,
  arquivo_id INTEGER NOT NULL REFERENCES arquivo(id),
  tipo TEXT NOT NULL,             -- miniatura, preview, exif, ocr, transcricao
  caminho_cache TEXT, conteudo TEXT, sha256 TEXT,
  gerador TEXT NOT NULL, gerador_versao TEXT NOT NULL, gerado_em TEXT NOT NULL
);

CREATE TABLE tarefa (
  id INTEGER PRIMARY KEY, tipo TEXT NOT NULL, parametros TEXT NOT NULL,
  estado TEXT NOT NULL, progresso REAL, mensagem TEXT,
  criada_em TEXT NOT NULL, iniciada_em TEXT, concluida_em TEXT, erro TEXT
);

-- Auditoria encadeada: cada registro contém o hash do anterior
CREATE TABLE auditoria (
  seq INTEGER PRIMARY KEY, ts TEXT NOT NULL,
  usuario TEXT NOT NULL, maquina TEXT NOT NULL,
  acao TEXT NOT NULL,             -- caso_criado, importacao_iniciada, parser_executado, evidencia_adicionada, relatorio_gerado, exportacao ...
  detalhes TEXT NOT NULL,         -- JSON
  hash_anterior TEXT NOT NULL, hash TEXT NOT NULL
);
-- Gatilhos impedem UPDATE/DELETE em auditoria, arquivo (hashes) e referencia_origem.
```

Decisões do modelo:

- **Datas**: `data_hora_ms` (UTC, inteiro) para ordenar e filtrar; `data_hora_original` + `formato_data` preservam o valor bruto; o fuso de exibição é do caso (padrão `America/Sao_Paulo`) e o relatório declara qual fuso usou.
- **Telefones**: guardamos também a forma normalizada (E.164, só dígitos) na busca, para que "11 98765-4321" encontre "+5511987654321".
- **Metadados específicos** de cada app vão em `metadados` (JSON), para o modelo universal não crescer uma coluna por aplicativo.
- **Evidência nunca é apagada fisicamente**: remoção lógica, para a auditoria continuar coerente.

## 6. Sistema de importadores e parsers

Dois papéis separados:

| | Importador (um por ferramenta) | Parser (um por aplicativo/artefato) |
|---|---|---|
| Sabe | como a ferramenta organiza a saída | como o app guarda os dados |
| Exemplos | Avilla, IPED, ADB, ALEAPP, iLEAPP, Pasta genérica | WhatsApp Android, Contatos Android, Chamadas, SMS, EXIF de mídia |
| Faz | detecta a ferramenta, abre contêineres, extrai dados do aparelho, diz "este arquivo é o msgstore.db do WhatsApp" | lê o arquivo (cópia verificada) e emite artefatos normalizados |

Fluxo de uma importação (cada etapa é uma tarefa retomável):

```
1. Detectar     cada importador dá uma nota de confiança para a pasta; o usuário confirma
2. Inventariar  varre tudo pela camada de leitura, classifica por assinatura, grava `arquivo`
3. Hash         SHA-256 de cada arquivo (paralelo); gera manifesto.sha256 e hash_manifesto
4. Planejar     importador associa arquivos a parsers (por caminho conhecido E por conteúdo:
                ex. banco com tabelas `message` e `jid` → WhatsApp, mesmo se renomeado)
5. Processar    cada parser roda numa `execucao_parser`; erros viram registros, não interrompem
6. Indexar      popula a busca; gera miniaturas das imagens
7. Concluir     registra estatísticas, erros e hash final na auditoria
```

Interfaces (Rust, simplificadas):

```rust
pub trait Importador {
    fn id(&self) -> &'static str;                 // "avilla"
    fn versao(&self) -> &'static str;
    fn detectar(&self, fonte: &dyn Fonte) -> Deteccao;          // confiança 0..1 + motivos
    fn planejar(&self, ctx: &ContextoImportacao) -> Result<Plano>; // arquivo → parser, dados do aparelho
}

pub trait Parser {
    fn id(&self) -> &'static str;                 // "whatsapp_android"
    fn versao(&self) -> &'static str;
    fn reconhece(&self, arq: &ArquivoInventariado, amostra: &Amostra) -> bool;
    fn processar(&self, entrada: &EntradaParser, saida: &mut SaidaArtefatos) -> Result<Resumo>;
}

pub trait Fonte {                                  // SOMENTE LEITURA: não há métodos de escrita
    fn listar(&self) -> Box<dyn Iterator<Item = Result<EntradaFonte>>>;
    fn abrir(&self, caminho: &CaminhoFonte) -> Result<Box<dyn Read + Seek>>;
}
```

- `SaidaArtefatos` grava em lotes (transações de milhares de linhas) e **exige** pelo menos uma `ReferenciaOrigem` por artefato; o tipo `NovoArtefato` não compila sem ela.
- Importadores e parsers ficam registrados numa lista no código (compilados). Sistema de plugins externos fica para a fase 5, quando houver necessidade real; antes disso só adiciona complexidade.
- Parsers são independentes de ferramenta: o mesmo `whatsapp_android` serve Avilla, ADB e pasta genérica.
- Importadores de ferramentas que já processam dados (IPED, ALEAPP) podem emitir artefatos direto do banco/relatório da ferramenta; nesse caso a referência aponta para o arquivo da ferramenta **e**, quando ela informa, para o arquivo original do aparelho.
- **Bancos SQLite**: o parser nunca abre o original. O arquivo e seus `-wal`/`-shm`/`-journal` são copiados para `cache/bancos/<sha256>/`, o hash da cópia é conferido com o original e só então a cópia é aberta. Isso também evita que o SQLite faça checkpoint do WAL no original.

## 7. Referência entre artefato normalizado e fonte original

Cadeia completa, do que o investigador vê até o bit no aparelho:

```
Evidência nº 3 (relatório)
  └─ artefato #48213  mensagem WhatsApp, 14/09/2026 14:31:07 (UTC-3), hash_registro 9f2c…
       ├─ execucao_parser #12  whatsapp_android v1.0.0, lida de cópia sha256 a41b… (= original)
       └─ referencia_origem
            ├─ papel=registro  arquivo #1882  "backup.ab!/apps/com.whatsapp/db/msgstore.db"
            │                  sha256 a41b…  tabela=message  chave=_id=48213
            │                  consulta="SELECT … FROM message WHERE _id=48213"
            │                  valores_brutos={"_id":48213,"key_id":"3EB0…","timestamp":1757871067000,…}
            └─ papel=anexo     arquivo #9921  "…/Media/WhatsApp Images/IMG-20260914-WA0003.jpg"  sha256 77d0…
                 └─ arquivo pertence à extracao #1 (Avilla, D:\Extracoes\Cel1, manifesto 5e8a…)
```

Garantias:

1. **Endereço estável**: `arquivo.caminho` é relativo à raiz da extração e inclui o caminho dentro do contêiner (`!`), então continua válido se a extração mudar de disco.
2. **Conteúdo verificável**: cada referência grava o SHA-256 do arquivo no momento da leitura. A qualquer momento, "Verificar integridade" recalcula e compara.
3. **Registro reproduzível**: tabela + chave + consulta + valores brutos permitem que um perito abra o banco original com qualquer ferramenta SQLite e encontre a mesma linha.
4. **Artefato imutável**: `hash_registro` (SHA-256 do artefato em JSON canônico) é gravado no relatório; se reprocessar gerar outro valor, a diferença aparece.
5. **Reprocessamento não quebra evidências**: uma nova versão do parser cria nova `execucao_parser`; artefatos antigos ligados a evidências são mantidos e marcados como substituídos, nunca apagados.
6. **Auditoria encadeada**: toda ação relevante gera registro com hash do anterior; o relatório imprime o último hash da cadeia (`auditoria_ate_seq`).

Na UI isso vira o painel **Detalhes técnicos** de qualquer item: origem, caminho, banco, tabela, ID do registro, timestamps (bruto e convertido), remetente/destinatário, hashes, parser e versão, ferramenta de origem e os valores brutos. O relatório traz as mesmas informações de forma resumida e um apêndice técnico completo.

## 8. Fases de desenvolvimento

| Fase | Objetivo | Conteúdo principal |
|---|---|---|
| **0 · Fundação** | base do projeto | repositório privado, CI (lint, testes), gerador de extrações sintéticas, estrutura documentada do Avilla a partir da amostra |
| **1 · Ciclo completo com Avilla** | importar → analisar → relatório | detalhada na §9 |
| **2 · Análise ampla** | explorar tudo da extração | linha do tempo unificada, galeria completa (vídeo, áudio, EXIF), documentos, navegador de arquivos, SMS dedicado, verificação de integridade por re-hash, relatório com escolha de seções, login local com perfis e permissões, registros apagados recuperados do WAL |
| **3 · IPED + localização** | segunda ferramenta e mapa | importador IPED, mapa offline (MapLibre + PMTiles), localizações de fotos (GPS do EXIF), Google/Maps, histórico do navegador, Telegram |
| **4 · Outras fontes** | ampliar entrada | importadores ADB, ALEAPP, iLEAPP; iOS; Signal quando houver banco decifrado |
| **5 · Inteligência** | correlação | vínculos entre pessoas, duplicados, comparação entre dispositivos, cruzamento entre casos, OCR e transcrição locais, plugins externos, edição servidor para rede interna |

## 9. O que entra na fase 1

Critério de pronto: **com uma extração Avilla real (ou de teste), o analista importa, encontra uma mensagem pela busca, abre a conversa reconstruída, marca 3 itens como evidência e gera um PDF cujos hashes conferem com o manifesto; a auditoria verifica íntegra.**

Backend:

- Casos (todos os campos da §5 da visão) e dispositivos (cadastro manual + atributos detectados com fonte).
- Camada de leitura: pasta; `.tar` e `.ab` sem senha se a amostra do Avilla mostrar que são o formato principal.
- Importadores: **Avilla** e **Pasta genérica** (este serve de fallback e de teste).
- Inventário completo com classificação por assinatura, SHA-256 de todos os arquivos, manifesto `sha256sum`, detecção de hash declarado.
- Parsers: **WhatsApp Android** (conversas, grupos e participantes, mensagens, respostas, encaminhadas, mídias vinculadas, localizações e contatos compartilhados, nomes via `wa.db`), **Contatos Android**, **Chamadas**, **SMS**, mídia (inventário e miniaturas de imagem).
- Busca global FTS5 com filtros: período, app, tipo, conversa/contato, extração, somente mídia, somente evidências.
- Jobs em background com progresso, cancelamento e retomada; log de erros de parsing por extração.
- Auditoria encadeada com verificação.
- Relatório PDF (Typst): capa, identificação do procedimento, responsável, dispositivo, aquisição e hashes, metodologia, evidências (mensagens e trechos no formato de chat, imagens com legenda, chamadas e contatos), observações, conclusão do analista, apêndice técnico com todas as referências de origem.

Interface:

- Visão geral do caso e do dispositivo (contagens por tipo, período coberto, apps encontrados).
- Assistente Novo caso → Adicionar dispositivo → Importar extração, com progresso.
- **Conversas**: lista (nome/foto, última mensagem), chat reconstruído com lados, horários, separadores por dia, respostas, encaminhadas, mídias, áudios, documentos, localizações; alternância Visão normal / Detalhes técnicos.
- Contatos e Chamadas.
- Busca global com resultados agrupados por tipo.
- Evidências selecionadas: adicionar de qualquer tela, reordenar, categorizar, título, observação, descrição, remover, "ver na origem".
- Auditoria (consulta e verificação).
- Identificação do analista no início da sessão (nome + usuário do Windows) gravada na auditoria. Login com senha e permissões ficam na fase 2, mas as tabelas já nascem preparadas.

Fica **fora** da fase 1, de propósito: linha do tempo, galeria avançada, mapa, IPED, Telegram/Signal, login com senha, OCR/transcrição.

Testes da fase 1: hashing e manifesto, cadeia de auditoria (incluindo detecção de adulteração), cada parser contra bancos sintéticos gerados em teste (esquemas antigo e novo do WhatsApp), garantia de que a pasta de origem não muda (hash e datas antes/depois da importação), geração do relatório.

---

## Pendências para começar

1. **Amostra da estrutura do Avilla** (bloqueia o importador Avilla, não o resto): a listagem de pastas de uma extração, de preferência de um aparelho de teste. No Windows, dentro da pasta da extração: `tree /F /A > arvore.txt` ou `dir /S /B > lista.txt`. Só nomes de arquivos, nenhum conteúdo. Dizer também quais opções do Avilla foram usadas (backup ADB, downgrade de APK do WhatsApp, cópia de pastas, etc.).
2. **Sistema das estações**: presumo **Windows 10/11** (o Avilla roda em Windows). Confirmar.
3. **Repositório**: criar um repositório **privado** no GitHub em nome de `Doctor-Henrique-Oliveira` quando a arquitetura for aprovada.
