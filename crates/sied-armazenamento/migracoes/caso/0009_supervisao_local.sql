-- Avaliação supervisionada EXPLÍCITA e local ao caso. Não treina modelos nem
-- transforma anotações em fatos/evidências. Autorizações/retenção são declaradas
-- pelo analista; nenhuma coleta ou exclusão automática decorre deste esquema.
CREATE TABLE dataset_supervisionado (
    id           INTEGER PRIMARY KEY,
    caso_id      TEXT NOT NULL REFERENCES caso(id),
    nome         TEXT NOT NULL,
    versao       INTEGER NOT NULL CHECK(versao >= 1),
    autorizacao  TEXT NOT NULL,
    retencao     TEXT NOT NULL,
    criado_em    TEXT NOT NULL,
    autor        TEXT NOT NULL,
    UNIQUE(caso_id, nome, versao)
);
CREATE TRIGGER dataset_supervisionado_sem_update BEFORE UPDATE ON dataset_supervisionado
BEGIN SELECT RAISE(ABORT, 'dataset autorizado é somente inclusão'); END;
CREATE TRIGGER dataset_supervisionado_sem_delete BEFORE DELETE ON dataset_supervisionado
BEGIN SELECT RAISE(ABORT, 'dataset autorizado é somente inclusão'); END;

CREATE TABLE amostra_supervisionada (
    id                       INTEGER PRIMARY KEY,
    dataset_id               INTEGER NOT NULL REFERENCES dataset_supervisionado(id),
    documento_id             INTEGER NOT NULL REFERENCES documento_derivado(id),
    versao                   INTEGER NOT NULL CHECK(versao >= 1),
    inicio_utf8              INTEGER NOT NULL CHECK(inicio_utf8 >= 0),
    fim_utf8                 INTEGER NOT NULL CHECK(fim_utf8 > inicio_utf8),
    trecho                   TEXT NOT NULL,
    sha256_texto_documento    TEXT NOT NULL CHECK(length(sha256_texto_documento) = 64),
    sha256_origem            TEXT NOT NULL CHECK(length(sha256_origem) = 64),
    sha256_amostra           TEXT NOT NULL CHECK(length(sha256_amostra) = 64),
    entidades_motor_versao   TEXT NOT NULL,
    tipos_previstos          TEXT NOT NULL CHECK(json_valid(tipos_previstos)),
    esperado                 INTEGER NOT NULL CHECK(esperado IN (0,1)),
    previsto                 INTEGER NOT NULL CHECK(previsto IN (0,1)),
    estado_rotulo            TEXT NOT NULL CHECK(estado_rotulo IN ('nao_revisado','confirmado','descartado','inconclusivo')),
    nota                     TEXT,
    criado_em                TEXT NOT NULL,
    revisor                  TEXT NOT NULL,
    UNIQUE(dataset_id, documento_id, inicio_utf8, fim_utf8, versao)
);
CREATE INDEX idx_amostra_dataset ON amostra_supervisionada(dataset_id, id);
CREATE INDEX idx_amostra_intervalo ON amostra_supervisionada(dataset_id, documento_id, inicio_utf8, fim_utf8, id);
CREATE TRIGGER amostra_supervisionada_sem_update BEFORE UPDATE ON amostra_supervisionada
BEGIN SELECT RAISE(ABORT, 'amostra rotulada é somente inclusão'); END;
CREATE TRIGGER amostra_supervisionada_sem_delete BEFORE DELETE ON amostra_supervisionada
BEGIN SELECT RAISE(ABORT, 'amostra rotulada é somente inclusão'); END;
