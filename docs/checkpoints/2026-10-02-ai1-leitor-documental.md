# AI-1 — primeira entrega visível: leitor documental

02/10/2026 · `feat/mdfml-ai1-documentos-locais` · PR #25, draft, sem merge.

## Implementado

No módulo **Arquivos**, selecionar um arquivo abre seus detalhes com **Ler documento**.
TXT, CSV, TSV, JSON, XML, MD e LOG UTF-8 são apresentados como texto exato, sem interpretar
HTML/Markdown/scripts. Até 2 MiB; sem OCR, PDF ou Office. Arquivo vazio, hash ausente,
limites, falhas e origem divergente possuem estados explícitos.

Painel expansível mostra caminho, caso/extração/arquivo, SHA-256 da origem e do texto,
motor/versão e horário da leitura. Página não é inventada. Pode ocultar e conferir/reler.
Não cria evidência, não persiste texto e não altera relatórios.

`ler_documento(arquivo_id, caso_id)` recebe apenas o arquivo escolhido e uma guarda de
contexto. O backend compara o caso ativo e resolve caminho, SHA-256, tamanho e extração
no inventário desse banco. `ArquivoComOrigem` ganhou só o identificador da extração no
SELECT existente: nenhuma migração, nova conexão ou escrita direta.

A leitura roda fora do mutex do serviço e da thread da janela. Uma operação documental
por aplicativo, com recusa imediata de concorrência; guarda RAII liberada também após pânico.
Seleção vinculada a caso/extração, linhas antigas indisponíveis durante placeholder,
resultados tardios e proveniência incompatível não aparecem em outro contexto.

## Contratos e validação

`EstadoDocumento`, `TextoDocumento` e `ResultadoDocumento` são gerados por **ts-rs no Rust**.
O job `Contratos documentais (Rust)` registrou o conteúdo gerado no log, permitindo recuperá-lo
sem depender de artifacts/quota ou de MSVC local. Contratos recuperados do job `110976787716`,
run `37048682162`, commit `2f5389ffcdd3e44f70bfb858c38853fcde2caeb6`.
Não são contratos escritos manualmente. Gates existentes não foram enfraquecidos.

- **94/94 testes da interface passaram**: 21 do leitor e 3 da fronteira documental novos,
  além da bateria existente. Cobrem seleção/fechamento, caso trocado, texto inerte,
  loading/vazio/recusas/erro, proveniência e respostas tardias.
- **Build TypeScript + Vite passou** com os contratos genuínos. Aviso não bloqueante de
  chunk acima de 500 kB, mantido sem ocultar o warning.
- `cargo fmt --all --check`, lockfile regenerado pelo Cargo e `git diff --check` passaram.
- Na consulta única da execução acima: geração Rust, testes/Clippy da fundação, medição e
  testes/Clippy do workspace Linux passaram no commit de contratos. Isso **não valida o
  adaptador desktop acrescentado depois**; Windows/instaladores ainda estavam em execução.
- 7 regressões Rust do adaptador preparadas (uma Unix): origem intacta, inventário/auditoria
  sem escrita, contexto de caso, divergência em pedido sintético, recusa de fontes e concorrência.
  Execução desses testes e build nativo do novo adaptador ainda dependem da CI/toolchain disponível.

## Prévia e teste manual

Entrada isolada de teste: `app/src/testes/previa-documentos.html`. Com `npm run dev` em `app`:
`http://127.0.0.1:1420/src/testes/previa-documentos.html`.
Usa o componente real e nove cenários exclusivamente sintéticos. SHA-256 e tamanho calculados
sobre o texto sintético; banner permanente **backend não conectado**. O mock pertence somente
a essa entrada, não à produção. Não representa execução do aplicativo Windows ou prova de integridade backend.

A automação do navegador recusou a abertura porque não conseguiu verificar a política administrativa.
Não foi contornada a restrição. A avaliação visual desta prévia fica para abertura manual pelo usuário;
testes DOM passaram, mas não se afirma que screenshots/responsividade foram inspecionados.

No app nativo atualizado: abrir caso sintético → Arquivos → selecionar TXT/CSV/TSV/JSON/XML/MD/LOG
do inventário → Ler documento → expandir proveniência → ocultar/reler → trocar arquivo/caso.
Verificar também arquivo vazio e formatos indisponíveis. Não usar caso real para validação inicial.

## Preservação e limitações

Não foram alterados parsers/Avilla, banco/migrações, hashes históricos, regras de evidência,
auditoria, relatórios, CSP/capabilities, segurança corporativa ou `.claude/`. Sem rede externa,
modelo/engine novo, instalação de ferramentas ou compilação Tauri local.

A fonte privada recusa links/reparse points intermediários e finais, diretórios e endereço
diferente do inventário. **Placeholders OneDrive também podem ser recusados**; não há hidratação
ou reparo automático da origem. Hash/tamanho conferidos antes da publicação; não se promete
contenção rígida contra mutação concorrente do filesystem ou interrupção de E/S bloqueada.
Ocultar a leitura descarta a apresentação, não afirma cancelar um trabalho Rust já iniciado.

O modelo de ameaças da fundação é histórico e não constitui validação dessa nova fronteira
desktop. Revisão formal dessa integração e validação nativa permanecem requisitos antes de merge.
Sem aguardar CI/instaladores, sem polling, sem merge. AI-1 inteira e AI-2 não concluídas/iniciadas,
respectivamente. A entrega atual é leitura UTF-8, não OCR/PDF/entidades/pesquisa multimodal.
