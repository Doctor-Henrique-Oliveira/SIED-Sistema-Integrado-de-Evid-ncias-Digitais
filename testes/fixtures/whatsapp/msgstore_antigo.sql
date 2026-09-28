-- FIXTURE SINTÉTICA: msgstore.db do WhatsApp Android, esquema "antigo" (tabela `messages`).
--
-- Nenhum dado é real. Nomes, números, textos e arquivos foram inventados para teste.
-- Os nomes de tabelas e colunas seguem a referência pública do aplicativo e ainda
-- precisam ser conferidos com um banco real de aparelho de teste. O esquema é um
-- subconjunto: só as colunas lidas pelo parser e algumas extras, para provar que a
-- linha inteira é preservada em `valores_brutos`.
--
-- Números: faixa +55 11 9000-00xx, usada só em fixtures do SIED.
-- Datas: a partir de 2026-01-02 12:00:00 UTC (1767355200000 ms).

CREATE TABLE messages (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_remote_jid TEXT NOT NULL,
    key_from_me INTEGER,
    key_id TEXT NOT NULL,
    status INTEGER,
    needs_push INTEGER,
    data TEXT,
    timestamp INTEGER,
    media_url TEXT,
    media_mime_type TEXT,
    media_wa_type TEXT,
    media_size INTEGER,
    media_name TEXT,
    media_caption TEXT,
    media_hash TEXT,
    media_duration INTEGER,
    origin INTEGER,
    latitude REAL,
    longitude REAL,
    thumb_image TEXT,
    remote_resource TEXT,
    received_timestamp INTEGER,
    send_timestamp INTEGER,
    receipt_server_timestamp INTEGER,
    receipt_device_timestamp INTEGER,
    raw_data BLOB,
    recipient_count INTEGER,
    quoted_row_id INTEGER,
    forwarded INTEGER
);

CREATE TABLE messages_quotes (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_remote_jid TEXT NOT NULL,
    key_from_me INTEGER,
    key_id TEXT NOT NULL,
    status INTEGER,
    data TEXT,
    timestamp INTEGER,
    media_wa_type TEXT,
    remote_resource TEXT
);

CREATE TABLE chat_list (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_remote_jid TEXT UNIQUE,
    message_table_id INTEGER,
    subject TEXT,
    creation INTEGER,
    last_read_message_table_id INTEGER,
    archived INTEGER,
    sort_timestamp INTEGER
);

CREATE TABLE group_participants (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    gjid TEXT NOT NULL,
    jid TEXT NOT NULL,
    admin INTEGER,
    pending INTEGER,
    sent_sender_key INTEGER
);

CREATE TABLE message_media (
    message_row_id INTEGER PRIMARY KEY,
    chat_row_id INTEGER,
    file_path TEXT,
    file_size INTEGER,
    mime_type TEXT,
    media_name TEXT,
    media_duration INTEGER
);

INSERT INTO chat_list (_id, key_remote_jid, subject, creation, archived, sort_timestamp) VALUES
    (1, '5511900000011@s.whatsapp.net', NULL, 1767355000000, 0, 1767355500000),
    (2, '120363000000000001@g.us', 'Grupo sintético (esquema antigo)', 1767354000000, 0, 1767355800000);

INSERT INTO group_participants (gjid, jid, admin, pending, sent_sender_key) VALUES
    ('120363000000000001@g.us', '5511900000012@s.whatsapp.net', 1, 0, 1),
    ('120363000000000001@g.us', '5521900000013@s.whatsapp.net', 0, 0, 1);

INSERT INTO messages_quotes (_id, key_remote_jid, key_from_me, key_id, status, data, timestamp,
                             media_wa_type, remote_resource) VALUES
    (1, '5511900000011@s.whatsapp.net', 0, 'ANTIGO000000001', 0, 'Oi (esquema antigo, sintético)',
        1767355200000, '0', NULL);

INSERT INTO messages (_id, key_remote_jid, key_from_me, key_id, status, data, timestamp,
                      media_mime_type, media_wa_type, media_size, media_name, media_caption,
                      latitude, longitude, remote_resource, received_timestamp, quoted_row_id,
                      forwarded) VALUES
    (1, '5511900000011@s.whatsapp.net', 0, 'ANTIGO000000001', 0,
        'Oi (esquema antigo, sintético)', 1767355200000, NULL, '0', 0, NULL, NULL,
        0, 0, NULL, 1767355201000, 0, 0),
    (2, '5511900000011@s.whatsapp.net', 1, 'ANTIGO000000002', 13,
        'Resposta com citação', 1767355260000, NULL, '0', 0, NULL, NULL,
        0, 0, NULL, 0, 1, 0),
    (3, '5511900000011@s.whatsapp.net', 0, 'ANTIGO000000003', 0,
        NULL, 1767355320000, 'image/jpeg', '1', 64, NULL, 'Imagem sintética antiga',
        0, 0, NULL, 1767355321000, 0, 0),
    (4, '5511900000011@s.whatsapp.net', 0, 'ANTIGO000000004', 0,
        NULL, 1767355380000, NULL, '5', 0, 'Local sintético', NULL,
        -22.906847, -43.172897, NULL, 1767355381000, 0, 0),
    (5, '5511900000011@s.whatsapp.net', 0, 'ANTIGO000000005', 0,
        NULL, 1767355440000, NULL, '15', 0, NULL, NULL,
        0, 0, NULL, 1767355441000, 0, 0),
    (6, '120363000000000001@g.us', 0, 'ANTIGO000000006', 6,
        NULL, 1767354000000, NULL, '0', 1, NULL, NULL,
        0, 0, '5511900000012@s.whatsapp.net', 0, 0, 0),
    (7, '120363000000000001@g.us', 0, 'ANTIGO000000007', 0,
        'Mensagem no grupo antigo', 1767355700000, NULL, '0', 0, NULL, NULL,
        0, 0, '5521900000013@s.whatsapp.net', 1767355701000, 0, 0),
    (8, '120363000000000001@g.us', 1, 'ANTIGO000000008', 5,
        'Encaminhada no grupo antigo', 1767355800000, NULL, '0', 0, NULL, NULL,
        0, 0, NULL, 0, 0, 1);

INSERT INTO message_media (message_row_id, chat_row_id, file_path, file_size, mime_type,
                           media_name, media_duration) VALUES
    (3, 1, 'Media/WhatsApp Images/IMG-20260102-WA0009.jpg', 64, 'image/jpeg', NULL, 0);
