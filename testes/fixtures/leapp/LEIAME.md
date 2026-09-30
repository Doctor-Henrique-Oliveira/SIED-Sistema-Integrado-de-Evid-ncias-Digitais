# Fixtures SINTÉTICAS do ALEAPP e do iLEAPP

Nada aqui veio de um relatório real. Tudo foi gravado por `montar.py`, que imita só o que o
código-fonte público dos projetos abrignoni/ALEAPP e abrignoni/iLEAPP define
(`scripts/ilapfuncs.py`, função `tsv`; `scripts/lavafuncs.py`, projeto `_lava_data.lava`):

- `aleapp/_TSV Exports/`: "SMS Messages", "Call logs " (com o espaço final do nome real do
  módulo, gravado duas vezes, então o cabeçalho se repete com outro BOM) e "Cache Location".
  `csv.writer` do Python com TAB, UTF-8 com BOM, CRLF; um campo com TAB, aspas e quebra de linha;
  datas com e sem fuso.
- `aleapp/_lava_data.lava` e `ileapp/_lava_data.lava`: só os campos lidos pelo SIED
  (`parser_info.leapp_name`, `leapp_version`, `param_input`, `param_type`) e os demais com
  valores de exemplo.
- `ileapp/_TSV Exports/SMS.tsv`: as 8 primeiras colunas do módulo "SMS" do iLEAPP.
- `_HTML/index.html`: marcador; o HTML não é lido.

Números de telefone, textos, coordenadas e datas são inventados. Os arquivos têm `-text` em
`.gitattributes` para o git não trocar o fim de linha. Para regravar:
`python3 testes/fixtures/leapp/montar.py`.
