//! Evidências selecionadas pelo analista.
//!
//! Uma evidência aponta para um registro, um trecho de conversa ou um arquivo do
//! inventário, e guarda o hash deles no momento em que foi marcada:
//!
//! - registro: `hash_registro` do artefato;
//! - arquivo: SHA-256 do inventário;
//! - trecho: SHA-256 da lista `id:hash_registro` de cada mensagem, uma por linha, na
//!   ordem da conversa (ver [`hash_do_trecho`]).
//!
//! O alvo e o hash nunca mudam (gatilhos no banco). Categoria, título, observação,
//! descrição e ordem podem ser editados; remover é lógico. Toda ação vai para a
//! auditoria na mesma transação.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::auditoria;
use crate::conversas;
use crate::extracoes::{ler_resumo, ArtefatoResumo, COLUNAS_RESUMO};
use crate::{ErroArmazenamento, Resultado, Sessao};

/// O que a evidência aponta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TipoEvidencia {
    /// Um registro (mensagem, contato, localização...).
    Registro,
    /// Trecho de conversa, da primeira à última mensagem escolhidas.
    Trecho,
    /// Arquivo do inventário (imagem, documento...).
    Arquivo,
}

impl TipoEvidencia {
    fn texto(self) -> &'static str {
        match self {
            TipoEvidencia::Registro => "registro",
            TipoEvidencia::Trecho => "trecho",
            TipoEvidencia::Arquivo => "arquivo",
        }
    }

    fn de(texto: &str) -> rusqlite::Result<Self> {
        match texto {
            "registro" => Ok(TipoEvidencia::Registro),
            "trecho" => Ok(TipoEvidencia::Trecho),
            "arquivo" => Ok(TipoEvidencia::Arquivo),
            outro => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("tipo de evidência {outro}"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

/// Pedido para marcar uma evidência.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct NovaEvidencia {
    #[ts(optional, type = "number")]
    pub artefato_id: Option<i64>,
    /// Com `artefato_id`, marca o trecho da conversa entre os dois registros.
    #[ts(optional, type = "number")]
    pub trecho_fim_id: Option<i64>,
    #[ts(optional, type = "number")]
    pub arquivo_id: Option<i64>,
    #[ts(optional)]
    pub categoria: Option<String>,
    #[ts(optional)]
    pub titulo: Option<String>,
    #[ts(optional)]
    pub observacao: Option<String>,
    #[ts(optional)]
    pub descricao_contextual: Option<String>,
}

/// Textos editáveis de uma evidência (substituem os anteriores; vazio apaga).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct TextosEvidencia {
    #[ts(optional)]
    pub categoria: Option<String>,
    #[ts(optional)]
    pub titulo: Option<String>,
    #[ts(optional)]
    pub observacao: Option<String>,
    #[ts(optional)]
    pub descricao_contextual: Option<String>,
}

/// Arquivo do inventário apontado por uma evidência.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArquivoEvidencia {
    #[ts(type = "number")]
    pub id: i64,
    pub extracao_id: String,
    pub caminho: String,
    pub sha256: Option<String>,
    #[ts(type = "number")]
    pub tamanho: i64,
    pub categoria: String,
    pub mime: Option<String>,
}

/// Evidência ativa, com o número que ela tem no relatório (1, 2, 3... na ordem).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Evidencia {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub numero: i64,
    pub tipo: TipoEvidencia,
    pub categoria: Option<String>,
    pub titulo: Option<String>,
    pub observacao: Option<String>,
    pub descricao_contextual: Option<String>,
    pub hash_no_momento: String,
    #[ts(type = "number")]
    pub itens: i64,
    pub criada_por: String,
    pub criada_em: String,
    pub atualizada_em: String,
    /// Registro (ou primeira mensagem do trecho).
    pub registro: Option<ArtefatoResumo>,
    /// Última mensagem do trecho.
    pub registro_fim: Option<ArtefatoResumo>,
    pub arquivo: Option<ArquivoEvidencia>,
    /// Conversa para "ver na origem", quando o registro pertence a uma.
    #[ts(type = "number | null")]
    pub conversa_id: Option<i64>,
}

/// Registros e arquivos já marcados, para a tela indicar "Evidência nº n".
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct Marcacoes {
    /// [id do artefato, número] de registros e das mensagens de trechos.
    #[ts(type = "Array<[number, number]>")]
    pub registros: Vec<(i64, i64)>,
    #[ts(type = "Array<[number, number]>")]
    pub arquivos: Vec<(i64, i64)>,
}

/// Hash de um trecho: SHA-256 das linhas `id:hash_registro`, na ordem da conversa.
pub fn hash_do_trecho<'a>(mensagens: impl IntoIterator<Item = (i64, &'a str)>) -> String {
    let mut h = Sha256::new();
    for (id, hash) in mensagens {
        h.update(format!("{id}:{hash}\n").as_bytes());
    }
    hex::encode(h.finalize())
}

fn limpo(t: Option<String>) -> Option<String> {
    t.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

const COLUNAS: &str = "e.id, e.tipo, e.categoria, e.titulo, e.observacao, e.descricao_contextual,
    e.hash_no_momento, e.itens, e.criada_por, e.criada_em, e.atualizada_em,
    e.artefato_id, e.trecho_fim_id, e.arquivo_id";

struct Linha {
    ev: Evidencia,
    artefato_id: Option<i64>,
    trecho_fim_id: Option<i64>,
    arquivo_id: Option<i64>,
}

fn ler_linha(r: &Row) -> rusqlite::Result<Linha> {
    let tipo: String = r.get(1)?;
    Ok(Linha {
        ev: Evidencia {
            id: r.get(0)?,
            numero: 0,
            tipo: TipoEvidencia::de(&tipo)?,
            categoria: r.get(2)?,
            titulo: r.get(3)?,
            observacao: r.get(4)?,
            descricao_contextual: r.get(5)?,
            hash_no_momento: r.get(6)?,
            itens: r.get(7)?,
            criada_por: r.get(8)?,
            criada_em: r.get(9)?,
            atualizada_em: r.get(10)?,
            registro: None,
            registro_fim: None,
            arquivo: None,
            conversa_id: None,
        },
        artefato_id: r.get(11)?,
        trecho_fim_id: r.get(12)?,
        arquivo_id: r.get(13)?,
    })
}

fn resumo(conn: &Connection, id: i64) -> Resultado<ArtefatoResumo> {
    Ok(conn.query_row(
        &format!("SELECT {COLUNAS_RESUMO} FROM artefato a WHERE a.id = ?1"),
        [id],
        ler_resumo,
    )?)
}

fn arquivo(conn: &Connection, id: i64) -> Resultado<ArquivoEvidencia> {
    conn.query_row(
        "SELECT id, extracao_id, caminho, sha256, tamanho, categoria, mime FROM arquivo WHERE id = ?1",
        [id],
        |r| {
            Ok(ArquivoEvidencia {
                id: r.get(0)?,
                extracao_id: r.get(1)?,
                caminho: r.get(2)?,
                sha256: r.get(3)?,
                tamanho: r.get(4)?,
                categoria: r.get(5)?,
                mime: r.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| ErroArmazenamento::NaoEncontrado("arquivo".into()))
}

fn completar(conn: &Connection, l: Linha, numero: i64) -> Resultado<Evidencia> {
    let mut ev = l.ev;
    ev.numero = numero;
    if let Some(id) = l.artefato_id {
        ev.registro = Some(resumo(conn, id)?);
        ev.conversa_id = conversas::conversa_do_artefato(conn, id)?;
    }
    if let Some(id) = l.trecho_fim_id {
        ev.registro_fim = Some(resumo(conn, id)?);
    }
    if let Some(id) = l.arquivo_id {
        ev.arquivo = Some(arquivo(conn, id)?);
    }
    Ok(ev)
}

/// Evidências ativas, na ordem do relatório.
pub fn listar(conn: &Connection) -> Resultado<Vec<Evidencia>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUNAS} FROM evidencia e WHERE e.removida_em IS NULL ORDER BY e.ordem, e.id"
    ))?;
    let linhas: Vec<Linha> = stmt.query_map([], ler_linha)?.collect::<Result<_, _>>()?;
    linhas
        .into_iter()
        .enumerate()
        .map(|(i, l)| completar(conn, l, i as i64 + 1))
        .collect()
}

/// Uma evidência ativa.
pub fn obter(conn: &Connection, id: i64) -> Resultado<Evidencia> {
    listar(conn)?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("evidência".into()))
}

/// Registros e arquivos marcados e o número de cada evidência.
pub fn marcacoes(conn: &Connection) -> Resultado<Marcacoes> {
    let mut m = Marcacoes::default();
    for ev in listar(conn)? {
        match ev.tipo {
            TipoEvidencia::Registro => {
                if let Some(r) = &ev.registro {
                    m.registros.push((r.id, ev.numero));
                }
            }
            TipoEvidencia::Trecho => {
                if let (Some(a), Some(b)) = (&ev.registro, &ev.registro_fim) {
                    for (msg, _) in conversas::trecho(conn, a.id, b.id)?.mensagens {
                        m.registros.push((msg.id, ev.numero));
                    }
                }
            }
            TipoEvidencia::Arquivo => {
                if let Some(f) = &ev.arquivo {
                    m.arquivos.push((f.id, ev.numero));
                }
            }
        }
    }
    Ok(m)
}

/// Marca uma evidência no fim da lista.
pub fn adicionar(tx: &Transaction, sessao: &Sessao, nova: NovaEvidencia) -> Resultado<Evidencia> {
    let (tipo, hash, itens, fim) = match (nova.artefato_id, nova.trecho_fim_id, nova.arquivo_id) {
        (Some(a), None, None) => {
            let hash: String = tx
                .query_row(
                    "SELECT hash_registro FROM artefato WHERE id = ?1",
                    [a],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| ErroArmazenamento::NaoEncontrado("registro".into()))?;
            (TipoEvidencia::Registro, hash, 1, None)
        }
        (Some(a), Some(b), None) => {
            // As conversas aparecem enquanto a importação grava em lotes: um trecho
            // marcado agora poderia ganhar mensagens depois e divergir no relatório
            // sem que nada tivesse sido adulterado.
            let status: Option<String> = tx
                .query_row(
                    "SELECT e.status FROM artefato a JOIN extracao e ON e.id = a.extracao_id
                     WHERE a.id = ?1",
                    [a],
                    |r| r.get(0),
                )
                .optional()?;
            if status.as_deref() == Some("em_andamento") {
                return Err(ErroArmazenamento::Recusado(
                    "a importação desta extração ainda está em andamento e a conversa pode \
                     ganhar mensagens; marque o trecho quando ela terminar"
                        .into(),
                ));
            }
            let t = conversas::trecho(tx, a, b)?;
            let hash = hash_do_trecho(t.mensagens.iter().map(|(m, h)| (m.id, h.as_str())));
            let primeira = t.mensagens.first().map(|(m, _)| m.id).unwrap_or(a);
            let ultima = t.mensagens.last().map(|(m, _)| m.id).unwrap_or(b);
            // Um trecho de uma mensagem só é um registro.
            if primeira == ultima {
                return adicionar(
                    tx,
                    sessao,
                    NovaEvidencia {
                        artefato_id: Some(primeira),
                        trecho_fim_id: None,
                        ..nova
                    },
                );
            }
            (
                TipoEvidencia::Trecho,
                hash,
                t.mensagens.len() as i64,
                Some((primeira, ultima)),
            )
        }
        (None, None, Some(f)) => {
            let a = arquivo(tx, f)?;
            let Some(sha) = a.sha256 else {
                return Err(ErroArmazenamento::Recusado(
                    "o arquivo não tem SHA-256 no inventário (ilegível na importação)".into(),
                ));
            };
            (TipoEvidencia::Arquivo, sha, 1, None)
        }
        _ => {
            return Err(ErroArmazenamento::Recusado(
                "informe um registro, um trecho (início e fim) ou um arquivo".into(),
            ))
        }
    };
    let (artefato_id, trecho_fim_id) = match fim {
        Some((a, b)) => (Some(a), Some(b)),
        None => (nova.artefato_id, None),
    };

    // A mesma coisa não entra duas vezes.
    let repetida: Option<i64> = tx
        .query_row(
            "SELECT id FROM evidencia WHERE removida_em IS NULL AND tipo = ?1
               AND artefato_id IS ?2 AND trecho_fim_id IS ?3 AND arquivo_id IS ?4",
            params![tipo.texto(), artefato_id, trecho_fim_id, nova.arquivo_id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = repetida {
        let numero = obter(tx, id)?.numero;
        return Err(ErroArmazenamento::Recusado(format!(
            "isto já está nas evidências (nº {numero})"
        )));
    }

    let agora = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let ordem: i64 = tx.query_row(
        "SELECT coalesce(max(ordem), 0) + 1 FROM evidencia WHERE removida_em IS NULL",
        [],
        |r| r.get(0),
    )?;
    tx.execute(
        "INSERT INTO evidencia(tipo, artefato_id, trecho_fim_id, arquivo_id, itens, hash_no_momento,
                               categoria, titulo, observacao, descricao_contextual, ordem,
                               criada_por, criada_em, atualizada_em)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
        params![
            tipo.texto(),
            artefato_id,
            trecho_fim_id,
            nova.arquivo_id,
            itens,
            hash,
            limpo(nova.categoria),
            limpo(nova.titulo),
            limpo(nova.observacao),
            limpo(nova.descricao_contextual),
            ordem,
            sessao.usuario,
            agora,
        ],
    )?;
    let id = tx.last_insert_rowid();
    let ev = obter(tx, id)?;
    auditoria::registrar(
        tx,
        sessao,
        "evidencia_adicionada",
        json!({
            "evidencia_id": id, "numero": ev.numero, "tipo": tipo, "artefato_id": artefato_id,
            "trecho_fim_id": trecho_fim_id, "arquivo_id": nova.arquivo_id, "itens": itens,
            "hash_no_momento": ev.hash_no_momento,
        }),
    )?;
    Ok(ev)
}

/// Troca categoria, título, observação e descrição.
pub fn alterar_textos(
    tx: &Transaction,
    sessao: &Sessao,
    id: i64,
    textos: TextosEvidencia,
) -> Resultado<Evidencia> {
    let antes = obter(tx, id)?;
    let agora = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let (categoria, titulo, observacao, descricao) = (
        limpo(textos.categoria),
        limpo(textos.titulo),
        limpo(textos.observacao),
        limpo(textos.descricao_contextual),
    );
    tx.execute(
        "UPDATE evidencia SET categoria = ?2, titulo = ?3, observacao = ?4, descricao_contextual = ?5,
                atualizada_em = ?6 WHERE id = ?1",
        params![id, categoria, titulo, observacao, descricao, agora],
    )?;
    auditoria::registrar(
        tx,
        sessao,
        "evidencia_alterada",
        json!({
            "evidencia_id": id, "numero": antes.numero,
            "antes": { "categoria": antes.categoria, "titulo": antes.titulo,
                       "observacao": antes.observacao, "descricao_contextual": antes.descricao_contextual },
            "depois": { "categoria": categoria, "titulo": titulo,
                        "observacao": observacao, "descricao_contextual": descricao },
        }),
    )?;
    obter(tx, id)
}

/// Leva a evidência para a posição `numero` (1 = primeira), empurrando as demais.
pub fn mover(tx: &Transaction, sessao: &Sessao, id: i64, numero: i64) -> Resultado<Vec<Evidencia>> {
    let lista = listar(tx)?;
    let atual = lista
        .iter()
        .position(|e| e.id == id)
        .ok_or_else(|| ErroArmazenamento::NaoEncontrado("evidência".into()))?;
    let destino = (numero.clamp(1, lista.len() as i64) - 1) as usize;
    let mut ids: Vec<i64> = lista.iter().map(|e| e.id).collect();
    let movida = ids.remove(atual);
    ids.insert(destino, movida);
    for (i, ev) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE evidencia SET ordem = ?2 WHERE id = ?1 AND ordem <> ?2",
            params![ev, i as i64 + 1],
        )?;
    }
    auditoria::registrar(
        tx,
        sessao,
        "evidencia_reordenada",
        json!({ "evidencia_id": id, "de": atual + 1, "para": destino + 1, "ordem": ids }),
    )?;
    listar(tx)
}

/// Tira a evidência da lista. O registro fica no banco com quem, quando e por quê.
pub fn remover(
    tx: &Transaction,
    sessao: &Sessao,
    id: i64,
    motivo: Option<String>,
) -> Resultado<()> {
    let antes = obter(tx, id)?;
    let motivo = limpo(motivo);
    let agora = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    tx.execute(
        "UPDATE evidencia SET removida_em = ?2, removida_por = ?3, motivo_remocao = ?4 WHERE id = ?1",
        params![id, agora, sessao.usuario, motivo],
    )?;
    auditoria::registrar(
        tx,
        sessao,
        "evidencia_removida",
        json!({ "evidencia_id": id, "numero": antes.numero, "motivo": motivo,
                "hash_no_momento": antes.hash_no_momento }),
    )?;
    Ok(())
}
