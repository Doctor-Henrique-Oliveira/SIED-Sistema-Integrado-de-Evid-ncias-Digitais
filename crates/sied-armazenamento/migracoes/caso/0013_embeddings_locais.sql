-- E03: índice vetorial local derivado para ranking híbrido.
-- O vetor é sempre reconstruível a partir do texto de origem; não altera evidências.
CREATE TABLE embedding_derivado (
    id              INTEGER PRIMARY KEY,
    caso_id         TEXT NOT NULL REFERENCES caso(id),
    origem_tipo     TEXT NOT NULL CHECK(origem_tipo IN ('artefato','documento','transcricao')),
    origem_id       INTEGER NOT NULL,
    extracao_id     TEXT NOT NULL REFERENCES extracao(id),
    arquivo_id      INTEGER REFERENCES arquivo(id),
    texto_hash      TEXT NOT NULL CHECK(length(texto_hash) = 64),
    dimensao        INTEGER NOT NULL CHECK(dimensao BETWEEN 64 AND 2048),
    vetor           BLOB NOT NULL,
    norma           REAL NOT NULL CHECK(norma >= 0),
    motor_id        TEXT NOT NULL,
    motor_versao    TEXT NOT NULL,
    executado_em    TEXT NOT NULL,
    hash_derivado   TEXT NOT NULL CHECK(length(hash_derivado) = 64),
    UNIQUE(origem_tipo, origem_id, texto_hash, motor_id, motor_versao)
);
CREATE INDEX idx_embedding_origem ON embedding_derivado(caso_id, origem_tipo, origem_id);
CREATE INDEX idx_embedding_extracao ON embedding_derivado(extracao_id);
CREATE TRIGGER embedding_sem_update BEFORE UPDATE ON embedding_derivado
BEGIN SELECT RAISE(ABORT, 'embedding derivado é somente inclusão'); END;
CREATE TRIGGER embedding_sem_delete BEFORE DELETE ON embedding_derivado
BEGIN SELECT RAISE(ABORT, 'embedding derivado é somente inclusão'); END;
