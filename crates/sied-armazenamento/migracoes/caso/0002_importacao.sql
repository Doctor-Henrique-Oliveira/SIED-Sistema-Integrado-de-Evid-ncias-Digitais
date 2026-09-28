-- Importação de extrações: inventário, execuções de parser, artefatos e origem.
-- Tabelas de conversas, pessoas, evidências e busca entram quando forem usadas.

CREATE TABLE extracao (
    id                 TEXT PRIMARY KEY,     -- UUID v7
    dispositivo_id     TEXT NOT NULL REFERENCES dispositivo(id),
    importador         TEXT NOT NULL,        -- id: avilla, generico...
    importador_nome    TEXT NOT NULL,
    importador_versao  TEXT NOT NULL,
    ferramenta_versao  TEXT,                 -- versão da ferramenta de extração, se detectada
    tipo_extracao      TEXT,
    origem_caminho     TEXT NOT NULL,        -- caminho absoluto no momento da importação
    sied_versao        TEXT NOT NULL,
    usuario            TEXT NOT NULL,
    maquina            TEXT NOT NULL,
    iniciada_em        TEXT NOT NULL,
    concluida_em       TEXT,
    status             TEXT NOT NULL CHECK (status IN
                         ('em_andamento','concluida','cancelada','falhou','interrompida')),
    total_arquivos     INTEGER,
    total_bytes        INTEGER,
    arquivos_ilegiveis INTEGER,
    itens_ignorados    INTEGER,
    hash_manifesto     TEXT,                 -- SHA-256 de importacoes/<id>/manifesto.sha256
    total_artefatos    INTEGER,
    total_avisos       INTEGER,
    erro               TEXT,
    observacoes        TEXT                  -- JSON
);
CREATE INDEX idx_extracao_dispositivo ON extracao(dispositivo_id);

-- Inventário: um registro por arquivo da extração. Imutável depois de gravado.
CREATE TABLE arquivo (
    id            INTEGER PRIMARY KEY,
    extracao_id   TEXT NOT NULL REFERENCES extracao(id),
    caminho       TEXT NOT NULL,             -- relativo à raiz da extração, com '/'
    nome          TEXT NOT NULL,
    extensao      TEXT,
    tamanho       INTEGER NOT NULL,
    modificado_em TEXT,                      -- como registrado na extração
    criado_em     TEXT,
    sha256        TEXT,                      -- NULL somente se ilegível (ver erro)
    assinatura    TEXT,                      -- tipo reconhecido pelo conteúdo
    mime          TEXT,
    categoria     TEXT NOT NULL,
    erro          TEXT,
    UNIQUE (extracao_id, caminho)
);
CREATE INDEX idx_arquivo_categoria ON arquivo(extracao_id, categoria);
CREATE INDEX idx_arquivo_sha256    ON arquivo(sha256);
CREATE TRIGGER arquivo_sem_update BEFORE UPDATE ON arquivo
BEGIN SELECT RAISE(ABORT, 'inventário é somente inclusão'); END;
CREATE TRIGGER arquivo_sem_delete BEFORE DELETE ON arquivo
BEGIN SELECT RAISE(ABORT, 'inventário é somente inclusão'); END;

-- Cada vez que um parser roda.
CREATE TABLE execucao_parser (
    id                 INTEGER PRIMARY KEY,
    extracao_id        TEXT NOT NULL REFERENCES extracao(id),
    parser             TEXT NOT NULL,
    parser_nome        TEXT NOT NULL,
    parser_versao      TEXT NOT NULL,
    arquivo_id         INTEGER REFERENCES arquivo(id),   -- entrada principal
    copia_sha256       TEXT,                             -- hash da cópia lida (= original)
    iniciada_em        TEXT NOT NULL,
    concluida_em       TEXT,
    status             TEXT NOT NULL CHECK (status IN
                         ('em_andamento','ok','parcial','falhou','cancelada','interrompida')),
    artefatos          INTEGER,
    avisos             INTEGER,
    erro               TEXT
);
CREATE INDEX idx_execucao_extracao ON execucao_parser(extracao_id);

-- Problemas não fatais encontrados pelo parser (registro ilegível, coluna ausente...).
CREATE TABLE aviso_parser (
    id            INTEGER PRIMARY KEY,
    execucao_id   INTEGER NOT NULL REFERENCES execucao_parser(id),
    mensagem      TEXT NOT NULL,
    papel_arquivo TEXT,
    id_registro   TEXT
);
CREATE INDEX idx_aviso_execucao ON aviso_parser(execucao_id);

-- Modelo universal de artefatos.
CREATE TABLE artefato (
    id                    INTEGER PRIMARY KEY,
    extracao_id           TEXT NOT NULL REFERENCES extracao(id),
    dispositivo_id        TEXT NOT NULL REFERENCES dispositivo(id),
    execucao_id           INTEGER NOT NULL REFERENCES execucao_parser(id),
    tipo                  TEXT NOT NULL,
    subtipo               TEXT,
    app                   TEXT,
    conta                 TEXT,
    remetente             TEXT,
    destinatarios         TEXT NOT NULL,     -- JSON (lista)
    de_mim                INTEGER,
    data_hora_ms          INTEGER,           -- UTC, para ordenar e filtrar
    data_hora_original    TEXT,              -- valor exatamente como na fonte
    formato_data          TEXT,
    conteudo              TEXT,
    id_externo            TEXT,
    resposta_a_id_externo TEXT,
    conversa_id_externo   TEXT,
    anexo_caminho         TEXT,
    anexo_arquivo_id      INTEGER REFERENCES arquivo(id),
    mime                  TEXT,
    nome_arquivo          TEXT,
    tamanho               INTEGER,
    latitude              REAL,
    longitude             REAL,
    estado_registro       TEXT NOT NULL,
    confianca             TEXT NOT NULL,
    metadados             TEXT NOT NULL,     -- JSON
    hash_registro         TEXT NOT NULL      -- SHA-256 do artefato normalizado emitido pelo parser
);
CREATE INDEX idx_artefato_extracao ON artefato(extracao_id, tipo, data_hora_ms);
CREATE INDEX idx_artefato_tempo    ON artefato(data_hora_ms, id);
CREATE INDEX idx_artefato_conversa ON artefato(app, conversa_id_externo, data_hora_ms);
CREATE TRIGGER artefato_sem_update BEFORE UPDATE ON artefato
BEGIN SELECT RAISE(ABORT, 'artefato é somente inclusão'); END;
CREATE TRIGGER artefato_sem_delete BEFORE DELETE ON artefato
BEGIN SELECT RAISE(ABORT, 'artefato é somente inclusão'); END;

-- De onde cada artefato foi lido. Um artefato tem 1..N referências.
CREATE TABLE referencia_origem (
    id              INTEGER PRIMARY KEY,
    artefato_id     INTEGER NOT NULL REFERENCES artefato(id),
    principal       INTEGER NOT NULL,        -- 1 = referência principal
    papel_arquivo   TEXT NOT NULL,
    arquivo_id      INTEGER NOT NULL REFERENCES arquivo(id),
    arquivo_sha256  TEXT NOT NULL,           -- hash do arquivo lido (= inventário)
    tabela          TEXT,
    id_registro     TEXT,
    consulta        TEXT,
    deslocamento    INTEGER,
    valores_brutos  TEXT NOT NULL            -- JSON: a linha original, coluna a coluna
);
CREATE INDEX idx_ref_artefato ON referencia_origem(artefato_id);
CREATE INDEX idx_ref_arquivo  ON referencia_origem(arquivo_id);
CREATE TRIGGER referencia_sem_update BEFORE UPDATE ON referencia_origem
BEGIN SELECT RAISE(ABORT, 'referência de origem é somente inclusão'); END;
CREATE TRIGGER referencia_sem_delete BEFORE DELETE ON referencia_origem
BEGIN SELECT RAISE(ABORT, 'referência de origem é somente inclusão'); END;
