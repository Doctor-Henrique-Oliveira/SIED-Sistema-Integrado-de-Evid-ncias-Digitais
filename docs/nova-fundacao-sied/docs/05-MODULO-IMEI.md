# Módulo de consulta de IMEI

## 1. Finalidade

Fornecer consulta verificável de identificadores de equipamento a partir de uma fonte de dados autorizada, versionada e rastreável.

O módulo não deve prometer consulta a operadora, titular, localização, bloqueio ou ocorrência criminal sem integração e autorização específicas.

## 2. Conceitos separados

| Conceito | O que significa no SIED |
|---|---|
| IMEI informado | valor digitado, importado ou encontrado em campo explicitamente rotulado |
| IMEI normalizado | somente os 15 dígitos, preservando também o literal original quando for evidência |
| Luhn válido | estrutura matemática válida; não prova existência ou legitimidade |
| TAC consultado | prefixo usado para procurar alocação na base disponível |
| Resultado de catálogo | fabricante/modelo e atributos exatamente como constam na fonte |
| Resultado operacional externo | bloqueio, linha, titularidade etc.; fora do escopo inicial |

## 3. Fluxo de consulta

1. usuário entra ou cola o IMEI;
2. interface normaliza sem perder o valor originalmente informado no evento de consulta;
3. valida quantidade de dígitos e Luhn;
4. mostra claramente validade estrutural;
5. extrai a chave de consulta esperada pelo dataset;
6. consulta o snapshot ativo;
7. retorna resultado, fonte e versão;
8. permite vincular o resultado a um dispositivo, com confirmação;
9. registra auditoria mínima;
10. nunca transforma ausência na base em “IMEI falso”.

## 4. Contrato de resultado

```text
ImeiLookupResult
- query_id
- input_literal
- normalized_imei
- luhn_status
- lookup_key
- match_status: exact | prefix | not_found | ambiguous | invalid
- manufacturer
- model
- marketing_name
- allocation_body
- attributes estruturados da fonte
- dataset_id
- dataset_version
- dataset_sha256
- imported_at
- warnings[]
```

Campos ausentes permanecem ausentes. Não completar nomes por adivinhação.

## 5. Contrato do dataset

Antes de importação, registrar:

- nome da fonte;
- entidade fornecedora;
- URL ou referência de obtenção;
- licença/termo de uso;
- data de publicação;
- data de obtenção;
- formato;
- encoding;
- tamanho;
- SHA-256 do arquivo original;
- quantidade de registros lidos, aceitos, rejeitados e duplicados;
- mapeamento de colunas;
- versão do importador;
- operador responsável.

O dataset não deve ser versionado no Git se contiver dados restritos, pessoais ou licença incompatível.

## 6. Formatos de entrada

Suportar inicialmente um único formato real fornecido e documentado. O importador pode ter um contrato canônico, mas adaptadores só são declarados após testes.

Exemplo canônico de staging:

```text
source_row_id
lookup_key
manufacturer
model
marketing_name
allocation_body
raw_payload_json
```

Guardar `raw_payload_json` permite auditoria da linha importada, respeitando licença e classificação da fonte.

## 7. Persistência recomendada

Catálogo local SQLite separado do banco do caso:

```text
imei_catalog.sqlite
  dataset
  dataset_import
  tac_record
  rejected_record
  catalog_audit
```

Motivos:

- consulta rápida offline;
- catálogo pode ser atualizado sem migrar cada caso;
- caso registra apenas versão/hash usado;
- rollback de snapshot;
- separação de licença e backup.

## 8. Versionamento e ativação

- importação cria snapshot inativo;
- validações rodam antes da ativação;
- ativação é transacional;
- apenas um snapshot ativo por fonte/escopo;
- snapshot anterior permanece disponível conforme retenção;
- consultas já registradas continuam apontando para a versão usada;
- atualização não reescreve silenciosamente resultados históricos.

## 9. Duplicidade e conflito

Quando a fonte contiver múltiplas linhas para a mesma chave:

- não escolher silenciosamente;
- classificar como duplicata idêntica ou conflito;
- preservar linhas e origem;
- aplicar regra documentada somente depois de validada;
- exibir resultado ambíguo quando necessário.

## 10. Auditoria e privacidade

Consulta vinculada a procedimento pode registrar ator, horário, finalidade, dataset e resultado resumido. A retenção do IMEI consultado deve seguir política institucional.

Consultas exploratórias não devem ser enviadas a serviço externo sem aviso e autorização. Logs técnicos não precisam repetir o IMEI completo; podem usar identificador da consulta e valor mascarado.

## 11. Interface mínima

- campo único com máscara tolerante;
- estado de validade estrutural;
- resultado principal sem excesso de cartões;
- fonte e versão visíveis;
- aviso quando não encontrado;
- comparação com fabricante/modelo já cadastrados;
- ação “Vincular ao dispositivo”;
- histórico somente quando permitido;
- importação/gestão de catálogo restrita a administrador autorizado.

## 12. Testes mínimos

- IMEI válido e inválido por Luhn;
- separadores e espaços;
- caracteres Unicode parecidos com dígitos;
- entrada curta, longa e vazia;
- encontrado, não encontrado e ambíguo;
- duplicata idêntica e conflitante;
- arquivo com encoding inválido;
- colunas ausentes;
- linhas enormes e arquivo grande;
- cancelamento e rollback;
- hash divergente;
- ativação e retorno ao snapshot anterior;
- ausência de vazamento em logs;
- autorização de importação e consulta;
- resultado histórico preso à versão correta.

## 13. Critério de pronto

O módulo só está pronto quando consegue importar o arquivo real autorizado, explicar rejeições, consultar offline, exibir fonte/versão/hash, vincular a dispositivo com auditoria e repetir o mesmo resultado usando o mesmo snapshot.

Uma tela que apenas valida 15 dígitos não é o módulo de consulta.
