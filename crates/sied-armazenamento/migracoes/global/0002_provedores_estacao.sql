-- E11: catálogo de provedores configurados na estação.
-- Não armazena segredos, tokens ou conteúdo de casos. Provedores externos
-- permanecem desabilitados e só podem ser habilitados após configuração explícita.
CREATE TABLE provedor_estacao (
    id           TEXT PRIMARY KEY,
    nome         TEXT NOT NULL,
    tipo         TEXT NOT NULL,
    modo         TEXT NOT NULL CHECK (modo IN ('local', 'externo')),
    habilitado   INTEGER NOT NULL DEFAULT 0 CHECK (habilitado IN (0, 1)),
    caminho      TEXT,
    versao       TEXT,
    observacao   TEXT,
    criado_em    TEXT NOT NULL,
    atualizado_em TEXT NOT NULL
);
CREATE INDEX idx_provedor_estacao_tipo ON provedor_estacao(tipo);
