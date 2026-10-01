# Checkpoint: pasta de relatórios conferida e destaque da busca em pedaços (01/10/2026)

Dívida técnica da auditoria (HANDOFF §25, itens F9 e F8), na construção contínua.

## IMPLEMENTADO

- **F9: conferência da pasta de relatórios** (`crates/sied-armazenamento/src/relatorios.rs`,
  `conferir_pasta`; comando `conferir_pasta_relatorios`).
  - Cada PDF registrado é relido e aparece na tela de Relatórios como **PDF confere**, **PDF
    alterado** (SHA-256 diferente do registrado na geração) ou **PDF ausente**.
  - O que estiver em `relatorios/` sem registro no caso é listado num aviso: PDF sem registro
    (queda entre gravar e registrar, B7), gravação interrompida (`.parcial`), hash sem relatório
    (`.sha256` avulso) ou outro arquivo. O `.sha256` de um relatório registrado é esperado.
  - Só lê: nada é apagado, movido nem registrado, e nada entra na trilha de auditoria.
  - Gerar um relatório novo reconfere a pasta (mesma raiz de chave da lista).
- **F8: destaque da busca em pedaços** (`crates/sied-armazenamento/src/busca.rs`, `pedacos`).
  - O backend pede ao FTS o mesmo trecho com marcadores e sem marcadores, e separa o trecho em
    `PedacoTrecho { texto, destaque }`: os marcadores são só inserções, então o que sobra ao
    percorrer o trecho puro são os marcadores de verdade. Se não fechar, o trecho vem sem destaque.
  - Os marcadores não saem mais do backend; a interface só pinta os pedaços.
  - `AchadoArtefato.trecho` passou de texto com marcadores para lista de pedaços (tipo TS
    regerado).

## TELAS (app real, caso SINTÉTICO)

- `docs/checkpoints/relatorios-busca/r1-busca-destaque.png`: busca com o termo destacado.
- `docs/checkpoints/relatorios-busca/r3-relatorio-avulsos.png`: relatório com "PDF confere" e
  aviso de 2 arquivos sem registro (um PDF e um `.parcial` colocados na pasta).

## TESTES

- 276 Rust (+3 medições ignoradas), 57 vitest, roteiro E2E no app real: todos passam.
- `relatorio.rs::pasta_de_relatorios_conferida_com_o_registro`: 3 relatórios gerados; um PDF
  alterado, um apagado, um PDF avulso, um `.sha256` avulso e um `.parcial`; confere situação e
  tipo de cada um e que a pasta e os registros não mudaram.
- `volume.rs::destaque_da_busca_nao_se_confunde_com_o_texto_da_mensagem`: SMS sintético com os
  caracteres dos marcadores no próprio texto, pelo FTS real: o texto volta igual e só o termo
  vem destacado.
- Unitários de `busca::pedacos` (casos normais, marcadores no texto, trecho que não fecha).
- Vitest: selo "PDF alterado", aviso dos arquivos sem registro (sem botão de ação), e que sem
  resposta da conferência nada é mostrado.

## ARQUIVOS

- `crates/sied-armazenamento/src/relatorios.rs`, `caso.rs`, `busca.rs`
- `app/src-tauri/src/servico.rs`, `comandos.rs`, `lib.rs`
- `app/src/api/cliente.ts`, `consultas.ts`, `tipos/{ArquivoAvulso,ConferenciaPastaRelatorios,PdfConferido,PedacoTrecho,AchadoArtefato}.ts`
- `app/src/rotas/PaginaRelatorios.tsx`, `PaginaBusca.tsx` e testes
- Testes Rust: `sied-relatorio/tests/relatorio.rs`, `sied-armazenamento/tests/{volume,importacao}.rs`

## DECISÕES

- A conferência roda ao abrir a tela de Relatórios (lê cada PDF; poucos MB por relatório). Não
  grava na trilha: é uma leitura, como abrir a pasta.
- Arquivos sem registro **não** são apagados nem oferecidos para apagar: são arquivos dentro da
  pasta do caso, e decidir sobre eles cabe ao analista.
- Pedaços em vez de posições: evita converter posições entre UTF-8 (Rust) e UTF-16 (JavaScript).

## PROBLEMAS

Nenhum em aberto.

## COMO EXECUTAR / COMO TESTAR

```text
cargo test --workspace
cd app && npm test
testes/e2e/executar.sh target/debug/sied-desktop
```

## LIMITAÇÕES

- A conferência compara o PDF com o SHA-256 gravado no banco do caso: quem reescreve banco e PDF
  juntos não é detectado (F3, assinatura/âncora externa, continua aberto).

## PRÓXIMA FASE

Avilla segue aguardando a árvore real. Da dívida técnica restam F2 (reconstrução dos derivados),
F3 (assinatura), F5 (unidade mapeada) e F6 (hash em paralelo), a avaliar pela necessidade.
