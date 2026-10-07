//! Conferência e reconstrução dos dados derivados do caso.
//!
//! Conversas, participantes e os índices de busca são calculados a partir de
//! `artefato`, `arquivo` e snapshots documentais e mantidos por gatilhos. Nada neles é fonte de
//! verdade: se divergirem (gatilho com defeito numa versão antiga, banco editado por
//! fora, índice FTS danificado), podem ser recalculados sem tocar nos registros de
//! origem, que são imutáveis.
//!
//! A conferência só lê. A reconstrução roda numa transação, com registro na
//! auditoria, e só é confirmada se a conferência depois dela não achar diferença.
//! Os índices de busca (FTS5 com texto próprio) são primeiro refeitos a partir do texto
//! guardado (`rebuild`, que conserta um índice danificado), depois esvaziados e
//! preenchidos de novo a partir da origem. As conversas são corrigidas no lugar, pela chave (extração, app, id no app): as que
//! continuam existindo mantêm o mesmo `id`.

use rusqlite::{Connection, Transaction};
use serde::Serialize;
use serde_json::json;
use ts_rs::TS;

use crate::auditoria;
use crate::{ErroArmazenamento, Resultado, Sessao};

/// Conversas como os gatilhos as agregam (mesmo cálculo do preenchimento da migração 3).
const ESPERADO_CONVERSA: &str = "
    SELECT extracao_id, coalesce(app, '') AS app, conversa_id_externo AS id_externo,
           COUNT(*) AS total_mensagens, SUM(coalesce(de_mim, 0)) AS enviadas,
           SUM(nome_arquivo IS NOT NULL OR anexo_caminho IS NOT NULL) AS com_anexo,
           MIN(data_hora_ms) AS primeira_ms, MAX(data_hora_ms) AS ultima_ms
    FROM artefato WHERE conversa_id_externo IS NOT NULL
    GROUP BY extracao_id, coalesce(app, ''), conversa_id_externo";

const GUARDADO_CONVERSA: &str = "
    SELECT extracao_id, app, id_externo, total_mensagens, enviadas, com_anexo,
           primeira_ms, ultima_ms
    FROM conversa";

/// Participantes pela chave da conversa (não pelo `id`, que é só da tabela derivada).
/// Remetente e destinatários contam uma vez por artefato.
const ESPERADO_PARTICIPANTE: &str = "
    SELECT x.extracao_id, x.app, x.conv, x.ident, COUNT(*) AS mensagens
    FROM (SELECT a.extracao_id, coalesce(a.app, '') AS app, a.conversa_id_externo AS conv,
                 a.id AS art, a.remetente AS ident
          FROM artefato a
          WHERE a.conversa_id_externo IS NOT NULL AND a.remetente IS NOT NULL AND a.remetente <> ''
          UNION
          SELECT a.extracao_id, coalesce(a.app, ''), a.conversa_id_externo, a.id, j.value
          FROM artefato a, json_each(a.destinatarios) j
          WHERE a.conversa_id_externo IS NOT NULL AND j.value <> '') x
    GROUP BY x.extracao_id, x.app, x.conv, x.ident";

const GUARDADO_PARTICIPANTE: &str = "
    SELECT c.extracao_id, c.app, c.id_externo, p.identificador, p.mensagens
    FROM conversa_participante p JOIN conversa c ON c.id = p.conversa_id";

/// Linha de busca de cada artefato (mesma expressão do gatilho `artefato_indexa_busca`).
const ESPERADO_BUSCA_ARTEFATO: &str = "
    SELECT id, conteudo, p || ' ' ||
           replace(replace(replace(replace(replace(replace(p,
               '+', ''), '-', ''), '(', ''), ')', ''), '.', ''), ' ', ''),
           nome_arquivo
    FROM (SELECT a.id, a.conteudo, a.nome_arquivo,
                 coalesce(a.remetente, '') || ' ' ||
                 coalesce((SELECT group_concat(value, ' ') FROM json_each(a.destinatarios)), '') AS p
          FROM artefato a)";

const GUARDADO_BUSCA_ARTEFATO: &str =
    "SELECT rowid, conteudo, pessoas, nome_arquivo FROM busca_artefato";

const ESPERADO_BUSCA_ARQUIVO: &str = "SELECT id, caminho FROM arquivo";
const GUARDADO_BUSCA_ARQUIVO: &str = "SELECT rowid, caminho FROM busca_arquivo";
// Os snapshots documentais são derivados históricos com hash próprio; só este
// índice de pesquisa pode ser reconstruído, nunca snapshots/candidatos/revisões.
const ESPERADO_BUSCA_DOCUMENTO: &str = "SELECT id, texto FROM documento_derivado";
const GUARDADO_BUSCA_DOCUMENTO: &str = "SELECT rowid, texto FROM busca_documento";
const ESPERADO_BUSCA_ENTIDADE: &str = "SELECT id, literal, tipo FROM entidade_documental";
const GUARDADO_BUSCA_ENTIDADE: &str = "SELECT rowid, literal, tipo FROM busca_entidade_documental";

struct Tabela {
    nome: &'static str,
    esperado: &'static str,
    guardado: &'static str,
    /// Tabela FTS5 cujo índice também é conferido (`integrity-check`).
    fts: bool,
}

const TABELAS: [Tabela; 6] = [
    Tabela {
        nome: "conversa",
        esperado: ESPERADO_CONVERSA,
        guardado: GUARDADO_CONVERSA,
        fts: false,
    },
    Tabela {
        nome: "conversa_participante",
        esperado: ESPERADO_PARTICIPANTE,
        guardado: GUARDADO_PARTICIPANTE,
        fts: false,
    },
    Tabela {
        nome: "busca_artefato",
        esperado: ESPERADO_BUSCA_ARTEFATO,
        guardado: GUARDADO_BUSCA_ARTEFATO,
        fts: true,
    },
    Tabela {
        nome: "busca_arquivo",
        esperado: ESPERADO_BUSCA_ARQUIVO,
        guardado: GUARDADO_BUSCA_ARQUIVO,
        fts: true,
    },
    Tabela {
        nome: "busca_documento",
        esperado: ESPERADO_BUSCA_DOCUMENTO,
        guardado: GUARDADO_BUSCA_DOCUMENTO,
        fts: true,
    },
    Tabela {
        nome: "busca_entidade_documental",
        esperado: ESPERADO_BUSCA_ENTIDADE,
        guardado: GUARDADO_BUSCA_ENTIDADE,
        fts: true,
    },
];

/// Situação de uma tabela derivada. Uma linha alterada conta como uma que falta (o
/// valor esperado) e uma que sobra (o valor guardado).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiferencaDerivado {
    pub tabela: String,
    /// Linhas esperadas, calculadas a partir dos registros de origem.
    #[ts(type = "number")]
    pub esperadas: i64,
    #[ts(type = "number")]
    pub faltando: i64,
    #[ts(type = "number")]
    pub sobrando: i64,
    /// Para os índices de busca: o índice confere com o texto guardado. Sempre
    /// `true` nas demais tabelas.
    pub indice_integro: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ConferenciaDerivados {
    pub tabelas: Vec<DiferencaDerivado>,
    pub consistente: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReconstrucaoDerivados {
    pub antes: ConferenciaDerivados,
    pub depois: ConferenciaDerivados,
}

fn contar(conn: &Connection, sql: &str) -> Resultado<i64> {
    Ok(conn.query_row(&format!("SELECT COUNT(*) FROM ({sql})"), [], |r| r.get(0))?)
}

fn indice_integro(conn: &Connection, tabela: &str) -> Resultado<bool> {
    match conn.execute(
        &format!("INSERT INTO {tabela}({tabela}) VALUES('integrity-check')"),
        [],
    ) {
        Ok(_) => Ok(true),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::DatabaseCorrupt =>
        {
            Ok(false)
        }
        Err(e) => Err(e.into()),
    }
}

/// Compara cada tabela derivada com o que os registros de origem produzem. Não grava.
pub fn conferir(conn: &Connection) -> Resultado<ConferenciaDerivados> {
    let mut tabelas = Vec::with_capacity(TABELAS.len());
    for t in &TABELAS {
        tabelas.push(DiferencaDerivado {
            tabela: t.nome.to_string(),
            esperadas: contar(conn, t.esperado)?,
            faltando: contar(conn, &format!("{} EXCEPT {}", t.esperado, t.guardado))?,
            sobrando: contar(conn, &format!("{} EXCEPT {}", t.guardado, t.esperado))?,
            indice_integro: !t.fts || indice_integro(conn, t.nome)?,
        });
    }
    let consistente = tabelas
        .iter()
        .all(|t| t.faltando == 0 && t.sobrando == 0 && t.indice_integro);
    Ok(ConferenciaDerivados {
        tabelas,
        consistente,
    })
}

/// Recalcula as tabelas derivadas a partir de `artefato` e `arquivo`, na transação
/// recebida, com registro na auditoria. Não toca nos registros de origem.
pub fn reconstruir(tx: &Transaction, sessao: &Sessao) -> Resultado<ReconstrucaoDerivados> {
    let antes = conferir(tx)?;
    tx.execute_batch(&format!(
        "DELETE FROM conversa_participante;
         DELETE FROM conversa
          WHERE (extracao_id, app, id_externo) NOT IN
                (SELECT extracao_id, coalesce(app, ''), conversa_id_externo
                 FROM artefato WHERE conversa_id_externo IS NOT NULL);
         INSERT INTO conversa (extracao_id, app, id_externo, total_mensagens, enviadas,
                               com_anexo, primeira_ms, ultima_ms)
         SELECT * FROM ({ESPERADO_CONVERSA}) WHERE true
         ON CONFLICT (extracao_id, app, id_externo) DO UPDATE SET
             total_mensagens = excluded.total_mensagens,
             enviadas        = excluded.enviadas,
             com_anexo       = excluded.com_anexo,
             primeira_ms     = excluded.primeira_ms,
             ultima_ms       = excluded.ultima_ms;
         INSERT INTO conversa_participante (conversa_id, identificador, mensagens)
         SELECT c.id, e.ident, e.mensagens
         FROM ({ESPERADO_PARTICIPANTE}) e
         JOIN conversa c ON c.extracao_id = e.extracao_id AND c.app = e.app
                        AND c.id_externo = e.conv;
         INSERT INTO busca_artefato(busca_artefato) VALUES('rebuild');
         DELETE FROM busca_artefato;
         INSERT INTO busca_artefato (rowid, conteudo, pessoas, nome_arquivo)
         {ESPERADO_BUSCA_ARTEFATO};
         INSERT INTO busca_arquivo(busca_arquivo) VALUES('rebuild');
         DELETE FROM busca_arquivo;
         INSERT INTO busca_arquivo (rowid, caminho) {ESPERADO_BUSCA_ARQUIVO};
         INSERT INTO busca_documento(busca_documento) VALUES('rebuild');
         DELETE FROM busca_documento;
         INSERT INTO busca_documento (rowid, texto) {ESPERADO_BUSCA_DOCUMENTO};
         INSERT INTO busca_entidade_documental(busca_entidade_documental) VALUES('rebuild');
         DELETE FROM busca_entidade_documental;
         INSERT INTO busca_entidade_documental (rowid, literal, tipo) {ESPERADO_BUSCA_ENTIDADE};"
    ))?;
    let depois = conferir(tx)?;
    if !depois.consistente {
        return Err(ErroArmazenamento::Inconsistente(format!(
            "dados derivados continuam divergentes após a reconstrução: {:?}",
            depois.tabelas
        )));
    }
    auditoria::registrar(
        tx,
        sessao,
        "derivados_reconstruidos",
        json!({
            "antes": antes.tabelas.iter().map(|t| json!({
                "tabela": t.tabela, "faltando": t.faltando, "sobrando": t.sobrando,
                "indice_integro": t.indice_integro,
            })).collect::<Vec<_>>(),
            "linhas": depois.tabelas.iter().map(|t| json!({
                "tabela": t.tabela, "linhas": t.esperadas,
            })).collect::<Vec<_>>(),
        }),
    )?;
    Ok(ReconstrucaoDerivados { antes, depois })
}
