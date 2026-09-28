# SIED — Sistema Integrado de Evidências Digitais

Plataforma forense local (offline) para importar, analisar, organizar e relatar evidências
extraídas de dispositivos móveis. Visão do produto em [`docs/VISAO.md`](docs/VISAO.md);
arquitetura aprovada em [`docs/ARQUITETURA.md`](docs/ARQUITETURA.md).

> **Estado atual: Fase 1 — fundação + primeira importação.** O app cria casos, cadastra
> dispositivos e importa uma pasta de extração: inventário com SHA-256 de todos os arquivos,
> manifesto `sha256sum`, SMS do Android interpretados com a origem de cada registro, tudo
> com trilha de auditoria. O importador do Avilla aguarda uma extração real para ser escrito.

## Estrutura

```
crates/
  sied-dominio/        tipos do modelo (Caso, Dispositivo, contrato de Artefato) e validações; sem E/S
  sied-armazenamento/  SQLite: banco global (catálogo) + banco autocontido por caso, migrações, auditoria
    migracoes/global/  SQL versionado do banco global
    migracoes/caso/    SQL versionado do banco de caso
  sied-ingestao/       fonte somente leitura (pasta), inventário e manifesto, pipeline de importação,
                       importadores (por ferramenta: Pasta genérica; Avilla só contrato),
                       parsers (por app: SMS Android), protocolo JSON para parsers externos
app/
  src-tauri/           camada Tauri: comandos, estado, logs
  src/                 interface React + TypeScript
    api/tipos/         tipos gerados do Rust (ts-rs) — não editar à mão
testes/e2e/            teste ponta a ponta do app real (WebDriver)
testes/gerador/        gera extrações SINTÉTICAS para teste (nenhum dado real)
docs/                  visão e arquitetura
```

## Onde o SIED grava dados

| O quê | Windows (padrão) | Variável para trocar |
|---|---|---|
| Banco global (catálogo de casos, configurações) | `%LOCALAPPDATA%\br.sied.desktop\sied-global.db` | `SIED_PASTA_DADOS` |
| Logs técnicos (diários) | `%LOCALAPPDATA%\br.sied.desktop\logs\` | (segue `SIED_PASTA_DADOS`) |
| Casos | `Documentos\SIED\Casos\<número do procedimento>\caso.sied.db` | `SIED_PASTA_CASOS` |

Cada pasta de caso é autocontida: copiá-la leva o caso inteiro, e "Abrir pasta de caso…"
a registra em outra estação. Nível de log: `SIED_LOG=debug`.

```
<pasta do caso>\
  caso.sied.db                       caso, dispositivos, extrações, inventário, registros, auditoria
  importacoes\<id da extração>\
    manifesto.sha256                 SHA-256 de cada arquivo da extração (formato sha256sum)
    log-importacao.jsonl             log detalhado da importação (um evento JSON por linha)
  cache\bancos\<id da extração>\<id do arquivo>\
                                     cópias de trabalho conferidas por hash (o parser lê só estas)
```

## Importação de extrações

A pasta da extração é **somente lida**: arquivos são abertos para leitura, links simbólicos não
são seguidos e nada é gravado nela. O SIED recusa uma pasta de extração que contenha a pasta do
caso, ou esteja dentro dela.

1. **Inventário**: cada arquivo é lido uma vez, recebe SHA-256 e é classificado pela assinatura do
   conteúdo (um banco SQLite renomeado para `.jpg` continua sendo banco).
2. **Manifesto**: `manifesto.sha256`, cujo próprio SHA-256 vai para a auditoria. Para conferir com
   ferramentas externas, a partir da raiz da extração: `sha256sum -c <caso>/importacoes/<id>/manifesto.sha256`.
3. **Cópia de trabalho**: antes de um parser ler um banco, ele (e o `-wal`/`-journal`) é copiado para a
   pasta do caso e o hash da cópia é comparado ao do inventário. Se o original mudou, a leitura é recusada.
4. **Parsers**: cada registro interpretado guarda arquivo, hash, tabela, ID do registro, consulta usada,
   valores brutos da linha, parser e versão. A tela da extração mostra tudo isso em "Detalhes técnicos".

Importadores disponíveis: **Pasta genérica** (qualquer pasta; interpreta `mmssms.db` do Android onde
estiver). O **Avilla** aparece como "ainda não disponível" até sua estrutura ser documentada a partir de
uma extração real. Parser disponível: **SMS do Android** (tabela `sms`; MMS ainda não).

Fechar o caso ou o programa durante uma importação a cancela e registra o cancelamento. Uma importação
interrompida por queda de energia aparece como "interrompida" ao reabrir o caso; basta importar de novo.

## Como executar (desenvolvimento, Windows 10/11)

Pré-requisitos: [Rust](https://rustup.rs) (estável), Node.js 22+, Microsoft C++ Build Tools
e WebView2 (já vem no Windows 11). Depois:

```powershell
cd app
npm install
npm run tauri dev        # abre o SIED com recarga automática da interface
```

Gerar instalador (NSIS e MSI, com instalador offline do WebView2 embutido):

```powershell
cd app
npm run tauri build
```

No Linux, instale também `libwebkit2gtk-4.1-dev` e `librsvg2-dev`.

## Como testar

```bash
cargo test --workspace          # domínio, bancos, migrações, auditoria, contratos, serviço do app
cargo clippy --workspace --all-targets -- -D warnings
cd app && npm run typecheck && npm test   # tipos e testes da interface
```

`cargo test` também regenera `app/src/api/tipos/`; se algo mudar ali, faça commit junto.

Teste ponta a ponta do app real (Linux: requer `tauri-driver`, `WebKitWebDriver`, `xvfb`,
e opcionalmente `openbox` + `wmctrl` para fechar a janela como o usuário):

```bash
cd app && npx tauri build --debug --no-bundle && cd ..
testes/e2e/executar.sh target/debug/sied-desktop
```

Ele inicia o SIED, cria um caso, cadastra um dispositivo (testando a rejeição de IMEI inválido),
fecha o programa, abre de novo e confere caso, dispositivo e trilha de auditoria. Depois gera uma
extração sintética (`testes/gerador/extracao_sintetica.py`, requer Python 3), importa, confere os
registros, os detalhes técnicos e o manifesto contra os arquivos, e prova que a extração não mudou.

Para experimentar a importação manualmente:
`python3 testes/gerador/extracao_sintetica.py C:\Temp\Extracao-SINTETICA` e importe essa pasta.

## Regras do repositório

- Nunca versionar dados reais de investigações, extrações, bancos (`*.db`), relatórios, `.env` ou credenciais
  (o `.gitignore` bloqueia os padrões comuns).
- Fixtures de teste são **sintéticas** e identificadas como tal.
