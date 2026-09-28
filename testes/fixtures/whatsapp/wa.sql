-- FIXTURE SINTÉTICA: wa.db do WhatsApp Android (tabela `wa_contacts`).
--
-- Nenhum dado é real. Nomes e números foram inventados para teste. Os nomes de
-- tabela e colunas seguem a referência pública do aplicativo e ainda precisam ser
-- conferidos com um banco real de aparelho de teste. Esquema reduzido.
--
-- `display_name` é o nome gravado na agenda do aparelho; `wa_name` é o nome que a
-- própria pessoa definiu no perfil do WhatsApp.

CREATE TABLE wa_contacts (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    jid TEXT NOT NULL,
    is_whatsapp_user BOOLEAN NOT NULL,
    status TEXT,
    status_timestamp INTEGER,
    number TEXT,
    raw_contact_id INTEGER,
    display_name TEXT,
    phone_type INTEGER,
    phone_label TEXT,
    unseen_msg_count INTEGER,
    photo_ts INTEGER,
    thumb_ts INTEGER,
    photo_id_timestamp INTEGER,
    given_name TEXT,
    family_name TEXT,
    wa_name TEXT,
    sort_name TEXT,
    nickname TEXT,
    company TEXT,
    title TEXT
);

INSERT INTO wa_contacts (jid, is_whatsapp_user, status, status_timestamp, number, raw_contact_id,
                         display_name, given_name, family_name, wa_name, sort_name) VALUES
    ('5511900000011@s.whatsapp.net', 1, 'Recado sintético', 1767000000000, '+55 11 90000-0011', 101,
        'Ana Sintética', 'Ana', 'Sintética', 'Ana S.', 'Ana Sintética'),
    ('5511900000012@s.whatsapp.net', 1, NULL, NULL, '+55 11 90000-0012', 102,
        'Bruno Sintético', 'Bruno', 'Sintético', NULL, 'Bruno Sintético'),
    -- Sem nome na agenda: só o nome de perfil.
    ('5521900000013@s.whatsapp.net', 1, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, 'Carla (perfil sintético)', NULL),
    ('120363000000000001@g.us', 0, NULL, NULL, NULL, NULL,
        'Grupo sintético de teste', NULL, NULL, NULL, 'Grupo sintético de teste');
