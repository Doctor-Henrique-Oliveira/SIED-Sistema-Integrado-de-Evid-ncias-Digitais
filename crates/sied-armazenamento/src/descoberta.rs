//! AI-0 local: expansão explícita, trechos literais e indicadores **candidatos**.
//!
//! Só consulta os campos existentes dos artefatos e os caminhos do inventário.
//! Não lê arquivos, documentos ou rede, não altera índices e não interpreta crime.
//! Indicadores fazem varredura SQL, não um índice: custo linear no escopo filtrado,
//! memória por registro/página e contagens de artefatos (não de valores distintos).

use rusqlite::{functions::FunctionFlags, params_from_iter, types::Value as ValorSql, Connection};
use serde::{Deserialize, Serialize};
use sied_dominio::{ErroCampo, ErrosValidacao};
use std::net::IpAddr;
use std::sync::Arc;
use ts_rs::TS;

use crate::busca::{
    colunas_achado, consulta_fts, digitos_telefone, ler_achado, AchadoArquivo, AchadoArtefato,
    FiltroBusca,
};
use crate::controle_descoberta::GuardaDescoberta;
use crate::extracoes::{uuid, Pagina, N_RESUMO};
use crate::{ControleDescoberta, ErroArmazenamento, Resultado};

const MAX_TERMOS: usize = 32;
const MAX_CARACTERES: usize = 200;
const MAX_PAGINA: i64 = 200;
/// Limite de candidatos exibidos por artefato, não da varredura/contagem.
const MAX_INDICADORES: usize = 64;
const CAMPOS: &[&str] = &["conteudo", "remetente", "destinatarios", "nome_arquivo"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ModoDescoberta {
    Expansao,
    Literal,
    Indicadores,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TipoIndicador {
    Ip,
    Email,
    Url,
    Dominio,
    Hash,
    Telefone,
    Coordenada,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct PedidoDescoberta {
    pub filtro: FiltroBusca,
    pub modo: ModoDescoberta,
    #[serde(default)]
    pub termos: Vec<String>,
    #[ts(optional)]
    pub indicador: Option<TipoIndicador>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct IndicadorDescoberta {
    pub tipo: TipoIndicador,
    pub valor: String,
    pub campo: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AchadoDescoberta {
    pub achado: AchadoArtefato,
    pub motivos: Vec<String>,
    pub indicadores: Vec<IndicadorDescoberta>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ResultadoDescoberta {
    pub modo: ModoDescoberta,
    pub termos_efetivos: Vec<String>,
    pub artefatos: Pagina<AchadoDescoberta>,
    pub arquivos: Pagina<AchadoArquivo>,
    pub algoritmo: String,
    pub executada_em: String,
}

fn invalido(campo: &str, mensagem: &str) -> ErroArmazenamento {
    ErrosValidacao {
        erros: vec![ErroCampo {
            campo: campo.into(),
            mensagem: mensagem.into(),
        }],
    }
    .into()
}

fn termos(p: &PedidoDescoberta, deslocamento: i64, limite: i64) -> Resultado<Vec<String>> {
    if deslocamento < 0 || !(1..=MAX_PAGINA).contains(&limite) {
        return Err(invalido(
            "paginacao",
            "deslocamento não negativo e limite entre 1 e 200",
        ));
    }
    if p.filtro
        .de_ms
        .zip(p.filtro.ate_ms)
        .is_some_and(|(de, ate)| de > ate)
    {
        return Err(invalido("periodo", "a data inicial deve anteceder a final"));
    }
    for (nome, valor) in [("app", &p.filtro.app), ("tipo", &p.filtro.tipo)] {
        if valor
            .as_ref()
            .is_some_and(|v| v.chars().count() > MAX_CARACTERES || v.contains('\0'))
        {
            return Err(invalido(
                nome,
                "filtro inválido ou maior que 200 caracteres",
            ));
        }
    }
    if p.modo != ModoDescoberta::Expansao && !p.termos.is_empty() {
        return Err(invalido(
            "termos",
            "termos adicionais somente no modo expansão",
        ));
    }
    if p.modo != ModoDescoberta::Indicadores && p.indicador.is_some() {
        return Err(invalido(
            "indicador",
            "tipo de indicador somente no modo indicadores",
        ));
    }
    if p.termos.len() > MAX_TERMOS {
        return Err(invalido("termos", "máximo de 32 termos adicionais"));
    }
    let mut lista = Vec::new();
    for termo in std::iter::once(&p.filtro.texto).chain(p.termos.iter()) {
        if termo.chars().count() > MAX_CARACTERES || termo.contains('\0') {
            return Err(invalido(
                "texto",
                "cada termo admite até 200 caracteres, sem NUL",
            ));
        }
        let t = termo.trim();
        if !t.is_empty() && !lista.iter().any(|x| x == t) {
            lista.push(t.to_string());
        }
        if lista.len() > MAX_TERMOS {
            return Err(invalido("termos", "máximo de 32 termos efetivos"));
        }
    }
    // Não percorrer nem receber listas arbitrariamente grandes, mesmo duplicadas.
    if lista.is_empty() && p.modo != ModoDescoberta::Indicadores {
        return Err(invalido("texto", "informe um trecho ou termo de pesquisa"));
    }
    Ok(lista)
}

fn parametro(valores: &mut Vec<ValorSql>, v: ValorSql) -> String {
    valores.push(v);
    format!("?{}", valores.len())
}

fn filtros(p: &PedidoDescoberta, valores: &mut Vec<ValorSql>) -> String {
    let f = &p.filtro;
    let mut partes = Vec::new();
    if let Some(e) = f.extracao_id {
        partes.push(format!(
            "a.extracao_id = {}",
            parametro(valores, ValorSql::Text(e.to_string()))
        ));
    }
    for (campo, valor) in [("app", &f.app), ("tipo", &f.tipo)] {
        if let Some(v) = valor.as_ref().filter(|v| !v.is_empty()) {
            partes.push(format!(
                "a.{campo} = {}",
                parametro(valores, ValorSql::Text(v.clone()))
            ));
        }
    }
    for (op, valor) in [(">=", f.de_ms), ("<=", f.ate_ms)] {
        if let Some(v) = valor {
            partes.push(format!(
                "a.data_hora_ms {op} {}",
                parametro(valores, ValorSql::Integer(v))
            ));
        }
    }
    if partes.is_empty() {
        "1".into()
    } else {
        partes.join(" AND ")
    }
}

fn literal(param: &str) -> String {
    // Campos separados: uma frase não pode casar atravessando campos diferentes.
    CAMPOS.iter().map(|c| if *c == "destinatarios" { format!("EXISTS (SELECT 1 FROM json_each(a.destinatarios) d WHERE instr(d.value, {param}) > 0)") } else { format!("instr(coalesce(a.{c}, ''), {param}) > 0") }).collect::<Vec<_>>().join(" OR ")
}

fn possui_emoji(t: &str) -> bool {
    // Símbolos Unicode fora das faixas mais comuns também são significativos
    // (©, ®, ™, setas etc.). Preferir um ramo literal explícito a descartá-los.
    t.chars().any(|c| {
        (!c.is_ascii() && !c.is_alphanumeric() && !c.is_whitespace()
            && !unicode_normalization::char::is_combining_mark(c))
            || matches!(c as u32, 0x200d | 0x20e3 | 0xfe0e | 0xfe0f | 0x2600..=0x27bf | 0x1f000..=0x1faff | 0xe0100..=0xe01ef)
    })
}

struct Ramo {
    cond: String,
    motivo: String,
    campo_arquivo: Option<String>,
}

fn ramo(t: &str, modo: ModoDescoberta, valores: &mut Vec<ValorSql>) -> Ramo {
    if modo == ModoDescoberta::Literal || possui_emoji(t) || consulta_fts(t).is_none() {
        let v = parametro(valores, ValorSql::Text(t.into()));
        return Ramo { cond: format!("({})", literal(&v)), motivo: format!("Trecho literal exato «{t}» em conteúdo/remetente/destinatários/nome do anexo (caixa e sequência Unicode preservadas)."), campo_arquivo: Some(format!("instr(f.caminho, {v}) > 0")) };
    }
    if let Some(d) = digitos_telefone(t) {
        let v = parametro(valores, ValorSql::Text(d));
        return Ramo { cond: format!("(instr(coalesce(sied_digitos(a.remetente), ''), {v}) > 0 OR EXISTS (SELECT 1 FROM json_each(a.destinatarios) d WHERE instr(coalesce(sied_digitos(d.value), ''), {v}) > 0))"), motivo: format!("Dígitos do telefone «{t}» em remetente/destinatários; candidato, não identidade."), campo_arquivo: None };
    }
    let v = parametro(valores, ValorSql::Text(consulta_fts(t).unwrap_or_default()));
    Ramo { cond: format!("a.id IN (SELECT rowid FROM busca_artefato WHERE busca_artefato MATCH {v})"), motivo: format!("FTS5 por prefixo «{t}» nos campos indexados de conteúdo/remetente/destinatários/nome do anexo (inclui identificadores de contatos)."), campo_arquivo: Some(format!("f.id IN (SELECT rowid FROM busca_arquivo WHERE busca_arquivo MATCH {v})")) }
}

fn vazio<T: TS>() -> Pagina<T> {
    Pagina {
        total: 0,
        itens: Vec::new(),
    }
}

/// Busca somente leitura, paginada e determinística. Não abre conteúdo do inventário.
/// Indicadores varrem campos estruturados/textuais; scores, identidade e significado
/// não são inferidos. Até 64 candidatos por artefato são exibidos, sem truncar totais.
pub fn buscar(
    conn: &Connection,
    pedido: &PedidoDescoberta,
    deslocamento: i64,
    limite: i64,
) -> Resultado<ResultadoDescoberta> {
    buscar_com_controle(
        conn,
        pedido,
        deslocamento,
        limite,
        &ControleDescoberta::novo(),
    )
}

/// O controle pertence à requisição, não à conexão. O chamador mantém a exclusão
/// do serviço durante a busca e decide o gate `concluir` antes de persistir.
/// Nenhuma página parcial é retornada quando o controle aborta a operação.
pub fn buscar_com_controle(
    conn: &Connection,
    pedido: &PedidoDescoberta,
    deslocamento: i64,
    limite: i64,
    controle: &Arc<ControleDescoberta>,
) -> Resultado<ResultadoDescoberta> {
    controle.verificar()?;
    let mut guarda = GuardaDescoberta::instalar(conn, controle);
    let resultado = buscar_interno(conn, pedido, deslocamento, limite, controle, &mut guarda);
    // A limpeza é feita também no erro; Drop cobre unwind e tenta novamente se
    // a remoção de função falhar. Não substituir o erro original pelo de limpeza.
    let limpeza = guarda.limpar();
    match resultado {
        Ok(resultado) => {
            limpeza?;
            controle.verificar()?;
            Ok(resultado)
        }
        Err(erro) => Err(controle.traduzir_sqlite(erro)),
    }
}

fn buscar_interno(
    conn: &Connection,
    pedido: &PedidoDescoberta,
    deslocamento: i64,
    limite: i64,
    controle: &Arc<ControleDescoberta>,
    guarda: &mut GuardaDescoberta<'_>,
) -> Resultado<ResultadoDescoberta> {
    let efetivos = termos(pedido, deslocamento, limite)?;
    controle.verificar()?;
    let mut valores = Vec::new();
    let filtro = filtros(pedido, &mut valores);
    let mut ramos = Vec::new();
    if pedido.modo == ModoDescoberta::Indicadores {
        registrar_indicador(conn, controle)?;
        guarda.indicador_registrado();
        let tipo = parametro(&mut valores, ValorSql::Integer(codigo(pedido.indicador)));
        let cond = CAMPOS.iter().map(|c| if *c == "destinatarios" {
            format!("EXISTS (SELECT 1 FROM json_each(a.destinatarios) d WHERE sied_ai0_indicador({tipo}, d.value))")
        } else { format!("sied_ai0_indicador({tipo}, a.{c})") }).chain(std::iter::once(format!("({tipo} IN (0, 7) AND a.latitude BETWEEN -90 AND 90 AND a.longitude BETWEEN -180 AND 180)"))).collect::<Vec<_>>().join(" OR ");
        let mut cond = format!("({cond})");
        if let Some(t) = efetivos.first() {
            let v = parametro(&mut valores, ValorSql::Text(t.clone()));
            cond.push_str(&format!(" AND ({})", literal(&v)));
        }
        ramos.push(Ramo { cond, motivo: "Indicador sintático local nos campos existentes; candidato para revisão humana, sem consulta externa. Coordenadas estruturadas só em intervalos plausíveis. Não lê arquivos/OCR/documentos. Até 64 candidatos por artefato; total conta artefatos.".into(), campo_arquivo: None });
    } else {
        for t in &efetivos {
            controle.verificar()?;
            ramos.push(ramo(t, pedido.modo, &mut valores));
        }
    }
    let uniao = ramos
        .iter()
        .map(|r| format!("({})", r.cond))
        .collect::<Vec<_>>()
        .join(" OR ");
    let cond = format!("({filtro}) AND ({uniao})");
    let total = conn.query_row(
        &format!("SELECT COUNT(*) FROM artefato a WHERE {cond}"),
        params_from_iter(valores.iter()),
        |r| r.get(0),
    )?;
    controle.verificar()?;
    let flags = ramos
        .iter()
        .map(|r| format!("coalesce(({}), 0)", r.cond))
        .collect::<Vec<_>>()
        .join(", ");
    let lim = parametro(&mut valores, ValorSql::Integer(limite));
    let off = parametro(&mut valores, ValorSql::Integer(deslocamento));
    let mut stmt = conn.prepare(&format!("SELECT {}, {flags} FROM artefato a WHERE {cond} ORDER BY a.data_hora_ms IS NULL, a.data_hora_ms DESC, a.id DESC LIMIT {lim} OFFSET {off}", colunas_achado()))?;
    let mut itens = stmt
        .query_map(params_from_iter(valores.iter()), |r| {
            let achado = ler_achado(r, false)?;
            let motivos = ramos
                .iter()
                .enumerate()
                .filter_map(|(i, ramo)| match r.get::<_, bool>(N_RESUMO + 2 + i) {
                    Ok(true) => Some(Ok(ramo.motivo.clone())),
                    Ok(false) => None,
                    Err(e) => Some(Err(e)),
                })
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(AchadoDescoberta {
                achado,
                motivos,
                indicadores: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    controle.verificar()?;
    if pedido.modo == ModoDescoberta::Indicadores {
        for item in &mut itens {
            item.indicadores = indicadores_achado(&item.achado, pedido.indicador, controle)?;
        }
    }
    // Inventário não possui app/tipo/data. Excluí-lo, em vez de ignorar esses filtros.
    let arquivos = if pedido.modo == ModoDescoberta::Indicadores
        || pedido.filtro.app.as_ref().is_some_and(|v| !v.is_empty())
        || pedido.filtro.tipo.as_ref().is_some_and(|v| !v.is_empty())
        || pedido.filtro.de_ms.is_some()
        || pedido.filtro.ate_ms.is_some()
    {
        vazio()
    } else {
        // Os parâmetros originais são reaproveitados; cada ramo seleciona uma linha
        // do inventário no máximo uma vez, mesmo quando vários termos a encontram.
        let cond = ramos
            .iter()
            .filter_map(|r| r.campo_arquivo.as_ref())
            .map(|c| format!("({c})"))
            .collect::<Vec<_>>()
            .join(" OR ");
        if cond.is_empty() {
            vazio()
        } else {
            let extracao = pedido
                .filtro
                .extracao_id
                .map(|_| " AND f.extracao_id = ?1")
                .unwrap_or("");
            let cond = format!("({cond}){extracao}");
            // SQLite exige que todos os parâmetros numerados sejam ligados, mesmo
            // quando algum número é apenas um intervalo sem uso na consulta.
            let total = conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM arquivo f WHERE {cond} AND {lim} >= 1 AND {off} >= 0"
                ),
                params_from_iter(valores.iter()),
                |r| r.get(0),
            )?;
            controle.verificar()?;
            let mut stmt = conn.prepare(&format!("SELECT f.id, f.extracao_id, f.caminho, f.categoria, f.tamanho, f.sha256 FROM arquivo f WHERE {cond} ORDER BY f.caminho, f.id LIMIT {lim} OFFSET {off}"))?;
            let itens = stmt
                .query_map(params_from_iter(valores.iter()), |r| {
                    Ok(AchadoArquivo {
                        id: r.get(0)?,
                        extracao_id: uuid(r.get(1)?)?,
                        caminho: r.get(2)?,
                        categoria: r.get(3)?,
                        tamanho: r.get(4)?,
                        sha256: r.get(5)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            controle.verificar()?;
            Pagina { total, itens }
        }
    };
    Ok(ResultadoDescoberta {
        modo: pedido.modo,
        termos_efetivos: efetivos,
        artefatos: Pagina { total, itens },
        arquivos,
        algoritmo: "mdfml-ai0-local-v1".into(),
        executada_em: chrono::Utc::now().to_rfc3339(),
    })
}

fn codigo(tipo: Option<TipoIndicador>) -> i64 {
    match tipo {
        None => 0,
        Some(TipoIndicador::Ip) => 1,
        Some(TipoIndicador::Email) => 2,
        Some(TipoIndicador::Url) => 3,
        Some(TipoIndicador::Dominio) => 4,
        Some(TipoIndicador::Hash) => 5,
        Some(TipoIndicador::Telefone) => 6,
        Some(TipoIndicador::Coordenada) => 7,
    }
}

fn registrar_indicador(
    conn: &Connection,
    controle: &Arc<ControleDescoberta>,
) -> rusqlite::Result<()> {
    let controle = Arc::clone(controle);
    conn.create_scalar_function(
        "sied_ai0_indicador",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            let tipo: i64 = ctx.get(0)?;
            let texto = ctx.get_raw(1);
            let texto = match texto {
                rusqlite::types::ValueRef::Text(t) => std::str::from_utf8(t).unwrap_or(""),
                _ => "",
            };
            let mut existe = false;
            percorrer_indicadores(texto, tipo, &controle, |_, _| {
                existe = true;
                false
            })
            .map_err(|_| {
                // O progress handler não executa dentro da UDF. Usar o mesmo
                // código de interrupção, sem fabricar false ou contagem parcial.
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERRUPT),
                    None,
                )
            })?;
            Ok(existe)
        },
    )
}

fn dominio(t: &str) -> bool {
    if !t.is_ascii() || t.len() > 253 || t.parse::<IpAddr>().is_ok() {
        return false;
    }
    let mut partes = t.split('.').peekable();
    let mut n = 0;
    while let Some(p) = partes.next() {
        n += 1;
        if p.is_empty()
            || p.len() > 63
            || p.starts_with('-')
            || p.ends_with('-')
            || !p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return false;
        }
        if partes.peek().is_none() && (p.len() < 2 || !p.bytes().all(|c| c.is_ascii_alphabetic())) {
            return false;
        }
    }
    n >= 2
}

fn email(t: &str) -> bool {
    let Some((local, host)) = t.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&c))
        && dominio(host)
}

fn host_url(t: &str) -> Option<&str> {
    let inicio = if t
        .get(..8)
        .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
    {
        8
    } else if t
        .get(..7)
        .is_some_and(|s| s.eq_ignore_ascii_case("http://"))
    {
        7
    } else {
        return None;
    };
    let host = t.get(inicio..)?.split(['/', '?', '#']).next()?;
    if host.is_empty() || host.contains('@') {
        return None;
    }
    if host.starts_with('[') {
        let fim = host.find(']')?;
        let ip = host.get(1..fim)?;
        let resto = &host[fim + 1..];
        if ip.parse::<std::net::Ipv6Addr>().is_err() || (!resto.is_empty() && !porta(resto)) {
            return None;
        }
        Some(ip)
    } else {
        let (nome, sufixo) = host
            .split_once(':')
            .map_or((host, None), |(n, p)| (n, Some(p)));
        if sufixo.is_some_and(|s| s.parse::<u16>().is_err()) {
            return None;
        }
        (dominio(nome) || nome.parse::<IpAddr>().is_ok()).then_some(nome)
    }
}

fn porta(t: &str) -> bool {
    t.strip_prefix(':')
        .is_some_and(|p| p.parse::<u16>().is_ok())
}

// O SQLite não chama o progress handler durante uma UDF. Os próprios percursos
// de caracteres precisam ceder cooperativamente, inclusive em um único campo
// enorme sem separadores. Uma interrupção nunca vira separador/indicador válido:
// o controle é revalidado antes de consumir qualquer trecho produzido.
fn interrompido_no_passo(controle: &ControleDescoberta, passos: &mut usize) -> bool {
    *passos += 1;
    if *passos == 1_024 {
        *passos = 0;
        controle.verificar().is_err()
    } else {
        false
    }
}

fn aparar_inicio<'a>(
    texto: &'a str,
    controle: &ControleDescoberta,
    mut remover: impl FnMut(char) -> bool,
) -> Resultado<&'a str> {
    controle.verificar()?;
    let mut inicio = 0;
    let mut passos = 0;
    for (i, c) in texto.char_indices() {
        if interrompido_no_passo(controle, &mut passos) {
            controle.verificar()?;
        }
        if !remover(c) {
            break;
        }
        inicio = i + c.len_utf8();
    }
    controle.verificar()?;
    Ok(&texto[inicio..])
}

fn aparar_fim<'a>(
    texto: &'a str,
    controle: &ControleDescoberta,
    mut remover: impl FnMut(char) -> bool,
) -> Resultado<&'a str> {
    controle.verificar()?;
    let mut fim = texto.len();
    let mut passos = 0;
    for (i, c) in texto.char_indices().rev() {
        if interrompido_no_passo(controle, &mut passos) {
            controle.verificar()?;
        }
        if !remover(c) {
            break;
        }
        fim = i;
    }
    controle.verificar()?;
    Ok(&texto[..fim])
}

fn aparar<'a>(
    texto: &'a str,
    controle: &ControleDescoberta,
    mut remover: impl FnMut(char) -> bool,
) -> Resultado<&'a str> {
    let texto = aparar_inicio(texto, controle, &mut remover)?;
    aparar_fim(texto, controle, remover)
}

fn virgula(texto: &str, controle: &ControleDescoberta) -> Resultado<Option<usize>> {
    for (i, parte) in texto.as_bytes().chunks(4_096).enumerate() {
        controle.verificar()?;
        if let Some(posicao) = parte.iter().position(|c| *c == b',') {
            return Ok(Some(i * 4_096 + posicao));
        }
    }
    controle.verificar()?;
    Ok(None)
}

/// Reconhecimento conservador, sem regex/modelo, domínio DNS ou consulta externa.
/// Strings encontradas são candidatas sintáticas, não identificações confirmadas.
/// Callback `false` interrompe cedo (contagem não acumula resultados na memória).
fn percorrer_indicadores(
    mut texto: &str,
    procurado: i64,
    controle: &ControleDescoberta,
    mut emitir: impl FnMut(TipoIndicador, &str) -> bool,
) -> Resultado<()> {
    controle.verificar()?;
    let mut continuar = |tipo: TipoIndicador, valor: &str| {
        procurado != 0 && procurado != codigo(Some(tipo)) || emitir(tipo, valor)
    };
    let mut passos = 0;
    for t in texto.split(|c: char| {
        interrompido_no_passo(controle, &mut passos)
            || c.is_whitespace()
            || ['"', '\'', '<', '>', ';', ',', '(', ')', '{', '}'].contains(&c)
    }) {
        controle.verificar()?;
        let t = aparar(t, controle, |c| ['.', '!', '?', '[', ']'].contains(&c))?;
        if t.is_empty() || t.len() > 2048 {
            continue;
        }
        if t.parse::<IpAddr>().is_ok() && !continuar(TipoIndicador::Ip, t) {
            return controle.verificar();
        }
        if email(t) && !continuar(TipoIndicador::Email, t) {
            return controle.verificar();
        }
        if let Some(host) = host_url(t) {
            if !continuar(TipoIndicador::Url, t) {
                return controle.verificar();
            }
            if dominio(host) && !continuar(TipoIndicador::Dominio, host) {
                return controle.verificar();
            }
            if host.parse::<IpAddr>().is_ok() && !continuar(TipoIndicador::Ip, host) {
                return controle.verificar();
            }
        } else if dominio(t) && !continuar(TipoIndicador::Dominio, t) {
            return controle.verificar();
        }
        if matches!(t.len(), 32 | 40 | 64 | 128)
            && t.bytes().all(|c| c.is_ascii_hexdigit())
            && !continuar(TipoIndicador::Hash, t)
        {
            return controle.verificar();
        }
    }
    // Telefones: grupos com formatação ou 8–15 dígitos contíguos. Um grupo
    // inteiro fora desse intervalo não é truncado para fabricar um telefone.
    if procurado == 0 || procurado == 6 {
        let mut passos = 0;
        for grupo in texto.split(|c: char| {
            interrompido_no_passo(controle, &mut passos)
                || !(c.is_ascii_digit() || "+-(). ".contains(c))
        }) {
            controle.verificar()?;
            let g = aparar(grupo, controle, |c| [' ', '.', '-', '(', ')'].contains(&c))?;
            let mut digitos = 0;
            for parte in g.as_bytes().chunks(4_096) {
                controle.verificar()?;
                digitos += parte.iter().filter(|c| c.is_ascii_digit()).count();
            }
            if (8..=15).contains(&digitos)
                && !g.contains("..")
                && !g.contains('.')
                && !continuar(TipoIndicador::Telefone, g)
            {
                return controle.verificar();
            }
        }
    }
    // Pares decimais de latitude/longitude separados por vírgula, sem criar
    // localização a partir de inteiros/IDs ou de valores fora dos intervalos.
    if procurado == 0 || procurado == 7 {
        while let Some(i) = virgula(texto, controle)? {
            let esquerda = &texto[..i];
            let direita = &texto[i + 1..];
            let esquerda_aparada = aparar_fim(esquerda, controle, char::is_whitespace)?;
            let direita_aparada = aparar_inicio(direita, controle, char::is_whitespace)?;
            let mut passos = 0;
            let inicio = esquerda_aparada
                .char_indices()
                .rev()
                .find(|(_, c)| {
                    interrompido_no_passo(controle, &mut passos)
                        || !(c.is_ascii_digit() || *c == '.' || *c == '-')
                })
                .map_or(0, |(i, c)| i + c.len_utf8());
            controle.verificar()?;
            let lat = aparar(&esquerda[inicio..], controle, char::is_whitespace)?;
            let mut passos = 0;
            let fim = direita_aparada
                .find(|c: char| {
                    interrompido_no_passo(controle, &mut passos)
                        || !(c.is_ascii_digit() || c == '.' || c == '-')
                })
                .unwrap_or(direita_aparada.len());
            controle.verificar()?;
            let lon = &direita_aparada[..fim];
            let limite_esquerdo = texto[..inicio]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_alphanumeric());
            let limite_direito = direita_aparada[fim..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric() && !matches!(c, '.' | '-' | '+'));
            if limite_esquerdo
                && limite_direito
                && lat.contains('.')
                && lon.contains('.')
                && lat
                    .parse::<f64>()
                    .ok()
                    .zip(lon.parse::<f64>().ok())
                    .is_some_and(|(a, b)| {
                        a.is_finite()
                            && b.is_finite()
                            && (-90.0..=90.0).contains(&a)
                            && (-180.0..=180.0).contains(&b)
                    })
            {
                controle.verificar()?;
                let fim_original = i + 1 + direita.len() - direita_aparada.len() + fim;
                if !continuar(TipoIndicador::Coordenada, &texto[inicio..fim_original]) {
                    return controle.verificar();
                }
            }
            controle.verificar()?;
            texto = direita;
        }
    }
    controle.verificar()
}

fn indicadores_achado(
    achado: &AchadoArtefato,
    tipo: Option<TipoIndicador>,
    controle: &ControleDescoberta,
) -> Resultado<Vec<IndicadorDescoberta>> {
    let a = &achado.artefato;
    let mut v: Vec<IndicadorDescoberta> = Vec::new();
    for (campo, texto) in [
        ("conteudo", a.conteudo.as_deref()),
        ("remetente", a.remetente.as_deref()),
        ("nome_arquivo", a.nome_arquivo.as_deref()),
    ]
    .into_iter()
    .chain(
        a.destinatarios
            .iter()
            .map(|d| ("destinatarios", Some(d.as_str()))),
    ) {
        controle.verificar()?;
        if v.len() >= MAX_INDICADORES {
            break;
        }
        if let Some(texto) = texto {
            percorrer_indicadores(texto, codigo(tipo), controle, |tipo, valor| {
                if !v
                    .iter()
                    .any(|x| x.tipo == tipo && x.campo == campo && x.valor == valor)
                {
                    v.push(IndicadorDescoberta {
                        tipo,
                        valor: valor.into(),
                        campo: campo.into(),
                    });
                }
                v.len() < MAX_INDICADORES
            })?;
        }
    }
    if tipo.is_none_or(|t| t == TipoIndicador::Coordenada) && v.len() < MAX_INDICADORES {
        if let Some((lat, lon)) = a.latitude.zip(a.longitude).filter(|(a, b)| {
            a.is_finite()
                && b.is_finite()
                && (-90.0..=90.0).contains(a)
                && (-180.0..=180.0).contains(b)
        }) {
            v.push(IndicadorDescoberta {
                tipo: TipoIndicador::Coordenada,
                valor: format!("{lat}, {lon}"),
                campo: "latitude/longitude".into(),
            });
        }
    }
    controle.verificar()?;
    Ok(v)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cancela_percurso_antes_de_consumir_campo_grande_sem_resultado_parcial() {
        let controle = ControleDescoberta::novo();
        let texto = format!("192.0.2.1 {} 198.51.100.1", "x".repeat(32_768));
        let mut emitidos = 0;
        let resultado = percorrer_indicadores(&texto, 1, &controle, |_, _| {
            emitidos += 1;
            controle.cancelar();
            true
        });
        assert!(matches!(
            resultado,
            Err(ErroArmazenamento::DescobertaCancelada)
        ));
        assert_eq!(emitidos, 1);
    }

    #[test]
    fn udf_interrompe_durante_sql_e_guarda_remove_captura() {
        let conn = Connection::open_in_memory().unwrap();
        let controle = ControleDescoberta::novo();
        let fraco = Arc::downgrade(&controle);
        let no_sql = Arc::clone(&controle);
        conn.create_scalar_function(
            "cancelar_no_sql",
            0,
            FunctionFlags::SQLITE_UTF8,
            move |_| {
                no_sql.cancelar();
                Ok("192.0.2.1".to_string())
            },
        )
        .unwrap();
        let mut guarda = GuardaDescoberta::instalar(&conn, &controle);
        registrar_indicador(&conn, &controle).unwrap();
        guarda.indicador_registrado();
        // Poucas operações: a interrupção vem da própria UDF, não do hook.
        let erro = conn
            .query_row("SELECT sied_ai0_indicador(1, cancelar_no_sql())", [], |r| {
                r.get::<_, bool>(0)
            })
            .unwrap_err();
        assert_eq!(
            erro.sqlite_error_code(),
            Some(rusqlite::ErrorCode::OperationInterrupted)
        );
        assert!(matches!(
            controle.traduzir_sqlite(erro.into()),
            ErroArmazenamento::DescobertaCancelada
        ));
        guarda.limpar().unwrap();
        drop(guarda);
        conn.remove_function("cancelar_no_sql", 0).unwrap();
        assert_eq!(
            conn.query_row("SELECT 7", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            7
        );
        assert!(conn
            .query_row("SELECT sied_ai0_indicador(1, '192.0.2.1')", [], |r| r
                .get::<_, bool>(0))
            .is_err());
        drop(controle);
        assert!(fraco.upgrade().is_none());
    }

    #[test]
    fn funcao_e_hook_sao_limpos_no_retorno_normal_e_erro() {
        fn executar(
            conn: &Connection,
            controle: &Arc<ControleDescoberta>,
            sql: &str,
        ) -> rusqlite::Result<bool> {
            let mut guarda = GuardaDescoberta::instalar(conn, controle);
            registrar_indicador(conn, controle)?;
            guarda.indicador_registrado();
            conn.query_row(sql, [], |r| r.get(0))
        }
        let conn = Connection::open_in_memory().unwrap();
        for sql in [
            "SELECT sied_ai0_indicador(1, '192.0.2.1')",
            "SELECT * FROM tabela_ausente",
        ] {
            let controle = ControleDescoberta::novo();
            let fraco = Arc::downgrade(&controle);
            let resultado = executar(&conn, &controle, sql);
            assert_eq!(resultado.is_ok(), sql.starts_with("SELECT sied"));
            assert!(controle.cancelar());
            assert!(conn
                .query_row("SELECT sied_ai0_indicador(1, '192.0.2.1')", [], |r| r
                    .get::<_, bool>(0))
                .is_err());
            assert_eq!(
                conn.query_row("SELECT 1", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                1
            );
            drop(controle);
            assert!(fraco.upgrade().is_none());
        }
    }

    fn extrair(t: &str, tipo: TipoIndicador) -> Vec<String> {
        let mut v = Vec::new();
        percorrer_indicadores(
            t,
            codigo(Some(tipo)),
            &ControleDescoberta::novo(),
            |_, valor| {
                v.push(valor.into());
                true
            },
        )
        .unwrap();
        v
    }
    #[test]
    fn sintaxe_nao_inventa_indicadores() {
        assert_eq!(
            extrair("999.1.2.3 192.0.2.1 [2001:db8::1]", TipoIndicador::Ip),
            ["192.0.2.1", "2001:db8::1"]
        );
        assert_eq!(
            extrair(
                "teste@example.invalid err@@example.invalid",
                TipoIndicador::Email
            ),
            ["teste@example.invalid"]
        );
        assert_eq!(
            extrair(
                "https://example.invalid/a javascript:alert(1) https://example.invalid:abc/a",
                TipoIndicador::Url
            ),
            ["https://example.invalid/a"]
        );
        assert_eq!(
            extrair(
                "-23.55052, -46.633308 999.0, 20.0",
                TipoIndicador::Coordenada
            ),
            ["-23.55052, -46.633308"]
        );
        assert_eq!(
            extrair("2026 1234 1234567890123456", TipoIndicador::Telefone),
            Vec::<String>::new()
        );
        assert!(extrair("12.3, 21.4e400 abc12.3, 21.4", TipoIndicador::Coordenada).is_empty());
    }
    #[test]
    fn texto_hostil_e_unicode_sao_dados() {
        assert!(extrair("'; DROP TABLE artefato; -- 🧑‍🔬🔎", TipoIndicador::Url).is_empty());
        assert!(possui_emoji("café ☕️"));
        assert!(possui_emoji("👨‍👩‍👧‍👦"));
        assert!(possui_emoji("1️⃣"));
        assert!(possui_emoji("documento ©"));
    }
}
