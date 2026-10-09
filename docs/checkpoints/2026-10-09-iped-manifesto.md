# Checkpoint — manifesto IPED no plano de importação

## Entrega

O importador IPED passou a enriquecer `PlanoImportacao` sem executar o IPED e sem
interpretar o índice Lucene ou os bancos internos da ferramenta:

- lê apenas até 64 KiB de `iped/IPEDConfig.txt`, `iped/LocalConfig.txt`,
  `iped/conf/OCRConfig.txt` e `iped/conf/AudioTranscriptConfig.txt`;
- registra no log do plano somente chaves de capacidade permitidas (hash, OCR,
  transcrição, similaridade visual, grafo, QR, carving, índice e parâmetros locais);
- preserva a versão somente quando aparece explicitamente em
  `iped/version.txt` ou no cabeçalho limitado de `iped/ReleaseNotes.txt`;
- não infere versão a partir do instalador local, não replica endpoints/segredos e
  não habilita processamento com base nesses valores;
- mantém o inventário e o hash de todos os arquivos, inclusive `iped/`, e mantém a
  lista CSV como único conteúdo IPED interpretado.

## Validação

Foram adicionados testes para versão declarada, capacidades permitidas e ausência de
inferência quando a versão não existe. `cargo fmt --all` passou. Os testes Rust não
puderam linkar neste Windows porque `link.exe`/MSVC não estão disponíveis; isso é uma
limitação do ambiente, não uma alteração de contrato ou um atalho no código.
