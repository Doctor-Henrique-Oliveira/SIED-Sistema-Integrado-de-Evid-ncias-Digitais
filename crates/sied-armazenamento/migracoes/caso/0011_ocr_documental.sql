-- E01: geometria OCR derivada e dimensões da imagem renderizada.
-- As coordenadas nunca alteram a origem; identificam palavras no bitmap local
-- usado pelo motor, com confiança opcional quando o provedor a publicar.
ALTER TABLE documento_derivado ADD COLUMN largura_px INTEGER CHECK(largura_px > 0);
ALTER TABLE documento_derivado ADD COLUMN altura_px INTEGER CHECK(altura_px > 0);

CREATE TABLE regiao_documental (
    id              INTEGER PRIMARY KEY,
    documento_id    INTEGER NOT NULL REFERENCES documento_derivado(id),
    ordem           INTEGER NOT NULL CHECK(ordem >= 0),
    inicio_utf8     INTEGER NOT NULL CHECK(inicio_utf8 >= 0),
    fim_utf8        INTEGER NOT NULL CHECK(fim_utf8 > inicio_utf8),
    texto           TEXT NOT NULL,
    x               INTEGER NOT NULL CHECK(x >= 0),
    y               INTEGER NOT NULL CHECK(y >= 0),
    largura         INTEGER NOT NULL CHECK(largura >= 0),
    altura          INTEGER NOT NULL CHECK(altura >= 0),
    confianca       INTEGER CHECK(confianca BETWEEN 0 AND 100),
    UNIQUE(documento_id, ordem)
);
CREATE INDEX idx_regiao_documento ON regiao_documental(documento_id, ordem);
CREATE TRIGGER regiao_documental_sem_update BEFORE UPDATE ON regiao_documental
BEGIN SELECT RAISE(ABORT, 'região OCR é somente inclusão'); END;
CREATE TRIGGER regiao_documental_sem_delete BEFORE DELETE ON regiao_documental
BEGIN SELECT RAISE(ABORT, 'região OCR é somente inclusão'); END;
