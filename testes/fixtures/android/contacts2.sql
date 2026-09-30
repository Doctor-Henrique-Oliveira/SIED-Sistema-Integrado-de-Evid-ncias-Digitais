-- FIXTURE SINTÉTICA: contacts2.db do Android (agenda do provedor de contatos).
--
-- Nenhum dado é real. Nomes, números (55xx9000000xx), e-mails (@example.com) e textos
-- foram inventados para teste. Os nomes de tabelas e colunas seguem a documentação
-- pública do Android (`ContactsContract`, AOSP), em esquema reduzido, e ainda precisam
-- ser conferidos com um banco de aparelho de teste. Os ids de `mimetypes` variam de
-- aparelho para aparelho: o parser sempre lê a tabela.

CREATE TABLE accounts (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_name TEXT,
    account_type TEXT,
    data_set TEXT
);

CREATE TABLE mimetypes (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    mimetype TEXT NOT NULL
);

CREATE TABLE contacts (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    name_raw_contact_id INTEGER,
    photo_id INTEGER,
    custom_ringtone TEXT,
    send_to_voicemail INTEGER NOT NULL DEFAULT 0,
    times_contacted INTEGER NOT NULL DEFAULT 0,
    last_time_contacted INTEGER,
    starred INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    has_phone_number INTEGER NOT NULL DEFAULT 0,
    lookup TEXT,
    contact_last_updated_timestamp INTEGER
);

CREATE TABLE raw_contacts (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER,
    sourceid TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    dirty INTEGER NOT NULL DEFAULT 0,
    deleted INTEGER NOT NULL DEFAULT 0,
    contact_id INTEGER,
    aggregation_mode INTEGER NOT NULL DEFAULT 0,
    times_contacted INTEGER NOT NULL DEFAULT 0,
    last_time_contacted INTEGER,
    starred INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    display_name TEXT,
    display_name_alt TEXT,
    display_name_source INTEGER NOT NULL DEFAULT 0,
    sort_key TEXT
);

CREATE TABLE data (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id INTEGER,
    mimetype_id INTEGER NOT NULL,
    raw_contact_id INTEGER NOT NULL,
    is_read_only INTEGER NOT NULL DEFAULT 0,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_super_primary INTEGER NOT NULL DEFAULT 0,
    data_version INTEGER NOT NULL DEFAULT 0,
    data1 TEXT, data2 TEXT, data3 TEXT, data4 TEXT, data5 TEXT,
    data6 TEXT, data7 TEXT, data8 TEXT, data9 TEXT, data10 TEXT,
    data11 TEXT, data12 TEXT, data13 TEXT, data14 TEXT, data15 TEXT
);

INSERT INTO accounts (_id, account_name, account_type) VALUES
    (1, 'analista.sintetico@example.com', 'com.google'),
    (2, 'WhatsApp', 'com.whatsapp'),
    -- Contatos só do aparelho: sem conta.
    (3, NULL, NULL);

INSERT INTO mimetypes (_id, mimetype) VALUES
    (1, 'vnd.android.cursor.item/email_v2'),
    (3, 'vnd.android.cursor.item/nickname'),
    (4, 'vnd.android.cursor.item/organization'),
    (5, 'vnd.android.cursor.item/phone_v2'),
    (7, 'vnd.android.cursor.item/name'),
    (8, 'vnd.android.cursor.item/postal-address_v2'),
    (10, 'vnd.android.cursor.item/photo'),
    (12, 'vnd.android.cursor.item/note'),
    (13, 'vnd.android.cursor.item/website'),
    (14, 'vnd.android.cursor.item/vnd.com.whatsapp.profile');

INSERT INTO contacts (_id, name_raw_contact_id, times_contacted, last_time_contacted, starred,
                      has_phone_number, contact_last_updated_timestamp) VALUES
    (1, 1, 5, 1767258000000, 1, 1, 1767000000000),
    (2, 2, 1, 1767261600000, 0, 1, 1767000100000),
    (3, 3, 0, NULL, 0, 0, 1767000200000),
    (4, 4, 0, NULL, 0, 1, 1767000300000),
    (5, 6, 0, NULL, 0, 1, 1767000400000);

INSERT INTO raw_contacts (_id, account_id, deleted, contact_id, times_contacted,
                          last_time_contacted, starred, display_name, sort_key) VALUES
    (1, 1, 0, 1, 5, 1767258000000, 1, 'Ana Sintética', 'Ana Sintética'),
    (2, 1, 0, 2, 1, 1767261600000, 0, 'Bruno Sintético', 'Bruno Sintético'),
    (3, 3, 0, 3, 0, NULL, 0, 'Dora Sintética', 'Dora Sintética'),
    -- Apagado no aparelho, ainda não removido pela sincronização.
    (4, 1, 1, 4, 0, NULL, 0, 'Eduardo Sintético', 'Eduardo Sintético'),
    -- A mesma Ana, pela conta do WhatsApp (o Android agrupa no contato 1).
    (5, 2, 0, 1, 0, NULL, 0, 'Ana Sintética', 'Ana Sintética'),
    -- Só um número, sem nome.
    (6, 3, 0, 5, 0, NULL, 0, NULL, NULL);

INSERT INTO data (_id, mimetype_id, raw_contact_id, is_primary, data1, data2, data3, data4, data15) VALUES
    (1, 7, 1, 0, 'Ana Sintética', 'Ana', 'Sintética', NULL, NULL),
    (2, 5, 1, 1, '+55 11 90000-0011', '2', NULL, '+5511900000011', NULL),
    (3, 1, 1, 0, 'ana.sintetica@example.com', '1', NULL, NULL, NULL),
    -- Miniatura da foto (quatro bytes inventados): não interpretada.
    (4, 10, 1, 0, NULL, NULL, NULL, NULL, X'FFD8FFE0'),
    (5, 7, 2, 0, 'Bruno Sintético', 'Bruno', 'Sintético', NULL, NULL),
    (6, 5, 2, 0, '11 90000-0012', '3', NULL, '+5511900000012', NULL),
    (7, 5, 2, 0, '+55 11 3000-0012', '0', 'Escritório sintético', '+551130000012', NULL),
    (8, 7, 3, 0, 'Dora Sintética', 'Dora', 'Sintética', NULL, NULL),
    (9, 4, 3, 0, 'Empresa Sintética Ltda', NULL, NULL, 'Analista', NULL),
    (10, 12, 3, 0, 'Observação sintética de teste', NULL, NULL, NULL, NULL),
    (11, 3, 3, 0, 'Dorinha', NULL, NULL, NULL, NULL),
    (12, 8, 3, 0, 'Rua Sintética, 100 - Cidade Teste', '1', NULL, NULL, NULL),
    (13, 7, 4, 0, 'Eduardo Sintético', 'Eduardo', 'Sintético', NULL, NULL),
    (14, 5, 4, 0, '+5571900000017', '2', NULL, '+5571900000017', NULL),
    (15, 5, 5, 0, '+55 11 90000-0011', '2', NULL, '+5511900000011', NULL),
    (16, 14, 5, 0, '5511900000011@s.whatsapp.net', 'WhatsApp', 'Mensagem para +55 11 90000-0011', NULL, NULL),
    (17, 5, 6, 0, '+5561900000016', '2', NULL, '+5561900000016', NULL),
    -- Linha órfã (contato inexistente): vira aviso, não é interpretada.
    (18, 5, 999, 0, '+5500900000000', '2', NULL, NULL, NULL),
    (19, 13, 3, 0, 'https://exemplo.invalid/dora', NULL, NULL, NULL, NULL);
