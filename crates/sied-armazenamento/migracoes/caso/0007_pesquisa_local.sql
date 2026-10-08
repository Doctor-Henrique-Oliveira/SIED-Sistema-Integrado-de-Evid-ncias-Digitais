-- AI-0: configurações e snapshots DERIVADOS, nunca fatos/evidências.
-- Histórico imutável; revisões recebem novas linhas e auditoria na mesma transação.
CREATE TABLE vocabulario_pesquisa (
    id INTEGER PRIMARY KEY,
    nome TEXT NOT NULL,
    versao INTEGER NOT NULL CHECK (versao >= 1),
    criado_em TEXT NOT NULL,
    autor TEXT NOT NULL,
    dados TEXT NOT NULL CHECK (json_valid(dados)),
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    UNIQUE(nome, versao)
);
CREATE TABLE consulta_descoberta (
    id INTEGER PRIMARY KEY,
    criada_em TEXT NOT NULL,
    dados TEXT NOT NULL CHECK (json_valid(dados)),
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    vocabulario_id INTEGER REFERENCES vocabulario_pesquisa(id)
);
CREATE TRIGGER vocabulario_pesquisa_sem_update BEFORE UPDATE ON vocabulario_pesquisa
BEGIN SELECT RAISE(ABORT, 'versão de vocabulário é somente inclusão'); END;
CREATE TRIGGER vocabulario_pesquisa_sem_delete BEFORE DELETE ON vocabulario_pesquisa
BEGIN SELECT RAISE(ABORT, 'versão de vocabulário é somente inclusão'); END;
CREATE TRIGGER consulta_descoberta_sem_update BEFORE UPDATE ON consulta_descoberta
BEGIN SELECT RAISE(ABORT, 'snapshot de consulta é somente inclusão'); END;
CREATE TRIGGER consulta_descoberta_sem_delete BEFORE DELETE ON consulta_descoberta
BEGIN SELECT RAISE(ABORT, 'snapshot de consulta é somente inclusão'); END;
