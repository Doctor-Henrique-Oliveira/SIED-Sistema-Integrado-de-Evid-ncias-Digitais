-- AI-1: snapshots documentais e ocorrências candidatas, nunca artefatos/fatos.
-- Tudo permanece no banco de trabalho autocontido do caso, fora da extração.
-- A revisão humana é append-only e não substitui o candidato original.
CREATE TABLE documento_derivado (
    id                       INTEGER PRIMARY KEY,
    caso_id                  TEXT NOT NULL REFERENCES caso(id),
    extracao_id              TEXT NOT NULL REFERENCES extracao(id),
    arquivo_id               INTEGER NOT NULL REFERENCES arquivo(id),
    caminho                  TEXT NOT NULL,
    sha256_origem            TEXT NOT NULL CHECK(length(sha256_origem) = 64),
    sha256_texto             TEXT NOT NULL CHECK(length(sha256_texto) = 64),
    texto                    TEXT NOT NULL,
    pagina                   INTEGER NOT NULL CHECK(pagina BETWEEN 0 AND 512),
    total_paginas            INTEGER CHECK(total_paginas BETWEEN 1 AND 512),
    motor_id                 TEXT NOT NULL,
    motor_versao             TEXT NOT NULL,
    executado_em             TEXT NOT NULL,
    entidades_limitadas      INTEGER NOT NULL CHECK(entidades_limitadas IN (0,1)),
    entidades_motor_versao   TEXT NOT NULL,
    CHECK((pagina = 0 AND total_paginas IS NULL) OR
          (pagina > 0 AND total_paginas IS NOT NULL AND pagina <= total_paginas)),
    UNIQUE(arquivo_id, sha256_origem, motor_id, motor_versao, pagina,
           sha256_texto, entidades_motor_versao)
);
CREATE INDEX idx_documento_arquivo ON documento_derivado(arquivo_id, pagina, id);
CREATE INDEX idx_documento_extracao ON documento_derivado(extracao_id, id);
CREATE TRIGGER documento_sem_update BEFORE UPDATE ON documento_derivado
BEGIN SELECT RAISE(ABORT, 'snapshot documental é somente inclusão'); END;
CREATE TRIGGER documento_sem_delete BEFORE DELETE ON documento_derivado
BEGIN SELECT RAISE(ABORT, 'snapshot documental é somente inclusão'); END;

CREATE VIRTUAL TABLE busca_documento USING fts5(
    texto, tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER documento_indexa_busca AFTER INSERT ON documento_derivado
BEGIN
    INSERT INTO busca_documento(rowid, texto) VALUES(NEW.id, NEW.texto);
END;

CREATE TABLE entidade_documental (
    id               INTEGER PRIMARY KEY,
    documento_id     INTEGER NOT NULL REFERENCES documento_derivado(id),
    tipo             TEXT NOT NULL CHECK(tipo IN ('telefone','email','url','dominio','ip','imei')),
    inicio_utf8      INTEGER NOT NULL CHECK(inicio_utf8 >= 0),
    fim_utf8         INTEGER NOT NULL CHECK(fim_utf8 > inicio_utf8),
    literal          TEXT NOT NULL,
    regra_versao     TEXT NOT NULL,
    UNIQUE(documento_id, tipo, inicio_utf8, fim_utf8, regra_versao)
);
CREATE INDEX idx_entidade_documento ON entidade_documental(documento_id, inicio_utf8, id);
CREATE TRIGGER entidade_sem_update BEFORE UPDATE ON entidade_documental
BEGIN SELECT RAISE(ABORT, 'ocorrência candidata é somente inclusão'); END;
CREATE TRIGGER entidade_sem_delete BEFORE DELETE ON entidade_documental
BEGIN SELECT RAISE(ABORT, 'ocorrência candidata é somente inclusão'); END;

-- Índice reconstruível de ocorrências, nunca um cadastro de pessoas/fatos.
CREATE VIRTUAL TABLE busca_entidade_documental USING fts5(
    literal, tipo, tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER entidade_indexa_busca AFTER INSERT ON entidade_documental
BEGIN
    INSERT INTO busca_entidade_documental(rowid, literal, tipo)
    VALUES(NEW.id, NEW.literal, NEW.tipo);
END;

CREATE TABLE revisao_entidade_documental (
    id           INTEGER PRIMARY KEY,
    entidade_id  INTEGER NOT NULL REFERENCES entidade_documental(id),
    estado       TEXT NOT NULL CHECK(estado IN ('nao_revisado','confirmado','descartado','inconclusivo')),
    nota         TEXT,
    revista_em   TEXT NOT NULL,
    revisor      TEXT NOT NULL,
    maquina      TEXT NOT NULL
);
CREATE INDEX idx_revisao_entidade ON revisao_entidade_documental(entidade_id, id);
CREATE TRIGGER revisao_entidade_sem_update BEFORE UPDATE ON revisao_entidade_documental
BEGIN SELECT RAISE(ABORT, 'revisão humana é somente inclusão'); END;
CREATE TRIGGER revisao_entidade_sem_delete BEFORE DELETE ON revisao_entidade_documental
BEGIN SELECT RAISE(ABORT, 'revisão humana é somente inclusão'); END;
