# Fixtures SINTÉTICAS do IPED

Nada aqui veio de um caso real. Só o que o código-fonte público do IPED (projeto
sepinf-inc/IPED: `ExportCSVTask`, `ExportFileTask` e os arquivos
`iped-engine-messages*.properties`) define foi imitado:

- `caso/iped/`: a pasta de saída do IPED, com `IPEDConfig.txt`, `conf/` e `index/`.
  Os arquivos são textos avisando que são sintéticos; não há índice Lucene.
- `caso/Lista de Arquivos.csv`: a lista de itens na versão em português (UTF-8 com
  BOM, CRLF, separador `;`, 15 colunas entre aspas, datas de `java.util.Date`).
- `caso/Exportados/arquivos/<h0>/<h1>/<MD5>.<ext>`: dois arquivos exportados, nomeados
  pelo MD5 como o IPED faz, com o atalho `=HIPERLINK(...)` na lista.
- `FileList.csv`: a mesma lista na versão em inglês (separador `,`), sem a pasta do caso.

Nomes, caminhos, marcadores, TrackIds, datas e os hashes dos itens não exportados são
inventados (os hashes repetem um dígito). Os arquivos têm `-text` em `.gitattributes`
para o git não trocar o fim de linha.
