# SIED — Sistema Integrado de Evidências Digitais

Plataforma forense local (offline) para importar, analisar, organizar e relatar evidências
extraídas de dispositivos móveis. Visão do produto em [`docs/VISAO.md`](docs/VISAO.md);
arquitetura aprovada em [`docs/ARQUITETURA.md`](docs/ARQUITETURA.md).

> **Estado atual: Fase 1 — fundação.** O app inicia, cria casos, cadastra dispositivos e
> mantém tudo gravado com trilha de auditoria. Ainda não importa extrações.

## Estrutura

```
crates/
  sied-dominio/        tipos do modelo (Caso, Dispositivo, contrato de Artefato) e validações; sem E/S
  sied-armazenamento/  SQLite: banco global (catálogo) + banco autocontido por caso, migrações, auditoria
    migracoes/global/  SQL versionado do banco global
    migracoes/caso/    SQL versionado do banco de caso
  sied-ingestao/       contratos: fonte somente leitura, Importador (por ferramenta), Parser (por app),
                       protocolo JSON para parsers externos, AvillaImporter (só contrato)
app/
  src-tauri/           camada Tauri: comandos, estado, logs
  src/                 interface React + TypeScript
    api/tipos/         tipos gerados do Rust (ts-rs) — não editar à mão
testes/e2e/            teste ponta a ponta do app real (WebDriver)
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
fecha o programa, abre de novo e confere caso, dispositivo e trilha de auditoria.

## Regras do repositório

- Nunca versionar dados reais de investigações, extrações, bancos (`*.db`), relatórios, `.env` ou credenciais
  (o `.gitignore` bloqueia os padrões comuns).
- Fixtures de teste são **sintéticas** e identificadas como tal.
