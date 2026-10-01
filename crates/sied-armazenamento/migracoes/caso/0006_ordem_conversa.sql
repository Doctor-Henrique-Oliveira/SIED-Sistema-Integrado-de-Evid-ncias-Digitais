-- Ordem das mensagens de uma conversa servida pelo índice.
--
-- O índice anterior, (app, conversa_id_externo, data_hora_ms), não começava pela
-- extração: a consulta filtrava a extração linha a linha e ordenava tudo antes de
-- paginar (página de uma conversa de 100 mil mensagens em 60–150 ms). Este começa pela
-- chave completa da conversa e termina na data; o `id` (rowid) vem no fim de todo
-- índice, então "data, id" sai na ordem do índice, sem ordenar.
--
-- Só índices: nenhum dado muda e nada precisa ser reconstruído.

CREATE INDEX idx_artefato_conversa_ordem
    ON artefato(extracao_id, app, conversa_id_externo, data_hora_ms);

DROP INDEX idx_artefato_conversa;
