-- Banco global do SIED: catálogo de casos e configurações da estação.
-- NÃO guarda dados de casos: cada caso tem seu próprio banco autocontido.

CREATE TABLE configuracao (
    chave       TEXT PRIMARY KEY,
    valor       TEXT NOT NULL,
    alterado_em TEXT NOT NULL
);

-- Catálogo: onde estão os casos conhecidos por esta estação.
-- Espelha alguns campos do caso só para listar sem abrir cada banco;
-- a fonte da verdade é sempre o banco do caso.
CREATE TABLE caso_catalogo (
    caso_id              TEXT PRIMARY KEY,   -- UUID do caso (igual ao do banco do caso)
    pasta                TEXT NOT NULL,      -- pasta do caso nesta estação
    numero_procedimento  TEXT,
    numero_bo            TEXT,
    unidade              TEXT NOT NULL,
    responsavel_analise  TEXT NOT NULL,
    criado_em            TEXT NOT NULL,
    registrado_em        TEXT NOT NULL,
    ultimo_acesso_em     TEXT
);
CREATE INDEX idx_caso_catalogo_acesso ON caso_catalogo(ultimo_acesso_em DESC);
