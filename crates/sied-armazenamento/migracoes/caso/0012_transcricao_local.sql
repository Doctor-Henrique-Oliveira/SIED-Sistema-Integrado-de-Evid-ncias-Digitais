-- E02: transcrição local derivada, sempre vinculada ao arquivo e ao hash do inventário.
-- O texto não substitui o áudio/vídeo original e segmentos são somente inclusão.
CREATE TABLE transcricao_local (
    id              INTEGER PRIMARY KEY,
    caso_id         TEXT NOT NULL REFERENCES caso(id),
    extracao_id     TEXT NOT NULL REFERENCES extracao(id),
    arquivo_id      INTEGER NOT NULL REFERENCES arquivo(id),
    caminho         TEXT NOT NULL,
    sha256_origem   TEXT NOT NULL,
    estado          TEXT NOT NULL CHECK(estado IN ('processando','pronta','sem_suporte','cancelada','erro')),
    idioma          TEXT,
    motor_id        TEXT NOT NULL,
    motor_versao    TEXT NOT NULL,
    iniciado_em     TEXT NOT NULL,
    concluido_em    TEXT,
    erro            TEXT,
    hash_derivado   TEXT NOT NULL,
    UNIQUE(arquivo_id, sha256_origem, motor_id, motor_versao)
);
CREATE INDEX idx_transcricao_arquivo ON transcricao_local(arquivo_id, iniciado_em DESC);
CREATE TABLE segmento_transcricao (
    id              INTEGER PRIMARY KEY,
    transcricao_id  INTEGER NOT NULL REFERENCES transcricao_local(id),
    ordem           INTEGER NOT NULL CHECK(ordem >= 0),
    inicio_ms       INTEGER NOT NULL CHECK(inicio_ms >= 0),
    fim_ms          INTEGER NOT NULL CHECK(fim_ms > inicio_ms),
    texto           TEXT NOT NULL CHECK(length(texto) BETWEEN 1 AND 4000),
    confianca       REAL CHECK(confianca IS NULL OR (confianca >= 0 AND confianca <= 1)),
    falante         TEXT,
    hash_segmento   TEXT NOT NULL,
    UNIQUE(transcricao_id, ordem)
);
CREATE INDEX idx_segmento_transcricao_tempo ON segmento_transcricao(transcricao_id, inicio_ms, fim_ms);
CREATE VIRTUAL TABLE busca_transcricao USING fts5(texto, content='segmento_transcricao', content_rowid='id', tokenize='unicode61 remove_diacritics 2');
CREATE TRIGGER segmento_transcricao_ai AFTER INSERT ON segmento_transcricao BEGIN
    INSERT INTO busca_transcricao(rowid, texto) VALUES (new.id, new.texto);
END;
CREATE TRIGGER transcricao_local_sem_update BEFORE UPDATE ON transcricao_local
BEGIN SELECT RAISE(ABORT, 'transcrição local é somente inclusão'); END;
CREATE TRIGGER segmento_transcricao_sem_update BEFORE UPDATE ON segmento_transcricao
BEGIN SELECT RAISE(ABORT, 'segmento de transcrição é somente inclusão'); END;
CREATE TRIGGER segmento_transcricao_sem_delete BEFORE DELETE ON segmento_transcricao
BEGIN SELECT RAISE(ABORT, 'segmento de transcrição é somente inclusão'); END;
