-- Descritores técnicos derivados. Não classificam objetos nem alteram imagens.
CREATE TABLE visual_derivado (
 id INTEGER PRIMARY KEY,
 caso_id TEXT NOT NULL REFERENCES caso(id),
 extracao_id TEXT NOT NULL REFERENCES extracao(id),
 arquivo_id INTEGER NOT NULL REFERENCES arquivo(id),
 caminho TEXT NOT NULL, sha256_origem TEXT NOT NULL,
 dhash TEXT NOT NULL CHECK(length(dhash)=16),
 largura INTEGER NOT NULL CHECK(largura>0 AND largura<=4096),
 altura INTEGER NOT NULL CHECK(altura>0 AND altura<=4096),
 luminancia_media REAL NOT NULL,
 motor_id TEXT NOT NULL, motor_versao TEXT NOT NULL, executado_em TEXT NOT NULL,
 hash_derivado TEXT NOT NULL,
 UNIQUE(arquivo_id,sha256_origem,motor_id,motor_versao)
);
CREATE INDEX visual_hash ON visual_derivado(sha256_origem);
CREATE TRIGGER visual_sem_update BEFORE UPDATE ON visual_derivado
BEGIN SELECT RAISE(ABORT,'descritor visual é somente inclusão'); END;
CREATE TRIGGER visual_sem_delete BEFORE DELETE ON visual_derivado
BEGIN SELECT RAISE(ABORT,'descritor visual é somente inclusão'); END;
