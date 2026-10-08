# SIED — Sistema Integrado de Evidências Digitais

Plataforma forense local (offline) para importar, analisar, organizar e relatar evidências
extraídas de dispositivos móveis. Visão do produto em [`docs/VISAO.md`](docs/VISAO.md);
arquitetura aprovada em [`docs/ARQUITETURA.md`](docs/ARQUITETURA.md).

> **Estado atual: Fase 1 — importar, analisar, selecionar e relatar.** O app cria casos, cadastra
> dispositivos e importa uma pasta de extração: inventário com SHA-256 de todos os arquivos,
> manifesto `sha256sum`, SMS e WhatsApp do Android interpretados com a origem de cada registro,
> tudo com trilha de auditoria. Conversas reconstruídas e busca global sobre o que foi importado.
> O analista marca evidências (mensagem, trecho de conversa ou arquivo) e gera um relatório PDF
> com hashes conferidos, origem completa e cadeia de custódia.
> Interface em tema escuro com barra lateral de 13 módulos (Dashboard, Casos, Dispositivos,
> Extrações, Conversas, Mídias, Arquivos, Contatos, Localizações, Consultas, Relatórios,
> Auditoria e Configurações); os módulos sem dados ainda dizem "ainda não disponível", nunca mostram números
> fictícios. O importador do Avilla aguarda uma extração real para ser escrito
> (roteiro em [`docs/PREPARACAO-AVILLA-WHATSAPP.md`](docs/PREPARACAO-AVILLA-WHATSAPP.md)).

## Estrutura

```
crates/
  sied-dominio/        tipos do modelo (Caso, Dispositivo, contrato de Artefato) e validações; sem E/S
  sied-armazenamento/  SQLite: banco global (catálogo) + banco autocontido por caso, migrações, auditoria
    migracoes/global/  SQL versionado do banco global
    migracoes/caso/    SQL versionado do banco de caso
  sied-ingestao/       fonte somente leitura (pasta), inventário e manifesto, pipeline de importação,
                       importadores (por ferramenta: Pasta genérica; Avilla só contrato),
                       parsers (por app: SMS e WhatsApp Android), protocolo JSON para parsers externos
  sied-midia/          miniaturas das imagens do inventário (lidas somente leitura, SHA-256 conferido)
  sied-relatorio/      relatório PDF: Typst embutido, modelo `modelo.typ`, conferência das evidências
app/
  src-tauri/           camada Tauri: comandos, estado, logs
  src/                 interface React + TypeScript
    api/tipos/         tipos gerados do Rust (ts-rs) — não editar à mão
    componentes/       casca (BarraLateral, BarraSuperior), blocos reutilizáveis (Estrutura, Icone),
                       painel técnico, importação, inventário de arquivos
    rotas/             uma página por módulo (rotas em main.tsx)
testes/e2e/            teste ponta a ponta do app real (WebDriver)
testes/gerador/        gera extrações SINTÉTICAS para teste (nenhum dado real)
testes/rust/           caso sintético compartilhado pelos testes Rust
docs/                  visão e arquitetura
```

## Onde o SIED grava dados

| O quê | Windows (padrão) | Variável para trocar |
|---|---|---|
| Banco global (catálogo de casos, configurações) | `%LOCALAPPDATA%\br.sied.desktop\sied-global.db` | `SIED_PASTA_DADOS` |
| Logs técnicos (diários) | `%LOCALAPPDATA%\br.sied.desktop\logs\` | (segue `SIED_PASTA_DADOS`) |
| Casos | `Documentos\SIED\Casos\<número do procedimento>\caso.sied.db` | `SIED_PASTA_CASOS` |

Cada pasta de caso é autocontida: copiá-la leva o caso inteiro, e "Abrir pasta de caso…"
a registra em outra estação. Copie o caso **fechado**: enquanto está aberto, a trava
`caso.sied.lock` impede que ele seja aberto em outra janela ou estação, e o `-wal` ainda pode ter
dados que não foram para o `caso.sied.db`. Prefira uma pasta local (não OneDrive, Dropbox ou
rede); a tela de novo caso avisa quando a pasta parece sincronizada ou remota. Nível de log:
`SIED_LOG=debug`.

```
<pasta do caso>\
  caso.sied.db                       caso, dispositivos, extrações, inventário, registros, auditoria
  caso.sied.lock                     trava enquanto o caso está aberto
  importacoes\<id da extração>\
    manifesto.sha256                 SHA-256 de cada arquivo da extração (formato sha256sum)
    log-importacao.jsonl             log detalhado da importação (um evento JSON por linha)
  cache\bancos\<id da extração>\<id do arquivo>\
                                     cópias de trabalho conferidas por hash (o parser lê só estas)
  miniaturas\<2 primeiros do SHA-256>\<SHA-256>-v1.jpg
                                     miniaturas das imagens (dado derivado; pode ser apagado)
  relatorios\AAAA-MM-DD_HHMMSS_relatorio.pdf
  relatorios\AAAA-MM-DD_HHMMSS_relatorio.pdf.sha256
                                     relatórios gerados (hora no fuso do caso) e o SHA-256 de cada um
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

Importadores disponíveis: **Pasta genérica** (qualquer pasta; interpreta `mmssms.db`, `msgstore.db`
e `wa.db` onde estiverem). O **Avilla** aparece como "ainda não disponível" até sua estrutura ser
documentada a partir de uma extração real.

Parsers disponíveis:
- **SMS do Android** (tabela `sms`; MMS ainda não).
- **WhatsApp Android**:
  - `msgstore.db` nos esquemas novo e antigo, detectados pelo conteúdo, junto com o `wa.db` da
    mesma pasta;
  - mensagens, grupos, contatos e anexos ligados ao inventário;
  - backups `.crypt*` só inventariados;
  - conferido até agora só com bancos sintéticos.

## Interface

- **Tema escuro** com tokens em `app/src/estilos.css` (dourado só para o institucional e a ação
  principal; verde, azul, laranja e vermelho para estados). Fontes do sistema, ícones embutidos:
  nada vem da internet.
- **Casca**: antes de haver dispositivo e extração concluída, o caso usa uma preparação compacta
  sem a barra lateral analítica. Depois disso, a barra lateral reúne os 13 módulos e o cabeçalho
  mostra o caso aberto, busca global (**Ctrl+K**), importação em curso e usuário @ estação.
- **Dashboard do caso**: dados do procedimento, atalhos, indicadores contados no banco do caso,
  dispositivos, extrações e alertas. A trilha e sua verificação ficam no módulo próprio **Auditoria**.
- **Progresso da importação**: as quatro etapas, barra da etapa atual, tempo decorrido, taxa e
  tempo restante do inventário (medidos, não estimados por fórmula fixa) e a cópia de trabalho em
  conferência. Tudo vem dos números que o backend informa.
- **Volume real**: listas de conversas, mensagens e mídias longas são virtualizadas (só o que está
  visível existe na tela) e as páginas vêm do banco conforme a rolagem.
- **Miniaturas** na galeria de Mídias e nos anexos das conversas: o SIED lê a imagem da extração
  (somente leitura), confere o SHA-256 com o inventário e só então reduz. Se o arquivo mudou ou
  sumiu, a tela diz isso em vez de mostrar a imagem.
- **Detalhes técnicos** em blocos recolhíveis: origem (arquivo, SHA-256, tabela, registro,
  consulta, valores brutos), data original e formato, parser, importador, estado e confiança.
- Estrutura completa das telas e componentes em [`docs/CONTINUIDADE.md`](docs/CONTINUIDADE.md) §15.

## Conversas e busca

- **Conversas** (três colunas: lista, chat e contexto/origem técnica): os registros de um mesmo app e da
  mesma conversa (`thread_id` no SMS, jid no WhatsApp) são agrupados e mostrados como chat: recebidas à esquerda, enviadas à direita, separador por dia e hora com segundos no
  fuso do caso. "Detalhes técnicos" mostra, para a mensagem clicada, a origem completa (arquivo, hash,
  tabela, registro, valores brutos). No WhatsApp:
  - título (assunto do grupo ou nome do contato), tipo e nomes dos participantes vêm do próprio
    app na mesma extração;
  - anexos mostram se o arquivo foi achado na extração, com a miniatura quando é imagem.
- **Filtros de conversa** valem para o caso inteiro: texto (título, participante, nome no app ou
  telefone com ou sem máscara), app, individuais ou grupos e "com anexo". Em conversa longa,
  **Localizar** consulta o banco e rola até cada ocorrência (Enter e Shift+Enter).
- **Busca global** (SQLite FTS5): conteúdo das mensagens, remetentes/destinatários e nomes de arquivo do
  inventário, sem diferenciar acentos e maiúsculas, com busca por prefixo. Um texto só com dígitos e
  separadores é tratado como telefone e encontrado em qualquer formato (`(11) 90000-0001` acha
  `+5511900000001`). Filtros por extração, app e período. Abrir um resultado leva à conversa, rola até a
  mensagem e a destaca.
- As duas coisas são **derivadas** (migração 3 do banco do caso): mantidas por gatilhos a cada registro
  gravado e reconstruíveis a partir dos artefatos. Casos antigos ganham conversas e índice ao abrir.

## Evidências e relatório

- **Marcar**: em Conversas, clique numa mensagem e use a barra "Evidência" para adicionar a
  mensagem ou marcar o início e o fim de um trecho; em Mídias e Arquivos, "Adicionar às
  evidências" no detalhe do arquivo. O SIED grava o hash do alvo nesse momento (do registro, do
  arquivo ou das mensagens do trecho, na ordem) e ele não muda mais. Itens marcados ganham o selo
  **E-n** no chat e no detalhe.
- **Organizar** (módulo Relatórios): reordenar, editar título, categoria, descrição e observação,
  abrir na origem e remover com motivo. A remoção é lógica e tudo vai para a auditoria.
- **Gerar PDF**: escolha as seções e escreva observações e conclusão. Antes de gerar, o SIED confere
  a trilha de auditoria (se estiver quebrada, não gera) e cada evidência de novo: registros no
  banco do caso, trechos recalculados e arquivos relidos da extração em somente leitura. O PDF
  mostra, por evidência, o conteúdo, o hash da seleção, o hash conferido e a origem completa
  (arquivo, SHA-256, tabela, registro, consulta, valores brutos, parser e versão), e termina com o
  quadro de integridade e a posição da trilha de auditoria. Cada PDF é registrado no caso (somente
  inclusão) com o seu SHA-256. Para conferir fora do SIED: `sha256sum -c <nome>.pdf.sha256` dentro
  da pasta `relatorios`.
- O Typst e as fontes vão dentro do programa: o relatório é gerado offline.

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
cargo test --workspace          # domínio, bancos, migrações, auditoria, contratos, relatório, serviço do app
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
Com `--whatsapp` antes da pasta, a extração ganha também `msgstore.db`, `wa.db`, mídias e um
backup `.crypt14`, todos sintéticos, gerados a partir de `testes/fixtures/whatsapp`. Com
`--volume`, ganha uma conversa SMS de 5.000 mensagens, 300 conversas curtas e 400 imagens PNG
desenhadas pelo próprio gerador, para testar as telas com volume.

## Regras do repositório

- Nunca versionar dados reais de investigações, extrações, bancos (`*.db`), relatórios, `.env` ou credenciais
  (o `.gitignore` bloqueia os padrões comuns).
- Fixtures de teste são **sintéticas** e identificadas como tal.
