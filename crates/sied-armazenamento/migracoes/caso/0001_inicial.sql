-- Banco autocontido de um caso (caso.sied.db).
-- Tudo que o caso precisa fica aqui, para que a pasta do caso possa ser
-- arquivada, copiada ou transferida para outra estação.

-- Um único caso por banco.
CREATE TABLE caso (
    id                   TEXT PRIMARY KEY,   -- UUID v7
    procedimento         TEXT,
    numero_procedimento  TEXT,
    numero_bo            TEXT,
    investigado          TEXT,
    vitima               TEXT,
    delegado             TEXT,
    responsavel_analise  TEXT NOT NULL,
    unidade              TEXT NOT NULL,
    observacoes          TEXT,
    fuso_horario         TEXT NOT NULL,
    criado_em            TEXT NOT NULL,
    atualizado_em        TEXT NOT NULL
);
CREATE TRIGGER caso_unico BEFORE INSERT ON caso
WHEN (SELECT COUNT(*) FROM caso) > 0
BEGIN SELECT RAISE(ABORT, 'o banco do caso já contém um caso'); END;

CREATE TABLE dispositivo (
    id               TEXT PRIMARY KEY,       -- UUID v7 (único entre casos)
    caso_id          TEXT NOT NULL REFERENCES caso(id),
    apelido          TEXT NOT NULL,
    fabricante       TEXT,
    modelo           TEXT,
    nome_comercial   TEXT,
    sistema          TEXT NOT NULL CHECK (sistema IN ('android','ios','outro','desconhecido')),
    versao_sistema   TEXT,
    imei1            TEXT,
    imei2            TEXT,
    serial           TEXT,
    numero           TEXT,
    operadora        TEXT,
    capacidade_bytes INTEGER,
    observacoes      TEXT,
    criado_em        TEXT NOT NULL,
    atualizado_em    TEXT NOT NULL
);
-- Chaves de correlação futura entre casos (mesmo aparelho em casos diferentes).
CREATE INDEX idx_dispositivo_imei1  ON dispositivo(imei1);
CREATE INDEX idx_dispositivo_imei2  ON dispositivo(imei2);
CREATE INDEX idx_dispositivo_serial ON dispositivo(serial);

-- Trilha de auditoria encadeada: cada registro guarda o hash do anterior.
CREATE TABLE auditoria (
    seq            INTEGER PRIMARY KEY,
    ts             TEXT NOT NULL,
    usuario        TEXT NOT NULL,
    maquina        TEXT NOT NULL,
    acao           TEXT NOT NULL,
    detalhes       TEXT NOT NULL,            -- JSON
    hash_anterior  TEXT NOT NULL,
    hash           TEXT NOT NULL
);
CREATE TRIGGER auditoria_sem_update BEFORE UPDATE ON auditoria
BEGIN SELECT RAISE(ABORT, 'auditoria é somente inclusão'); END;
CREATE TRIGGER auditoria_sem_delete BEFORE DELETE ON auditoria
BEGIN SELECT RAISE(ABORT, 'auditoria é somente inclusão'); END;
