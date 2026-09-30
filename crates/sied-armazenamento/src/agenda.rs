//! Contatos (agenda do aparelho e contatos dos apps) e registro de chamadas, lidos dos
//! artefatos gravados pelos parsers.
//!
//! A ligação de uma chamada (ou de um identificador de conversa) a um contato da
//! agenda do Android é feita **na consulta**, só dentro da mesma extração e só quando
//! os dígitos são **idênticos**: o número como gravado ou o número normalizado pelo
//! próprio aparelho (`normalized_number` da chamada, `data4` do telefone na agenda). O
//! SIED não junta números parecidos (sem DDI, sem nono dígito...) nem grava o vínculo:
//! um parser novo ou uma reimportação muda o resultado sem deixar vínculo velho.

use rusqlite::{params, params_from_iter, types::Value as ValorSql, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sied_dominio::ExtracaoId;
use std::collections::HashMap;
use ts_rs::TS;

use crate::busca::digitos_telefone;
use crate::conexao::so_digitos;
use crate::extracoes::{uuid, Pagina};
use crate::{normalizar_busca, Resultado};

/// `app` dos registros da agenda do Android (parser `android_contatos`).
pub const APP_AGENDA: &str = sied_ingestao::parsers::android_contatos::APP;

/// Artefatos de um tipo em todas as extrações do caso (ou numa só). O `INDEXED BY` é
/// deliberado, como nas conversas: sem estatísticas o SQLite pode preferir o índice de
/// tempo e percorrer todos os registros do caso para achar os poucos deste tipo.
fn base(tipo: &str, extracao: Option<ExtracaoId>) -> (String, Vec<ValorSql>) {
    let mut v = vec![ValorSql::Text(tipo.into())];
    let onde = match extracao {
        Some(e) => {
            v.push(ValorSql::Text(e.to_string()));
            "a.extracao_id = ?2"
        }
        None => "a.extracao_id IN (SELECT id FROM extracao)",
    };
    (
        format!("FROM artefato a INDEXED BY idx_artefato_extracao WHERE {onde} AND a.tipo = ?1"),
        v,
    )
}

fn texto_de(m: &Map<String, Value>, chave: &str) -> Option<String> {
    m.get(chave)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

fn metadados(texto: &str) -> Map<String, Value> {
    serde_json::from_str(texto).unwrap_or_default()
}

// ------------------------------------------------------------------ vínculo

/// Contato da agenda ligado a um número pelo mesmo número.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ContatoVinculado {
    /// Registro (artefato) do contato.
    #[ts(type = "number")]
    pub id: i64,
    pub nome: Option<String>,
    /// "ativo", "apagado"... como o parser leu.
    pub estado_registro: String,
}

/// Número da agenda → contatos, por extração.
#[derive(Default)]
struct IndiceAgenda {
    por_extracao: HashMap<String, HashMap<String, Vec<ContatoVinculado>>>,
}

impl IndiceAgenda {
    fn carregar(&mut self, conn: &Connection, extracao: &str) -> Resultado<()> {
        if self.por_extracao.contains_key(extracao) {
            return Ok(());
        }
        let mut chaves: HashMap<String, Vec<ContatoVinculado>> = HashMap::new();
        let mut stmt = conn.prepare_cached(
            "SELECT a.id, a.conteudo, a.estado_registro, a.metadados FROM artefato a
             WHERE a.extracao_id = ?1 AND a.app = ?2 AND a.tipo = 'contato' ORDER BY a.id",
        )?;
        let mut rs = stmt.query(params![extracao, APP_AGENDA])?;
        while let Some(r) = rs.next()? {
            let contato = ContatoVinculado {
                id: r.get(0)?,
                nome: r.get(1)?,
                estado_registro: r.get(2)?,
            };
            let m = metadados(&r.get::<_, String>(3)?);
            for t in m
                .get("telefones")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(t) = t.as_object() else { continue };
                for campo in ["numero", "normalizado"] {
                    if let Some(k) = texto_de(t, campo).as_deref().and_then(so_digitos) {
                        let lista = chaves.entry(k).or_default();
                        if !lista.iter().any(|c| c.id == contato.id) {
                            lista.push(contato.clone());
                        }
                    }
                }
            }
        }
        self.por_extracao.insert(extracao.to_string(), chaves);
        Ok(())
    }

    /// Contatos com exatamente os mesmos dígitos de algum dos números informados.
    fn procurar<'a>(
        &self,
        extracao: &str,
        numeros: impl IntoIterator<Item = &'a str>,
    ) -> Vec<ContatoVinculado> {
        let mut achados: Vec<ContatoVinculado> = Vec::new();
        let Some(chaves) = self.por_extracao.get(extracao) else {
            return achados;
        };
        for n in numeros {
            let Some(k) = so_digitos(n) else { continue };
            for c in chaves.get(&k).into_iter().flatten() {
                if !achados.iter().any(|x| x.id == c.id) {
                    achados.push(c.clone());
                }
            }
        }
        achados.sort_by_key(|c| c.id);
        achados
    }
}

/// Agenda do Android para cada identificador de conversa que é telefone (SMS),
/// procurada nas extrações em que ele aparece. Identificadores de outros formatos
/// (jid do WhatsApp...) ficam sem vínculo: esses apps têm os próprios contatos.
pub(crate) fn agenda_de_identificadores(
    conn: &Connection,
    itens: &[(String, String)],
) -> Resultado<Vec<Vec<ContatoVinculado>>> {
    let mut indice = IndiceAgenda::default();
    let mut stmt = conn.prepare(
        "SELECT DISTINCT c.extracao_id FROM conversa_participante p
         JOIN conversa c ON c.id = p.conversa_id
         WHERE c.app = ?1 AND p.identificador = ?2 ORDER BY 1",
    )?;
    let mut saida = Vec::with_capacity(itens.len());
    for (app, identificador) in itens {
        let mut achados: Vec<ContatoVinculado> = Vec::new();
        if digitos_telefone(identificador).is_some() {
            let extracoes: Vec<String> = stmt
                .query_map(params![app, identificador], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            for e in extracoes {
                indice.carregar(conn, &e)?;
                for c in indice.procurar(&e, [identificador.as_str()]) {
                    if !achados.iter().any(|x| x.id == c.id) {
                        achados.push(c);
                    }
                }
            }
        }
        saida.push(achados);
    }
    Ok(saida)
}

// ------------------------------------------------------------------ contatos

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TelefoneContato {
    /// Como gravado na agenda (com ou sem máscara).
    pub numero: String,
    /// Tipo interpretado pelo parser ("celular", "trabalho", "personalizado"...).
    pub tipo: Option<String>,
    /// Rótulo livre escrito pelo usuário do aparelho.
    pub rotulo: Option<String>,
}

/// Registro de contato: agenda do aparelho ou contatos de um app (ex.: wa.db).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ContatoAgenda {
    #[ts(type = "number")]
    pub id: i64,
    pub extracao_id: ExtracaoId,
    pub app: Option<String>,
    /// Nome gravado no registro; `None` quando o registro não tem nome.
    pub nome: Option<String>,
    /// Conta que sincroniza o contato (ex.: conta Google), quando gravada.
    pub conta: Option<String>,
    pub tipo_conta: Option<String>,
    pub telefones: Vec<TelefoneContato>,
    pub emails: Vec<String>,
    pub empresa: Option<String>,
    /// Identificador do registro no app (id do contato bruto, jid do WhatsApp...).
    pub id_externo: Option<String>,
    pub estado_registro: String,
    pub favorito: bool,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct FiltroContatos {
    #[ts(optional)]
    pub extracao_id: Option<ExtracaoId>,
    /// Trecho do nome (sem acento e sem diferenciar maiúsculas) ou do telefone (pelos
    /// dígitos, com ou sem máscara).
    #[ts(optional)]
    pub texto: Option<String>,
    #[ts(optional)]
    pub app: Option<String>,
}

/// Quantos registros de um tipo cada app tem (para os filtros das listas).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TotalApp {
    pub app: String,
    #[ts(type = "number")]
    pub total: i64,
}

/// Grupos do WhatsApp também chegam como registro de contato (é assim que o app os
/// guarda), mas são conversas: ficam fora da lista de contatos.
const NAO_GRUPO: &str = " AND coalesce(a.subtipo, '') <> 'grupo'";

fn condicoes_contatos(f: &FiltroContatos) -> (String, Vec<ValorSql>) {
    let (mut sql, mut v) = base("contato", f.extracao_id);
    sql.push_str(NAO_GRUPO);
    if let Some(app) = f.app.as_deref().filter(|a| !a.is_empty()) {
        v.push(ValorSql::Text(app.into()));
        sql.push_str(&format!(" AND a.app = ?{}", v.len()));
    }
    if let Some(t) = f.texto.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        v.push(ValorSql::Text(normalizar_busca(t)));
        let mut ou = format!("instr(sied_normal(a.conteudo), ?{}) > 0", v.len());
        if let Some(d) = digitos_telefone(t) {
            v.push(ValorSql::Text(d));
            let n = v.len();
            ou.push_str(&format!(
                " OR EXISTS (SELECT 1 FROM json_each(a.metadados, '$.telefones') t
                     WHERE instr(sied_digitos(t.value ->> '$.numero'), ?{n}) > 0
                        OR instr(sied_digitos(t.value ->> '$.normalizado'), ?{n}) > 0)
                  OR instr(sied_digitos(a.metadados ->> '$.numero'), ?{n}) > 0
                  OR (a.app IS NOT '{APP_AGENDA}' AND instr(sied_digitos(a.id_externo), ?{n}) > 0)"
            ));
        }
        sql.push_str(&format!(" AND ({ou})"));
    }
    (sql, v)
}

fn contato(r: &rusqlite::Row) -> rusqlite::Result<ContatoAgenda> {
    let m = metadados(&r.get::<_, String>(5)?);
    let mut telefones: Vec<TelefoneContato> = m
        .get("telefones")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|t| {
            Some(TelefoneContato {
                numero: texto_de(t, "numero")?,
                tipo: texto_de(t, "tipo_nome"),
                rotulo: texto_de(t, "rotulo"),
            })
        })
        .collect();
    // Contatos de apps (ex.: wa.db) guardam um só número.
    if telefones.is_empty() {
        if let Some(n) = texto_de(&m, "numero") {
            telefones.push(TelefoneContato {
                numero: n,
                tipo: None,
                rotulo: None,
            });
        }
    }
    let emails = m
        .get("emails")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|e| texto_de(e, "endereco"))
        .collect();
    Ok(ContatoAgenda {
        id: r.get(0)?,
        extracao_id: uuid(r.get(1)?)?,
        app: r.get(2)?,
        nome: r.get(3)?,
        conta: r.get(4)?,
        tipo_conta: texto_de(&m, "tipo_conta"),
        telefones,
        emails,
        empresa: texto_de(&m, "empresa"),
        id_externo: r.get(6)?,
        estado_registro: r.get(7)?,
        favorito: m.get("favorito").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// Contatos do caso em ordem alfabética (sem nome por último).
pub fn contatos(
    conn: &Connection,
    f: &FiltroContatos,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<ContatoAgenda>> {
    let (cond, mut v) = condicoes_contatos(f);
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) {cond}"),
        params_from_iter(v.iter()),
        |r| r.get(0),
    )?;
    v.push(ValorSql::Integer(limite));
    v.push(ValorSql::Integer(deslocamento));
    let n = v.len();
    let mut stmt = conn.prepare(&format!(
        "SELECT a.id, a.extracao_id, a.app, a.conteudo, a.conta, a.metadados, a.id_externo,
                a.estado_registro
         {cond}
         ORDER BY a.conteudo IS NULL, sied_normal(a.conteudo), a.id
         LIMIT ?{} OFFSET ?{n}",
        n - 1
    ))?;
    let itens = stmt
        .query_map(params_from_iter(v.iter()), contato)?
        .collect::<Result<_, _>>()?;
    Ok(Pagina { total, itens })
}

/// Apps que têm registros de contato no caso.
pub fn apps_contatos(conn: &Connection) -> Resultado<Vec<TotalApp>> {
    let (cond, v) = base("contato", None);
    let mut stmt = conn.prepare(&format!(
        "SELECT coalesce(a.app, ''), COUNT(*) {cond}{NAO_GRUPO}
         GROUP BY 1 ORDER BY 2 DESC, 1"
    ))?;
    let r = stmt.query_map(params_from_iter(v.iter()), |r| {
        Ok(TotalApp {
            app: r.get(0)?,
            total: r.get(1)?,
        })
    })?;
    Ok(r.collect::<Result<_, _>>()?)
}

// ------------------------------------------------------------------ chamadas

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Chamada {
    #[ts(type = "number")]
    pub id: i64,
    pub extracao_id: ExtracaoId,
    /// "recebida", "efetuada", "perdida", "rejeitada", "bloqueada"... ou "desconhecido".
    pub subtipo: Option<String>,
    pub de_mim: Option<bool>,
    /// Número como gravado no registro; `None` quando não foi apresentado.
    pub numero: Option<String>,
    /// Código gravado no lugar do número pelas versões antigas do Android ("-1"
    /// desconhecido, "-2" privado, "-3" telefone público, "-4" pago).
    pub numero_especial: Option<String>,
    #[ts(type = "number | null")]
    pub data_hora_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub duracao_s: Option<i64>,
    /// Nome que o aparelho guardou junto da chamada (cópia da agenda na hora).
    pub nome_no_registro: Option<String>,
    /// "permitido", "restrito", "desconhecido", "telefone_publico".
    pub apresentacao: Option<String>,
    /// Chip/conta telefônica usada (como gravada).
    pub conta_telefonica: Option<String>,
    pub recursos: Vec<String>,
    pub estado_registro: String,
    /// Contatos da agenda da mesma extração com exatamente o mesmo número.
    pub agenda: Vec<ContatoVinculado>,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct FiltroChamadas {
    #[ts(optional)]
    pub extracao_id: Option<ExtracaoId>,
    /// Número (pelos dígitos) ou nome: o gravado junto da chamada ou o do contato da
    /// agenda com o mesmo número.
    #[ts(optional)]
    pub texto: Option<String>,
    #[ts(optional)]
    pub subtipo: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TotalSubtipo {
    pub subtipo: String,
    #[ts(type = "number")]
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResumoChamadas {
    #[ts(type = "number")]
    pub total: i64,
    pub por_subtipo: Vec<TotalSubtipo>,
    /// Soma das durações gravadas (segundos).
    #[ts(type = "number")]
    pub duracao_total_s: i64,
    /// Números distintos (pelos dígitos, exatamente como gravados).
    #[ts(type = "number")]
    pub numeros_distintos: i64,
}

/// Número da chamada: remetente (recebida) ou destinatário (efetuada).
const NUMERO: &str = "coalesce(a.remetente, a.destinatarios ->> '$[0]')";

fn condicoes_chamadas(conn: &Connection, f: &FiltroChamadas) -> Resultado<(String, Vec<ValorSql>)> {
    let (mut sql, mut v) = base("chamada", f.extracao_id);
    if let Some(s) = f.subtipo.as_deref().filter(|s| !s.is_empty()) {
        v.push(ValorSql::Text(s.into()));
        sql.push_str(&format!(" AND a.subtipo = ?{}", v.len()));
    }
    let Some(t) = f.texto.as_deref().map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok((sql, v));
    };
    if let Some(d) = digitos_telefone(t) {
        v.push(ValorSql::Text(d));
        let n = v.len();
        sql.push_str(&format!(
            " AND (instr(sied_digitos({NUMERO}), ?{n}) > 0
                OR instr(sied_digitos(a.metadados ->> '$.numero_normalizado'), ?{n}) > 0)"
        ));
        return Ok((sql, v));
    }
    // Nome: o gravado na chamada, ou o de um contato da agenda da mesma extração cujo
    // número tenha exatamente os mesmos dígitos (a mesma regra da coluna "Na agenda").
    let alvo = normalizar_busca(t);
    let (cond_agenda, mut va) = base("contato", f.extracao_id);
    va.push(ValorSql::Text(APP_AGENDA.into()));
    va.push(ValorSql::Text(alvo.clone()));
    let mut stmt = conn.prepare(&format!(
        "SELECT a.extracao_id, a.metadados {cond_agenda}
           AND a.app = ?{} AND instr(sied_normal(a.conteudo), ?{}) > 0",
        va.len() - 1,
        va.len()
    ))?;
    let mut pares: Vec<[String; 2]> = Vec::new();
    let mut rs = stmt.query(params_from_iter(va.iter()))?;
    while let Some(r) = rs.next()? {
        let extracao: String = r.get(0)?;
        let m = metadados(&r.get::<_, String>(1)?);
        for tel in m
            .get("telefones")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(tel) = tel.as_object() else { continue };
            for campo in ["numero", "normalizado"] {
                if let Some(k) = texto_de(tel, campo).as_deref().and_then(so_digitos) {
                    pares.push([extracao.clone(), k]);
                }
            }
        }
    }
    v.push(ValorSql::Text(alvo));
    let nt = v.len();
    v.push(ValorSql::Text(
        serde_json::to_string(&pares).expect("pares serializáveis"),
    ));
    let nj = v.len();
    sql.push_str(&format!(
        " AND (instr(sied_normal(a.metadados ->> '$.nome_no_registro'), ?{nt}) > 0
            OR EXISTS (SELECT 1 FROM json_each(?{nj}) k
                WHERE k.value ->> '$[0]' = a.extracao_id
                  AND k.value ->> '$[1]' IN (sied_digitos({NUMERO}),
                                            sied_digitos(a.metadados ->> '$.numero_normalizado'))))"
    ));
    Ok((sql, v))
}

/// Chamadas do caso, da mais recente para a mais antiga (sem data por último).
pub fn chamadas(
    conn: &Connection,
    f: &FiltroChamadas,
    deslocamento: i64,
    limite: i64,
) -> Resultado<Pagina<Chamada>> {
    let (cond, mut v) = condicoes_chamadas(conn, f)?;
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) {cond}"),
        params_from_iter(v.iter()),
        |r| r.get(0),
    )?;
    v.push(ValorSql::Integer(limite));
    v.push(ValorSql::Integer(deslocamento));
    let n = v.len();
    let mut stmt = conn.prepare(&format!(
        "SELECT a.id, a.extracao_id, a.subtipo, a.de_mim, {NUMERO}, a.data_hora_ms, a.metadados,
                a.estado_registro
         {cond}
         ORDER BY a.data_hora_ms IS NULL, a.data_hora_ms DESC, a.id DESC
         LIMIT ?{} OFFSET ?{n}",
        n - 1
    ))?;
    // Cada chamada com o número normalizado pelo aparelho (só para procurar na agenda).
    let lidas: Vec<(Chamada, Option<String>)> = stmt
        .query_map(params_from_iter(v.iter()), |r| {
            let m = metadados(&r.get::<_, String>(6)?);
            let chamada = Chamada {
                id: r.get(0)?,
                extracao_id: uuid(r.get(1)?)?,
                subtipo: r.get(2)?,
                de_mim: r.get(3)?,
                numero: r.get(4)?,
                numero_especial: texto_de(&m, "numero_especial"),
                data_hora_ms: r.get(5)?,
                duracao_s: m.get("duracao_s").and_then(Value::as_i64),
                nome_no_registro: texto_de(&m, "nome_no_registro"),
                apresentacao: texto_de(&m, "apresentacao"),
                conta_telefonica: texto_de(&m, "conta_telefonica"),
                recursos: m
                    .get("recursos")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect(),
                estado_registro: r.get(7)?,
                agenda: Vec::new(),
            };
            Ok((chamada, texto_de(&m, "numero_normalizado")))
        })?
        .collect::<Result<_, _>>()?;

    let mut indice = IndiceAgenda::default();
    let mut itens = Vec::with_capacity(lidas.len());
    for (mut c, normalizado) in lidas {
        let extracao = c.extracao_id.to_string();
        indice.carregar(conn, &extracao)?;
        c.agenda = indice.procurar(
            &extracao,
            c.numero
                .as_deref()
                .into_iter()
                .chain(normalizado.as_deref()),
        );
        itens.push(c);
    }
    Ok(Pagina { total, itens })
}

/// Totais do registro de chamadas do caso (para os indicadores e filtros da tela).
pub fn resumo_chamadas(conn: &Connection) -> Resultado<ResumoChamadas> {
    let (cond, v) = base("chamada", None);
    let mut stmt = conn.prepare(&format!(
        "SELECT coalesce(a.subtipo, 'desconhecido'), COUNT(*),
                coalesce(SUM(a.metadados ->> '$.duracao_s'), 0)
         {cond} GROUP BY 1 ORDER BY 2 DESC, 1"
    ))?;
    let mut por_subtipo = Vec::new();
    let mut duracao_total_s = 0;
    let mut rs = stmt.query(params_from_iter(v.iter()))?;
    while let Some(r) = rs.next()? {
        por_subtipo.push(TotalSubtipo {
            subtipo: r.get(0)?,
            total: r.get(1)?,
        });
        duracao_total_s += r.get::<_, i64>(2)?;
    }
    let numeros_distintos: i64 = conn.query_row(
        &format!("SELECT COUNT(DISTINCT sied_digitos({NUMERO})) {cond}"),
        params_from_iter(v.iter()),
        |r| r.get(0),
    )?;
    Ok(ResumoChamadas {
        total: por_subtipo.iter().map(|s| s.total).sum(),
        por_subtipo,
        duracao_total_s,
        numeros_distintos,
    })
}
