# Fixtures SINTÉTICAS do Android (agenda e registro de chamadas)

**Nenhum dado aqui é real.** Nomes, números (`55xx9000000xx`), e-mails (`@example.com`), endereços
e textos foram inventados para teste. Os arquivos são scripts SQL, não bancos, para que o conteúdo
fique legível e revisável no Git.

| Arquivo | Vira | Tabelas |
|---|---|---|
| `contacts2.sql` | `contacts2.db` | `accounts`, `mimetypes`, `contacts`, `raw_contacts`, `data` |
| `calllog.sql` | `calllog.db` | `calls` |

Os nomes de tabelas e colunas seguem a documentação pública do Android (`ContactsContract` e
`CallLog.Calls`, AOSP), em esquema reduzido, e **não foram conferidos com um banco real** de aparelho
de teste. Nada aqui representa a estrutura de pastas de qualquer ferramenta de extração (Avilla,
IPED...).

Os números de Ana (`+55 11 90000-0011`) e Bruno (`+55 11 90000-0012`) são os mesmos da fixture
`../whatsapp/wa.sql`, para exercitar a ligação de chamadas à agenda pelo mesmo número.

Usados por:
- `crates/sied-ingestao/src/parsers/android_contatos.rs` e `android_chamadas.rs` (parsers);
- `crates/sied-armazenamento/tests/agenda_chamadas.rs` (importação completa no banco do caso);
- `testes/gerador/extracao_sintetica.py --android` (extração sintética para o app real).
