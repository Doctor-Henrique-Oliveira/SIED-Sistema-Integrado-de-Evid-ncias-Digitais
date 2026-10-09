# Contatos iOS — contrato confirmado

O SIED reconhece `AddressBook.sqlitedb`/`addressbook.sqlitedb` apenas como
**candidato** a contatos iOS. O parser `ios_contatos` abre a cópia de trabalho
somente para leitura e só interpreta o banco quando a tabela `ABPerson` existe.

Cada linha é emitida como artefato `Contato`, com a referência estável ao
`rowid`, o hash/origem preenchidos pelo pipeline e as colunas brutas preservadas
em `metadados.colunas_brutas`. Um nome só é exibido quando uma coluna existente
(`First`, `FirstName`, `Last`, `LastName`, `Organization` ou `Name`) contém texto;
nenhum valor ausente é inferido. Colunas de versões desconhecidas continuam
preservadas, mas não são reinterpretadas.

O parser não abre nem decodifica automaticamente tabelas `ABMultiValue` ou
outros bancos de agenda sem amostra/documentação correspondente. Um banco sem
`ABPerson` é recusado com `FormatoNaoReconhecido`, permanecendo apenas no
inventário. Isso evita transformar um arquivo com nome parecido em evidência
interpretada.

## Limites atuais

- no máximo 100 colunas da tabela `ABPerson` por linha, para limitar materialização;
- a origem original nunca é alterada: somente a cópia verificada é consultada;
- RCS, contatos iOS fora de `ABPerson`, e-mail proprietário e extensões avançadas
  do WhatsApp continuam aguardando documentação e amostras autorizadas.
