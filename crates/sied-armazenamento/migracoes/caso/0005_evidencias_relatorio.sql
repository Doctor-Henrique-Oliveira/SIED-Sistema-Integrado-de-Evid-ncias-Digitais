-- Evidências selecionadas e relatórios gerados.
--
-- Uma evidência aponta para registros e arquivos imutáveis (artefato, arquivo) e
-- guarda o hash deles no momento em que foi marcada. O que o analista escreve
-- (categoria, título, observação, descrição) e a ordem podem mudar; o alvo e o hash
-- não. Remover é lógico: a linha fica, com quem removeu, quando e por quê.

CREATE TABLE evidencia (
    id                    INTEGER PRIMARY KEY,
    tipo                  TEXT NOT NULL CHECK (tipo IN ('registro', 'trecho', 'arquivo')),
    artefato_id           INTEGER REFERENCES artefato(id),   -- registro, ou primeira mensagem do trecho
    trecho_fim_id         INTEGER REFERENCES artefato(id),   -- última mensagem do trecho
    arquivo_id            INTEGER REFERENCES arquivo(id),
    itens                 INTEGER NOT NULL,                  -- mensagens no trecho; 1 nos demais
    hash_no_momento       TEXT NOT NULL,                     -- hash_registro, SHA-256 do arquivo ou do trecho
    categoria             TEXT,
    titulo                TEXT,
    observacao            TEXT,
    descricao_contextual  TEXT,
    ordem                 INTEGER NOT NULL,
    criada_por            TEXT NOT NULL,
    criada_em             TEXT NOT NULL,
    atualizada_em         TEXT NOT NULL,
    removida_em           TEXT,
    removida_por          TEXT,
    motivo_remocao        TEXT,
    CHECK (
        (tipo = 'registro' AND artefato_id IS NOT NULL AND trecho_fim_id IS NULL AND arquivo_id IS NULL AND itens = 1)
     OR (tipo = 'trecho'   AND artefato_id IS NOT NULL AND trecho_fim_id IS NOT NULL AND arquivo_id IS NULL AND itens >= 1)
     OR (tipo = 'arquivo'  AND artefato_id IS NULL AND trecho_fim_id IS NULL AND arquivo_id IS NOT NULL AND itens = 1)
    )
);
CREATE INDEX idx_evidencia_ordem    ON evidencia(removida_em, ordem);
CREATE INDEX idx_evidencia_artefato ON evidencia(artefato_id);
CREATE INDEX idx_evidencia_arquivo  ON evidencia(arquivo_id);

CREATE TRIGGER evidencia_sem_delete BEFORE DELETE ON evidencia
BEGIN SELECT RAISE(ABORT, 'evidência não é apagada: a remoção é lógica'); END;
CREATE TRIGGER evidencia_alvo_fixo BEFORE UPDATE OF tipo, artefato_id, trecho_fim_id, arquivo_id, itens,
    hash_no_momento, criada_por, criada_em ON evidencia
BEGIN SELECT RAISE(ABORT, 'o alvo e o hash de uma evidência não mudam'); END;
CREATE TRIGGER evidencia_removida_fixa BEFORE UPDATE ON evidencia
WHEN OLD.removida_em IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'evidência removida não é alterada'); END;

-- Cada PDF gerado: o que entrou, quem gerou, o SHA-256 do arquivo e até onde a
-- trilha de auditoria ia naquele momento.
CREATE TABLE relatorio (
    id                  INTEGER PRIMARY KEY,
    modelo              TEXT NOT NULL,
    secoes              TEXT NOT NULL,       -- JSON com as seções escolhidas
    evidencias          TEXT NOT NULL,       -- JSON com id, número e hash de cada evidência incluída
    gerado_por          TEXT NOT NULL,
    maquina             TEXT NOT NULL,
    gerado_em           TEXT NOT NULL,
    caminho             TEXT NOT NULL,       -- relativo à pasta do caso
    sha256              TEXT NOT NULL,
    tamanho             INTEGER NOT NULL,
    sied_versao         TEXT NOT NULL,
    auditoria_ate_seq   INTEGER NOT NULL,
    auditoria_hash      TEXT NOT NULL
);
CREATE TRIGGER relatorio_sem_update BEFORE UPDATE ON relatorio
BEGIN SELECT RAISE(ABORT, 'relatório é somente inclusão'); END;
CREATE TRIGGER relatorio_sem_delete BEFORE DELETE ON relatorio
BEGIN SELECT RAISE(ABORT, 'relatório é somente inclusão'); END;
