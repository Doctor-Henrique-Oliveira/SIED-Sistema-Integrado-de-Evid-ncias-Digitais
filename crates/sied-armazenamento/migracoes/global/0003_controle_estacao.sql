-- E11: permissões locais e trilha das ações fora do banco de caso.
-- A trilha é somente inclusão e não guarda segredos nem conteúdo de evidência.
CREATE TABLE permissao_estacao (
    usuario      TEXT NOT NULL,
    acao         TEXT NOT NULL,
    permitido    INTEGER NOT NULL DEFAULT 0 CHECK (permitido IN (0, 1)),
    criado_em    TEXT NOT NULL,
    atualizado_em TEXT NOT NULL,
    PRIMARY KEY (usuario, acao)
);

CREATE TABLE auditoria_estacao (
    seq          INTEGER PRIMARY KEY,
    ts           TEXT NOT NULL,
    usuario      TEXT NOT NULL,
    maquina      TEXT NOT NULL,
    acao         TEXT NOT NULL,
    detalhes     TEXT NOT NULL,
    hash_anterior TEXT NOT NULL,
    hash         TEXT NOT NULL UNIQUE
);

CREATE TRIGGER auditoria_estacao_sem_update BEFORE UPDATE ON auditoria_estacao
BEGIN SELECT RAISE(ABORT, 'auditoria da estação é somente inclusão'); END;

CREATE TRIGGER auditoria_estacao_sem_delete BEFORE DELETE ON auditoria_estacao
BEGIN SELECT RAISE(ABORT, 'auditoria da estação é somente inclusão'); END;

