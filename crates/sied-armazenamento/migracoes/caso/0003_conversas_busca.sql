-- Conversas e busca global.
--
-- As duas coisas são DERIVADAS dos artefatos e do inventário: nada aqui é fonte de
-- verdade, e tudo pode ser reconstruído a partir de `artefato` e `arquivo`. Por isso
-- estas tabelas aceitam UPDATE (contadores), ao contrário das tabelas de origem.
-- São mantidas por gatilhos, na mesma transação da gravação do artefato/arquivo.

-- Uma conversa por (extração, app, id da conversa no app). Genérico: serve para SMS,
-- WhatsApp ou qualquer parser que preencha `conversa_id_externo`.
CREATE TABLE conversa (
    id               INTEGER PRIMARY KEY,
    extracao_id      TEXT NOT NULL REFERENCES extracao(id),
    app              TEXT NOT NULL,
    id_externo       TEXT NOT NULL,
    total_mensagens  INTEGER NOT NULL,
    enviadas         INTEGER NOT NULL,
    com_anexo        INTEGER NOT NULL,
    primeira_ms      INTEGER,
    ultima_ms        INTEGER,
    UNIQUE (extracao_id, app, id_externo)
);
CREATE INDEX idx_conversa_ultima ON conversa(extracao_id, ultima_ms);

-- Participantes vistos em cada conversa (remetentes e destinatários dos artefatos).
-- `identificador` é o valor exatamente como o parser emitiu (telefone, jid...).
CREATE TABLE conversa_participante (
    conversa_id    INTEGER NOT NULL REFERENCES conversa(id),
    identificador  TEXT NOT NULL,
    mensagens      INTEGER NOT NULL,
    PRIMARY KEY (conversa_id, identificador)
);

-- Busca nos artefatos. rowid = artefato.id.
--   conteudo      texto do registro
--   pessoas       remetente e destinatários, como emitidos e só com dígitos
--                 (para "11 98765-4321" encontrar "+5511987654321")
--   nome_arquivo  nome do anexo
CREATE VIRTUAL TABLE busca_artefato USING fts5(
    conteudo, pessoas, nome_arquivo,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Busca no inventário. rowid = arquivo.id. O tokenizador separa por '/', '.', '_'.
CREATE VIRTUAL TABLE busca_arquivo USING fts5(
    caminho,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Texto de pessoas de um artefato: identificadores originais + versão só com dígitos.
-- (Repetido nos gatilhos e no preenchimento abaixo: SQLite não tem funções de usuário
-- em SQL puro. Qualquer mudança aqui exige nova migração.)

CREATE TRIGGER artefato_indexa_busca AFTER INSERT ON artefato
BEGIN
    INSERT INTO busca_artefato (rowid, conteudo, pessoas, nome_arquivo)
    SELECT NEW.id, NEW.conteudo, p || ' ' ||
           replace(replace(replace(replace(replace(replace(p,
               '+', ''), '-', ''), '(', ''), ')', ''), '.', ''), ' ', ''),
           NEW.nome_arquivo
    FROM (SELECT coalesce(NEW.remetente, '') || ' ' ||
                 coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.destinatarios)), '')
                 AS p);
END;

CREATE TRIGGER artefato_agrega_conversa AFTER INSERT ON artefato
WHEN NEW.conversa_id_externo IS NOT NULL
BEGIN
    INSERT INTO conversa (extracao_id, app, id_externo, total_mensagens, enviadas,
                          com_anexo, primeira_ms, ultima_ms)
    VALUES (NEW.extracao_id, coalesce(NEW.app, ''), NEW.conversa_id_externo, 1,
            coalesce(NEW.de_mim, 0), NEW.nome_arquivo IS NOT NULL OR NEW.anexo_caminho IS NOT NULL,
            NEW.data_hora_ms, NEW.data_hora_ms)
    ON CONFLICT (extracao_id, app, id_externo) DO UPDATE SET
        total_mensagens = total_mensagens + 1,
        enviadas        = enviadas + excluded.enviadas,
        com_anexo       = com_anexo + excluded.com_anexo,
        primeira_ms     = min(coalesce(primeira_ms, excluded.primeira_ms), coalesce(excluded.primeira_ms, primeira_ms)),
        ultima_ms       = max(coalesce(ultima_ms, excluded.ultima_ms), coalesce(excluded.ultima_ms, ultima_ms));

    INSERT INTO conversa_participante (conversa_id, identificador, mensagens)
    SELECT c.id, ident, 1
    FROM conversa c,
         (SELECT NEW.remetente AS ident WHERE NEW.remetente IS NOT NULL
          UNION SELECT value FROM json_each(NEW.destinatarios))
    WHERE c.extracao_id = NEW.extracao_id AND c.app = coalesce(NEW.app, '')
      AND c.id_externo = NEW.conversa_id_externo AND ident <> ''
    ON CONFLICT (conversa_id, identificador) DO UPDATE SET mensagens = mensagens + 1;
END;

CREATE TRIGGER arquivo_indexa_busca AFTER INSERT ON arquivo
BEGIN
    INSERT INTO busca_arquivo (rowid, caminho) VALUES (NEW.id, NEW.caminho);
END;

-- Preenche a partir do que já foi importado (casos criados na versão 2).
INSERT INTO busca_arquivo (rowid, caminho) SELECT id, caminho FROM arquivo;

INSERT INTO busca_artefato (rowid, conteudo, pessoas, nome_arquivo)
SELECT id, conteudo, p || ' ' ||
       replace(replace(replace(replace(replace(replace(p,
           '+', ''), '-', ''), '(', ''), ')', ''), '.', ''), ' ', ''),
       nome_arquivo
FROM (SELECT a.id, a.conteudo, a.nome_arquivo,
             coalesce(a.remetente, '') || ' ' ||
             coalesce((SELECT group_concat(value, ' ') FROM json_each(a.destinatarios)), '') AS p
      FROM artefato a);

INSERT INTO conversa (extracao_id, app, id_externo, total_mensagens, enviadas, com_anexo,
                      primeira_ms, ultima_ms)
SELECT extracao_id, coalesce(app, ''), conversa_id_externo, COUNT(*),
       SUM(coalesce(de_mim, 0)),
       SUM(nome_arquivo IS NOT NULL OR anexo_caminho IS NOT NULL),
       MIN(data_hora_ms), MAX(data_hora_ms)
FROM artefato WHERE conversa_id_externo IS NOT NULL
GROUP BY extracao_id, coalesce(app, ''), conversa_id_externo;

INSERT INTO conversa_participante (conversa_id, identificador, mensagens)
SELECT c.id, x.ident, COUNT(*)
FROM (SELECT a.extracao_id, coalesce(a.app, '') AS app, a.conversa_id_externo AS conv,
             a.id AS art, a.remetente AS ident
      FROM artefato a WHERE a.remetente IS NOT NULL AND a.remetente <> ''
      UNION
      SELECT a.extracao_id, coalesce(a.app, ''), a.conversa_id_externo, a.id, j.value
      FROM artefato a, json_each(a.destinatarios) j WHERE j.value <> '') x
JOIN conversa c ON c.extracao_id = x.extracao_id AND c.app = x.app AND c.id_externo = x.conv
GROUP BY c.id, x.ident;
