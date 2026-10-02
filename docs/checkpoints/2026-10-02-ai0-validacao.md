# Checkpoint — AI-0: viabilização da validação

Data: 02/10/2026. **VALIDAÇÃO PARCIAL; NÃO PRONTA PARA INTEGRAÇÃO.**

Branch: `feat/mdfml-ai0-busca-local`; PR #24 permanece draft, sem merge.
Esta missão continua somente a AI-0. **AI-1 não foi iniciada nem autorizada nesta etapa.**
O recorte funcional e suas limitações estão no
[checkpoint de implementação](2026-10-02-ai0-busca-local.md).

## Resultado externo efetivamente observado

Uma consulta de estado à CI do commit
`21f43f187193a9c5803258a00ef4b4197072ba8f`, execução
[37041607181](https://github.com/Doctor-Henrique-Oliveira/SIED-Sistema-Integrado-de-Evid-ncias-Digitais/actions/runs/37041607181),
mais diagnóstico do job Linux já encerrado, constatou:

- `cargo test -p sied-armazenamento --lib`: **81 passaram, zero falhas/ignorados**.
  Inclui testes do controle cooperativo, consultas e exports `ts-rs` do armazenamento.
  Não equivale à bateria de integração ou à validação do desktop.
- Upload de `sied-contratos-typescript` falhou por **cota de artifacts do GitHub esgotada**:
  `Artifact storage quota has been hit`. Não foi falha de compilação Rust.
- O upload era uma etapa obrigatória anterior às demais verificações; sua falha
  fez o build frontend, fmt, Clippy, testes workspace e gate de contratos serem pulados.
- Não havia artifact de contratos disponível naquela consulta. Os logs antigos
  não contêm os bytes desses arquivos; nomes de testes não permitem recuperá-los.
- Windows e instaladores ainda estavam em execução no snapshot observado. Esse
  estado não é apresentado como resultado final. Não houve polling ou espera.

Os novos testes/infraestrutura deste checkpoint ainda exigem uma execução própria.
O push aciona a CI normal do PR; esta missão não aguarda nem consulta novamente seus jobs.

## Dependência local exata

O toolchain instalado é `stable-x86_64-pc-windows-msvc` (Rust/Cargo 1.98.1).
Não foram encontrados `link.exe`, `cl.exe`, compilador C alternativo completo,
Windows SDK/UCRT com headers e bibliotecas nem ambiente WSL/Docker utilizável.
O `rust-lld.exe` isolado não fornece compilador C ou SDK.

Mesmo o armazenamento depende de `rusqlite` com SQLite `bundled`, que compila C
e precisa de ligação nativa. Sem esse conjunto, não é possível executar localmente
a bateria Rust, gerar oficialmente os contratos novos ou empacotar o desktop.
**Não houve nova compilação Tauri local, instalação/reparo de toolchain nem alteração
de Windows, WebView2, McAfee ou política corporativa.**

## Validação viabilizada sem atalhos

- A CI testa domínio, ingestão e armazenamento (unidades e integrações) antes do
  frontend, e executa Clippy desses crates e a medição sintética AI-0 em release.
  A bateria/Clippy do workspace completo continuam exigidos depois do build frontend.
- Somente os dois uploads auxiliares de contratos recebem `continue-on-error`.
  Upload de instaladores, testes, geração, Clippy, compilação e gate de versionamento
  **não** recebem essa exceção. Gatilhos e permissões da CI permanecem iguais.
- `.github/scripts/contratos-typescript.mjs` publica nos logs os bytes dos arquivos
  que os testes Rust realmente exportaram. Não gera, infere nem reescreve DTOs.
  São publicados blocos distintos após a base e após o workspace completo.
- A recuperação confere HEAD esperado, fase, checkout real, fingerprints das fontes
  Rust/Cargo/config/lock, nomes, limites, Base64 canônico, hashes SHA-256 e manifesto.
  Valida o bloco inteiro e todos os destinos antes de qualquer escrita; rejeita
  blocos incompletos, divergentes, caminhos externos e symlinks/junctions.
- O gate permanece recusando contratos modificados ou novos não versionados.
  Portanto a próxima execução ainda pode ficar vermelha até a incorporação dos
  exports genuínos e sua nova validação. Isso é deliberado, não um gate removido.

Para recuperar, depois de baixar **uma vez** o log do job do commit esperado,
no checkout limpo desse HEAD, sem aguardar automaticamente:

```powershell
node .github/scripts/contratos-typescript.mjs recuperar '<log-local>' '<HEAD-completo>' workspace
```

A fase `workspace` inclui também contratos do desktop, como `CodigoErro`.
A fase `armazenamento` serve à conferência preliminar, não certifica os contratos
completos. Revisar/versionar os bytes recuperados e repetir build e gate oficial.

## Avaliação sintética adicionada

- Corpus de 14 SMS, sete exemplos positivos e sete negativos, cobrindo as sete
  classes sintáticas: IP, e-mail, URL, domínio, hash, telefone e coordenadas.
  Compara conjuntos exatos de tipo/campo/valor, detectando FP/FN no corpus limitado.
  Não promete recall real, validade externa ou identidade de pessoas/dispositivos.
- Benchmark `medir_ai0_com_volume`: gerador SMS já existente, 50.000 mensagens
  mais sete sementes, três repetições; literal, expansão, indicadores, snapshot e
  paginação. Fonte construída antes da importação normal, caso somente temporário.
  Mede latências observadas e bytes do payload JSON; não mede RSS/CPU, não controla
  cache do SO e não estabelece SLO. Atingir prazo marca medição incompleta e reprova
  o teste; não conta como medição aprovada.
- O hash da semente também produz um candidato sintático a telefone. A contagem
  esperada reconhece essa sobreposição; o dado não foi alterado para escondê-la.
- Leitura não cria auditoria; snapshot deve criar exatamente um evento atômico.
  Fontes/hashes/auditoria são conferidos. Nenhum caso instalado/real foi aberto ou
  alterado; `.claude/`, parsers, migrações e código funcional permanecem intocados.

```powershell
cargo test --locked -p sied-dominio -p sied-ingestao -p sied-armazenamento
$env:SIED_VOLUME_AI0 = '50000'
$env:SIED_REPETICOES_AI0 = '3'
cargo test --locked --release -p sied-armazenamento --test desempenho medir_ai0_com_volume -- --ignored --exact --nocapture
```

## Validação local desta missão

- Interface: `npm.cmd test -- --maxWorkers=2`: **103/103 testes**, 16 arquivos.
- `npm.cmd run build`: **falhou**, oito imports de DTOs ausentes e erros derivados.
  Não foram criados tipos manuais, casts ou exceções para fazê-lo passar.
- `npm.cmd exec -- vite build`: bundle gerado, 117 módulos, aviso não bloqueante de
  chunk acima de 500 kB. **Não substitui typecheck nem build completo.**
- `cargo fmt --all --check` e `git diff --check`: **aprovados**.
- `node --test .github/scripts/contratos-typescript.test.mjs`: **20/20 passaram**,
  sem skips, no Node local 24.14.1. A CI usa Node 22; essa execução ainda é pendente.
  Somente fixtures temporárias sintéticas (textos, sem contratos DTO inventados).
  Inclui compatibilidade LF/CRLF, índice/HEAD/worktree divergentes e fontes extras,
  corrupção, limites, fase/manifesto, recuperação integral e destinos inseguros.
- `node --check` dos dois scripts: **aprovado**. A revisão estática independente
  do corpus/benchmark não substitui sua compilação e execução Rust.

## Pendências reais para integração

1. Executar a CI do novo código, recuperar/versionar os contratos genuínos completos
   e comprovar que o gate de geração não produz diferenças.
2. Aprovar Rust/Clippy do workspace em Linux/Windows, build frontend tipado e build
   completo Tauri/instaladores. Cota de upload ainda pode impedir a entrega dos
   instaladores mesmo se a compilação passar; não foi alterada nesta missão.
3. Executar corpus e benchmark novos, avaliar resultados e definir orçamento de
   bytes/recursos. RSS/CPU, campos/páginas grandes e critério de aceitação seguem abertos.
4. Revalidar o modelo de ameaças do incremento e validar operacionalmente o fluxo
   com caso SINTÉTICO pelo instalador oficial, sem abrir caso de produção.
5. Revisão persistida/Central de Indicadores e perfis institucionais do planejamento
   não estão entregues neste recorte. Distinguir o recorte da conclusão integral AI-0.

**AI-0 não pode ser declarada tecnicamente concluída nem integrada neste checkpoint.**
Sem merge e sem avanço para AI-1.
