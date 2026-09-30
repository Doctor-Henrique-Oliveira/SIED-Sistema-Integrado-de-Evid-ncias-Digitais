-- FIXTURE SINTÉTICA: calllog.db do Android (tabela `calls` do provedor de contatos).
--
-- Nenhum dado é real. Números (55xx9000000xx), nomes e datas foram inventados para
-- teste. Os nomes de tabela e colunas seguem a documentação pública do Android
-- (`CallLog.Calls`, AOSP) e ainda precisam ser conferidos com um banco de aparelho de
-- teste. Datas em milissegundos UTC a partir de 2026-01-01 09:00 UTC.

CREATE TABLE calls (
    _id INTEGER PRIMARY KEY AUTOINCREMENT,
    number TEXT,
    presentation INTEGER NOT NULL DEFAULT 1,
    post_dial_digits TEXT NOT NULL DEFAULT '',
    via_number TEXT NOT NULL DEFAULT '',
    date INTEGER,
    duration INTEGER,
    data_usage INTEGER,
    type INTEGER,
    features INTEGER NOT NULL DEFAULT 0,
    subscription_component_name TEXT,
    subscription_id TEXT,
    phone_account_address TEXT,
    phone_account_hidden INTEGER NOT NULL DEFAULT 0,
    sub_id INTEGER DEFAULT -1,
    new INTEGER,
    name TEXT,
    numbertype INTEGER,
    numberlabel TEXT,
    countryiso TEXT,
    voicemail_uri TEXT,
    is_read INTEGER,
    geocoded_location TEXT,
    lookup_uri TEXT,
    matched_number TEXT,
    normalized_number TEXT,
    photo_id INTEGER NOT NULL DEFAULT 0,
    photo_uri TEXT,
    formatted_number TEXT,
    add_for_all_users INTEGER NOT NULL DEFAULT 1,
    last_modified INTEGER DEFAULT 0,
    _data TEXT,
    has_content INTEGER,
    mime_type TEXT,
    source_data TEXT,
    source_package TEXT,
    transcription TEXT,
    transcription_state INTEGER NOT NULL DEFAULT 0,
    state INTEGER,
    dirty INTEGER NOT NULL DEFAULT 0,
    deleted INTEGER NOT NULL DEFAULT 0,
    backed_up INTEGER NOT NULL DEFAULT 0,
    restored INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    is_omtp_voicemail INTEGER NOT NULL DEFAULT 0
);

INSERT INTO calls (_id, number, presentation, date, duration, type, features,
                   subscription_component_name, subscription_id, new, name, numbertype,
                   countryiso, is_read, geocoded_location, normalized_number,
                   formatted_number, last_modified) VALUES
    -- Recebida de contato da agenda (nome guardado pelo aparelho na hora da chamada).
    (1, '+5511900000011', 1, 1767258000000, 125, 1, 0,
        'com.android.phone/com.android.services.telephony.TelephonyConnectionService', '1',
        0, 'Ana Sintética', 2, 'BR', 1, 'Brasil', '+5511900000011', '+55 11 90000-0011',
        1767258125000),
    -- Efetuada, com vídeo e Wi-Fi; número gravado com máscara.
    (2, '(11) 90000-0012', 1, 1767261600000, 42, 2, 9,
        'com.android.phone/com.android.services.telephony.TelephonyConnectionService', '1',
        0, 'Bruno Sintético', 2, 'BR', 1, 'Brasil', '+5511900000012', '(11) 90000-0012',
        1767261642000),
    -- Perdida de número fora da agenda, ainda não vista.
    (3, '+5521900000013', 1, 1767265200000, 0, 3, 0, NULL, '1', 1, NULL, NULL, 'BR', 0,
        'Rio de Janeiro', '+5521900000013', NULL, 1767265200000),
    -- Rejeitada.
    (4, '+5531900000014', 1, 1767268800000, 0, 5, 0, NULL, '2', 0, NULL, NULL, 'BR', 1,
        NULL, '+5531900000014', NULL, 1767268800000),
    -- Bloqueada.
    (5, '+5541900000015', 1, 1767272400000, 0, 6, 0, NULL, '2', 0, NULL, NULL, 'BR', 1,
        NULL, NULL, NULL, 1767272400000),
    -- Número restrito pela operadora: sem número.
    (6, '', 2, 1767276000000, 30, 1, 0, NULL, '1', 0, NULL, NULL, 'BR', 1,
        NULL, NULL, NULL, 1767276030000),
    -- Código de tipo que o SIED não conhece (preservado, não interpretado).
    (7, '+5511900000011', 1, 1767279600000, 10, 99, 0, NULL, '1', 0, 'Ana Sintética', 2, 'BR', 1,
        NULL, '+5511900000011', NULL, 1767279610000),
    -- Número especial das versões antigas (-2 = privado): não é telefone.
    (8, '-2', 1, 1767283200000, 5, 1, 0, NULL, '1', 0, NULL, NULL, 'BR', 1,
        NULL, NULL, NULL, 1767283205000),
    -- Perdida, sem nome guardado; o número gravado difere da agenda, mas o normalizado
    -- pelo aparelho é idêntico ao do contato Eduardo (apagado na agenda).
    (9, '071900000017', 1, 1767286800000, 0, 3, 0, NULL, '1', 1, NULL, NULL, 'BR', 0,
        NULL, '+5571900000017', NULL, 1767286800000);
