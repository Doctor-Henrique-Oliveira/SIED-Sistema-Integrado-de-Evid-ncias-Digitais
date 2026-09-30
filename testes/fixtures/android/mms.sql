-- MMS SINTÉTICOS do Android (tabelas pdu, addr e part do mmssms.db).
-- Nenhum dado é real: números 55xx9000000xx, textos e arquivos inventados.
-- Esquema reduzido pela documentação pública (Telephony.Mms, AOSP); não conferido
-- com um banco de aparelho de teste. Aplicado sobre um mmssms.db que já tem a tabela sms.

CREATE TABLE pdu (
    _id INTEGER PRIMARY KEY AUTOINCREMENT, thread_id INTEGER, date INTEGER, date_sent INTEGER DEFAULT 0,
    msg_box INTEGER, read INTEGER DEFAULT 0, m_id TEXT, sub TEXT, sub_cs INTEGER, ct_t TEXT, ct_l TEXT,
    exp INTEGER, m_cls TEXT, m_type INTEGER, v INTEGER, m_size INTEGER, pri INTEGER, rr INTEGER,
    rpt_a INTEGER, resp_st INTEGER, st INTEGER, tr_id TEXT, retr_st INTEGER, retr_txt TEXT,
    retr_txt_cs INTEGER, read_status INTEGER, ct_cls INTEGER, resp_txt TEXT, d_tm INTEGER,
    d_rpt INTEGER, locked INTEGER DEFAULT 0, sub_id INTEGER DEFAULT -1, seen INTEGER DEFAULT 0,
    creator TEXT, text_only INTEGER DEFAULT 0
);
CREATE TABLE addr (
    _id INTEGER PRIMARY KEY, msg_id INTEGER, contact_id INTEGER, address TEXT, type INTEGER,
    charset INTEGER
);
CREATE TABLE part (
    _id INTEGER PRIMARY KEY AUTOINCREMENT, mid INTEGER, seq INTEGER DEFAULT 0, ct TEXT, name TEXT,
    chset INTEGER, cd TEXT, fn TEXT, cid TEXT, cl TEXT, ctt_s INTEGER, ctt_t TEXT, _data TEXT, text TEXT
);

-- date em SEGUNDOS (ao contrário do sms, que usa milissegundos).
-- 1: recebida na conversa 1 (mesmo thread dos SMS de +5511900000001): foto + texto.
INSERT INTO pdu (_id, thread_id, date, date_sent, msg_box, read, m_id, sub, m_type, m_size, seen, text_only)
VALUES (1, 1, 1767232800, 1767232790, 1, 1, 'SINTETICO-MID-1', NULL, 132, 2048, 1, 0);
-- 2: enviada em grupo (conversa 4) só com texto e assunto, com uma cópia (CC).
INSERT INTO pdu (_id, thread_id, date, msg_box, read, m_id, sub, sub_cs, m_type, seen, text_only)
VALUES (2, 4, 1767236400, 2, 1, 'SINTETICO-MID-2', 'Aviso sintético', 106, 128, 1, 1);
-- 3: recebida com duas mídias (áudio e imagem): só a primeira vira o anexo.
INSERT INTO pdu (_id, thread_id, date, msg_box, read, m_type, seen)
VALUES (3, 1, 1767240000, 1, 0, 132, 0);
-- 4: notificação de MMS ainda não baixado (sem partes).
INSERT INTO pdu (_id, thread_id, date, msg_box, read, sub, m_type, m_size, exp, ct_l)
VALUES (4, 2, 1767243600, 1, 0, 'Mensagem multimídia sintética', 130, 30000, 1767848400, 'http://mmsc.invalido/SINTETICO');
-- 5: data absurda (fica só no valor original).
INSERT INTO pdu (_id, thread_id, date, msg_box, m_type) VALUES (5, 1, 99999999999999, 1, 132);

-- Endereços: 137 = de (FROM), 151 = para (TO), 130 = cópia (CC).
INSERT INTO addr VALUES (1, 1, NULL, '+5511900000001', 137, 106);
INSERT INTO addr VALUES (2, 1, NULL, 'insert-address-token', 151, 106);
INSERT INTO addr VALUES (3, 2, NULL, 'insert-address-token', 137, 106);
INSERT INTO addr VALUES (4, 2, NULL, '+5521900000002', 151, 106);
INSERT INTO addr VALUES (5, 2, NULL, '+5531900000003', 151, 106);
INSERT INTO addr VALUES (6, 3, NULL, '+5511900000001', 137, 106);
INSERT INTO addr VALUES (7, 2, NULL, '+5541900000015', 130, 106);
INSERT INTO addr VALUES (8, 4, NULL, '+5521900000002', 137, 106);
INSERT INTO addr VALUES (9, 5, NULL, '+5511900000001', 137, 106);

-- Partes: application/smil é o layout; text/plain traz o texto na coluna text.
INSERT INTO part (_id, mid, seq, ct, name, chset, cid, cl, _data, text)
VALUES (1, 1, -1, 'application/smil', NULL, NULL, '<smil>', 'smil.xml', NULL, '<smil><body><par><img src="IMG_0002.png"/><text src="text_0.txt"/></par></body></smil>');
INSERT INTO part (_id, mid, seq, ct, name, chset, cid, cl, _data, text)
VALUES (2, 1, 0, 'image/png', 'IMG_0002.png', NULL, '<img>', 'IMG_0002.png', '/data/user_de/0/com.android.providers.telephony/app_parts/PART_1767232800000_IMG_0002.png', NULL);
INSERT INTO part (_id, mid, seq, ct, name, chset, cid, cl, _data, text)
VALUES (3, 1, 0, 'text/plain', NULL, 106, '<text_0>', 'text_0.txt', NULL, 'Foto do local combinado (MMS sintético)');
INSERT INTO part (_id, mid, seq, ct, chset, cl, text)
VALUES (4, 2, 0, 'text/plain', 106, 'text_0.txt', 'Mensagem em grupo (MMS sintético)');
INSERT INTO part (_id, mid, seq, ct, name, cl, _data)
VALUES (5, 3, 0, 'audio/amr', 'audio_sintetico.amr', 'audio_sintetico.amr', '/data/user_de/0/com.android.providers.telephony/app_parts/PART_1767240000000_audio_sintetico.amr');
INSERT INTO part (_id, mid, seq, ct, name, cl, _data)
VALUES (6, 3, 1, 'image/png', 'figura_sintetica.png', 'figura_sintetica.png', '/data/user_de/0/com.android.providers.telephony/app_parts/PART_1767240000001_figura_sintetica.png');
-- Parte órfã (mensagem 999 não existe).
INSERT INTO part (_id, mid, seq, ct, text) VALUES (7, 999, 0, 'text/plain', 'Parte órfã sintética');
