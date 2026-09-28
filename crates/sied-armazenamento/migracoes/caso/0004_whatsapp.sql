-- Leitor WhatsApp: vínculo de anexos com o inventário e consultas por identificador.
--
-- Nada aqui altera dados já gravados: a coluna nova fica vazia nos artefatos
-- existentes (SMS não tem anexo) e os índices só aceleram consultas.

-- Como o anexo informado pelo parser (caminho gravado pelo app) foi ligado ao
-- inventário: caminho_exato, sufixo_caminho, nome_e_tamanho, ambiguo,
-- tamanho_diferente ou nao_encontrado. Vazio quando o artefato não tem anexo.
ALTER TABLE artefato ADD COLUMN anexo_vinculo TEXT;

-- Candidatos a anexo pelo nome do arquivo.
CREATE INDEX idx_arquivo_nome ON arquivo(extracao_id, nome);

-- Registro da conversa e contatos pelo identificador no app (título da conversa e
-- nomes dos participantes, resolvidos na consulta).
CREATE INDEX idx_artefato_id_externo ON artefato(extracao_id, app, tipo, id_externo);
