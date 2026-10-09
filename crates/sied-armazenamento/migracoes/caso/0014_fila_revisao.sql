-- Índices de leitura para a fila de revisão humana. Não alteram snapshots,
-- inferências nem decisões append-only; apenas evitam varredura do caso inteiro.
CREATE INDEX idx_documento_caso_motor
    ON documento_derivado(caso_id, entidades_motor_versao, id);
