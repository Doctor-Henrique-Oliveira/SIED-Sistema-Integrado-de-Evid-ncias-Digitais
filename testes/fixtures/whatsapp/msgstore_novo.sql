-- FIXTURE SINTÉTICA: msgstore.db do WhatsApp Android, esquema "novo" (tabela `message`).
--
-- Nenhum dado é real. Nomes, números, textos e arquivos foram inventados para teste.
-- Os nomes de tabelas e colunas seguem a referência pública do aplicativo e ainda
-- precisam ser conferidos com um banco real de aparelho de teste. O esquema é um
-- subconjunto: só as colunas lidas pelo parser e algumas extras, para provar que a
-- linha inteira é preservada em `valores_brutos`.
--
-- Números: faixa +55 11 9000-00xx, usada só em fixtures do SIED.
-- Datas: a partir de 2026-01-02 12:00:00 UTC (1767355200000 ms).

CREATE TABLE jid (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    user TEXT NOT NULL,
    server TEXT NOT NULL,
    agent INTEGER,
    device INTEGER,
    type INTEGER,
    raw_string TEXT
);

CREATE TABLE chat (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    jid_row_id INTEGER UNIQUE,
    hidden INTEGER,
    subject TEXT,
    created_timestamp INTEGER,
    display_message_row_id INTEGER,
    last_message_row_id INTEGER,
    archived INTEGER,
    sort_timestamp INTEGER,
    group_type INTEGER
);

CREATE TABLE message (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_row_id INTEGER NOT NULL,
    from_me INTEGER NOT NULL,
    key_id TEXT NOT NULL,
    sender_jid_row_id INTEGER,
    status INTEGER,
    broadcast INTEGER,
    recipient_count INTEGER,
    participant_hash TEXT,
    origination_flags INTEGER,
    origin INTEGER,
    timestamp INTEGER,
    received_timestamp INTEGER,
    receipt_server_timestamp INTEGER,
    message_type INTEGER,
    text_data TEXT,
    starred INTEGER,
    lookup_tables INTEGER,
    sort_id INTEGER NOT NULL DEFAULT 0,
    message_add_on_flags INTEGER
);

CREATE TABLE message_media (
    message_row_id INTEGER PRIMARY KEY,
    chat_row_id INTEGER,
    file_path TEXT,
    file_size INTEGER,
    mime_type TEXT,
    media_name TEXT,
    media_duration INTEGER,
    file_hash TEXT,
    width INTEGER,
    height INTEGER
);

CREATE TABLE message_quoted (
    message_row_id INTEGER PRIMARY KEY,
    chat_row_id INTEGER,
    parent_message_chat_row_id INTEGER,
    from_me INTEGER,
    sender_jid_row_id INTEGER,
    key_id TEXT,
    timestamp INTEGER,
    message_type INTEGER,
    text_data TEXT
);

CREATE TABLE message_location (
    message_row_id INTEGER PRIMARY KEY,
    chat_row_id INTEGER,
    latitude REAL,
    longitude REAL,
    place_name TEXT,
    place_address TEXT,
    url TEXT
);

CREATE TABLE message_vcard (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_row_id INTEGER,
    vcard TEXT
);

CREATE TABLE message_forwarded (
    message_row_id INTEGER PRIMARY KEY,
    forward_score INTEGER
);

CREATE TABLE message_system (
    message_row_id INTEGER PRIMARY KEY,
    action_type INTEGER NOT NULL
);

CREATE TABLE group_participant_user (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_jid_row_id INTEGER NOT NULL,
    user_jid_row_id INTEGER NOT NULL,
    rank INTEGER,
    pending INTEGER
);

INSERT INTO jid (_id, user, server, agent, device, type, raw_string) VALUES
    (1, '5511900000011', 's.whatsapp.net', 0, 0, 0, '5511900000011@s.whatsapp.net'),
    (2, '5511900000012', 's.whatsapp.net', 0, 0, 0, '5511900000012@s.whatsapp.net'),
    (3, '5521900000013', 's.whatsapp.net', 0, 0, 0, '5521900000013@s.whatsapp.net'),
    (4, '120363000000000001', 'g.us', 0, 0, 1, '120363000000000001@g.us'),
    (5, 'status', 'broadcast', 0, 0, 5, 'status@broadcast');

-- 1: conversa individual com "Ana"; 2: grupo; 3: conversa individual com "Bruno".
INSERT INTO chat (_id, jid_row_id, hidden, subject, created_timestamp, archived, sort_timestamp, group_type) VALUES
    (1, 1, 0, NULL, 1767355000000, 0, 1767356100000, 0),
    (2, 4, 0, 'Grupo sintético de teste', 1767354000000, 0, 1767355900000, 0),
    (3, 2, 0, NULL, 1767355500000, 1, 1767356000000, 0);

INSERT INTO group_participant_user (group_jid_row_id, user_jid_row_id, rank, pending) VALUES
    (4, 2, 2, 0),
    (4, 3, 0, 0);

INSERT INTO message (_id, chat_row_id, from_me, key_id, sender_jid_row_id, status, broadcast,
                     timestamp, received_timestamp, receipt_server_timestamp, message_type,
                     text_data, starred, sort_id) VALUES
    -- conversa com Ana
    (1, 1, 0, 'SINT0000000001', 0, 0, 0, 1767355200000, 1767355201000, 0, 0,
        'Oi! Mensagem sintética do WhatsApp', 0, 1),
    (2, 1, 1, 'SINT0000000002', 0, 13, 0, 1767355260000, 0, 1767355261000, 0,
        'Oi, Ana. Tudo certo?', 0, 2),
    (3, 1, 0, 'SINT0000000003', 0, 0, 0, 1767355320000, 1767355321000, 0, 1,
        'Foto sintética', 1, 3),
    (4, 1, 1, 'SINT0000000004', 0, 5, 0, 1767355380000, 0, 1767355381000, 0,
        'Recebi a foto', 0, 4),
    (5, 1, 0, 'SINT0000000005', 0, 0, 0, 1767355440000, 1767355441000, 0, 5,
        NULL, 0, 5),
    (6, 1, 0, 'SINT0000000006', 0, 0, 0, 1767355500000, 1767355501000, 0, 15,
        NULL, 0, 6),
    (7, 1, 0, 'SINT0000000007', 0, 0, 0, 1767355560000, 1767355561000, 0, 4,
        'Contato Sintético', 0, 7),
    (8, 1, 0, 'SINT0000000008', 0, 0, 0, 1767355620000, 1767355621000, 0, 99,
        NULL, 0, 8),
    -- grupo
    (9, 2, 0, 'SINT0000000009', 0, 0, 0, 1767354000000, 0, 0, 7,
        NULL, 0, 9),
    (10, 2, 0, 'SINT0000000010', 2, 0, 0, 1767355700000, 1767355701000, 0, 0,
        'Bom dia, grupo (sintético)', 0, 10),
    (11, 2, 0, 'SINT0000000011', 3, 0, 0, 1767355800000, 1767355801000, 0, 9,
        'Segue o documento', 0, 11),
    (12, 2, 1, 'SINT0000000012', 0, 4, 0, 1767355900000, 0, 1767355901000, 0,
        'Mensagem encaminhada (sintética)', 0, 12),
    -- conversa com Bruno: áudio cujo arquivo não está na extração
    (13, 3, 0, 'SINT0000000013', 0, 0, 0, 1767356000000, 1767356001000, 0, 2,
        NULL, 0, 13);

INSERT INTO message_media (message_row_id, chat_row_id, file_path, file_size, mime_type,
                           media_name, media_duration, file_hash, width, height) VALUES
    (3, 1, 'Media/WhatsApp Images/IMG-20260102-WA0001.jpg', 64, 'image/jpeg', NULL, 0,
        'c2ludGV0aWNvLWltYWdlbQ==', 640, 480),
    (11, 2, 'Media/WhatsApp Documents/relatorio-sintetico.pdf', 128, 'application/pdf',
        'relatorio-sintetico.pdf', 0, 'c2ludGV0aWNvLWRvYw==', 0, 0),
    (13, 3, 'Media/WhatsApp Voice Notes/202601/PTT-20260102-WA0002.opus', 32, 'audio/ogg; codecs=opus',
        NULL, 7, 'c2ludGV0aWNvLWF1ZGlv', 0, 0);

INSERT INTO message_quoted (message_row_id, chat_row_id, parent_message_chat_row_id, from_me,
                            sender_jid_row_id, key_id, timestamp, message_type, text_data) VALUES
    (4, 1, 1, 0, 0, 'SINT0000000003', 1767355320000, 1, 'Foto sintética');

INSERT INTO message_location (message_row_id, chat_row_id, latitude, longitude, place_name,
                              place_address, url) VALUES
    (5, 1, -23.55052, -46.633308, 'Local sintético', 'Endereço sintético, 100', NULL);

INSERT INTO message_vcard (message_row_id, vcard) VALUES
    (7, 'BEGIN:VCARD
VERSION:3.0
FN:Contato Sintético
TEL;type=CELL:+55 11 90000-0099
END:VCARD');

INSERT INTO message_forwarded (message_row_id, forward_score) VALUES (12, 1);

INSERT INTO message_system (message_row_id, action_type) VALUES (9, 11);
