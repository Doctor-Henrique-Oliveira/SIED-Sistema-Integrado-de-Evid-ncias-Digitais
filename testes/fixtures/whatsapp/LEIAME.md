# Fixtures SINTÉTICAS do WhatsApp Android

**Nenhum dado aqui é real.** Nomes, números (`55119000000xx`), textos, coordenadas e caminhos foram
inventados para teste. Os arquivos são scripts SQL, não bancos, para que o conteúdo fique legível
e revisável no Git.

| Arquivo | Vira | Esquema |
|---|---|---|
| `msgstore_novo.sql` | `msgstore.db` | novo: `message`, `chat`, `jid` e auxiliares (`message_media`, `message_quoted`, `message_location`, `message_vcard`, `message_forwarded`, `message_system`, `group_participant_user`) |
| `msgstore_antigo.sql` | `msgstore.db` | antigo: `messages`, `chat_list`, `group_participants`, `messages_quotes`, `message_media` |
| `wa.sql` | `wa.db` | `wa_contacts` |

Os nomes de tabelas e colunas seguem a referência pública do app e **não foram conferidos com um
banco real** de aparelho de teste. Nada aqui representa a estrutura de pastas de qualquer
ferramenta de extração (Avilla, IPED...).

Usados por:
- `crates/sied-ingestao/src/parsers/whatsapp_android/testes.rs` (parser);
- `crates/sied-armazenamento/tests/whatsapp.rs` (importação completa no banco do caso);
- `testes/gerador/extracao_sintetica.py --whatsapp` (extração sintética para o app real).
