//! Exportação TSV de um relatório do ALEAPP (Android) ou do iLEAPP (iOS).
//!
//! Formato conferido no código-fonte público dos dois projetos (abrignoni/ALEAPP e
//! abrignoni/iLEAPP, função `tsv` de `scripts/ilapfuncs.py`, a mesma nos dois):
//! - um arquivo por artefato em `<relatório>/_TSV Exports/<nome do artefato>.tsv`;
//! - UTF-8 com BOM, `csv.writer` do Python com separador TAB: campo com TAB, aspas ou
//!   quebra de linha vai entre aspas, aspas internas dobradas, linhas terminadas em CRLF;
//! - primeira linha com os nomes das colunas; o arquivo é aberto em modo "acrescentar",
//!   então o cabeçalho pode se repetir no meio, com outro BOM (quando o módulo grava duas
//!   vezes);
//! - datas como `str(datetime)` do Python: `2026-01-02 13:00:00+00:00` quando têm fuso,
//!   ou `2026-01-02 13:00:00` quando o módulo não informa o fuso;
//! - a linha do tempo do próprio xLEAPP usa a primeira coluna como data; o mapa usa as
//!   colunas `Latitude` e `Longitude`.
//!
//! Há um parser por ferramenta (`aleapp_tsv`, `ileapp_tsv` e `xleapp_tsv` quando o
//! relatório não diz qual foi), para que a execução e o `app` de cada registro digam de
//! onde veio. Um registro "outro" por linha, com o nome do artefato no subtipo e a linha inteira em
//! `valores_brutos`. A data só entra na linha do tempo quando traz o fuso; sem fuso, fica
//! só o valor original, com um aviso por arquivo.

use serde_json::{json, Map, Value};
use sied_dominio::artefato::{
    DataHora, FormatoData, NovoArtefato, ReferenciaRegistro, TipoArtefato,
};
use std::fs::File;
use std::io::{BufRead, BufReader};

use super::comum::Emissor;
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const PAPEL: &str = "tsv_leapp";
/// Pasta das exportações TSV dentro do relatório.
pub const PASTA_TSV: &str = "_TSV Exports";
/// Linha lógica maior que isso é recusada (arquivo com aspas que não fecham).
const MAX_LINHA: usize = 4 << 20;
/// Tamanho máximo do resumo textual da linha (a linha inteira fica nos brutos).
const MAX_CONTEUDO: usize = 2_000;

/// Ferramenta que gerou o relatório.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ferramenta {
    Aleapp,
    Ileapp,
    /// O relatório não identifica a ferramenta (só as exportações TSV).
    Desconhecida,
}

impl Ferramenta {
    /// Pelo `leapp_name` do projeto (`_lava_data.lava`).
    pub fn pelo_nome(nome: &str) -> Self {
        match nome.trim().to_ascii_lowercase().as_str() {
            "aleapp" => Ferramenta::Aleapp,
            "ileapp" => Ferramenta::Ileapp,
            _ => Ferramenta::Desconhecida,
        }
    }

    pub fn id_parser(self) -> &'static str {
        match self {
            Ferramenta::Aleapp => "aleapp_tsv",
            Ferramenta::Ileapp => "ileapp_tsv",
            Ferramenta::Desconhecida => "xleapp_tsv",
        }
    }

    /// Valor de `app` dos registros.
    pub fn app(self) -> &'static str {
        match self {
            Ferramenta::Aleapp => "aleapp",
            Ferramenta::Ileapp => "ileapp",
            Ferramenta::Desconhecida => "xleapp",
        }
    }

    pub fn nome(self) -> &'static str {
        match self {
            Ferramenta::Aleapp => "ALEAPP",
            Ferramenta::Ileapp => "iLEAPP",
            Ferramenta::Desconhecida => "ALEAPP/iLEAPP",
        }
    }
}

pub struct ParserLeappTsv(pub Ferramenta);

/// Separa uma linha lógica (sem o fim de linha) em campos.
fn campos(linha: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut atual = String::new();
    let mut aspas = false;
    let mut inicio_campo = true;
    let mut chars = linha.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if aspas && chars.peek() == Some(&'"') => {
                atual.push('"');
                chars.next();
            }
            '"' if aspas => aspas = false,
            // Aspas só abrem um campo no começo dele (como no módulo csv do Python).
            '"' if inicio_campo => aspas = true,
            '\t' if !aspas => {
                v.push(std::mem::take(&mut atual));
                inicio_campo = true;
                continue;
            }
            c => atual.push(c),
        }
        inicio_campo = false;
    }
    v.push(atual);
    v
}

/// Resultado da leitura de uma data do xLEAPP.
#[derive(Debug, PartialEq)]
enum Data {
    Convertida(i64),
    /// Data e hora válidas, mas sem fuso: não entra na linha do tempo.
    SemFuso,
    Nenhuma,
}

fn ler_data(s: &str) -> Data {
    let s = s.trim();
    if s.len() < 19 {
        return Data::Nenhuma;
    }
    let s = s.replacen('T', " ", 1);
    let s = s
        .strip_suffix('Z')
        .map(|x| format!("{x}+00:00"))
        .unwrap_or(s);
    if let Ok(d) = chrono::DateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f%:z") {
        return match DataHora::ms_plausivel(d.timestamp_millis()) {
            Some(ms) => Data::Convertida(ms),
            None => Data::Nenhuma,
        };
    }
    if chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f").is_ok() {
        return Data::SemFuso;
    }
    Data::Nenhuma
}

fn coordenada(v: &str, limite: f64) -> Option<f64> {
    let n: f64 = v.trim().parse().ok()?;
    (n.is_finite() && n.abs() <= limite).then_some(n)
}

/// Nome do artefato a partir do nome do arquivo (`SMS messages.tsv` → `SMS messages`).
pub fn nome_artefato(nome_arquivo: &str) -> &str {
    nome_arquivo.strip_suffix(".tsv").unwrap_or(nome_arquivo)
}

/// Resumo "coluna: valor" dos campos preenchidos, para a busca e a lista.
fn resumo(nomes: &[String], v: &[String]) -> Option<String> {
    let mut s = String::new();
    for (n, x) in nomes.iter().zip(v) {
        let x = x.trim();
        if x.is_empty() {
            continue;
        }
        if !s.is_empty() {
            s.push_str(" · ");
        }
        s.push_str(n.trim());
        s.push_str(": ");
        s.push_str(x);
        if s.len() > MAX_CONTEUDO {
            let mut corte = MAX_CONTEUDO;
            while !s.is_char_boundary(corte) {
                corte -= 1;
            }
            s.truncate(corte);
            s.push('…');
            break;
        }
    }
    (!s.is_empty()).then_some(s)
}

impl Parser for ParserLeappTsv {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: self.0.id_parser().into(),
            nome: format!("Exportação TSV do {}", self.0.nome()),
            versao: "1.0.0".into(),
        }
    }

    fn papeis(&self) -> &'static [&'static str] {
        &[PAPEL]
    }

    fn processar(
        &self,
        entrada: &EntradaParser,
        saida: &mut dyn SaidaParser,
    ) -> Resultado<ResumoParser> {
        let arq = entrada.arquivo(PAPEL).ok_or_else(|| ErroIngestao::Parser {
            parser: self.0.id_parser().into(),
            mensagem: format!("arquivo com papel '{PAPEL}' não informado"),
        })?;
        let artefato = nome_artefato(arq.origem.nome()).to_string();
        let app = self.0.app().to_string();
        let mut leitor = BufReader::new(File::open(&arq.copia_verificada)?);
        let mut e = Emissor::novo(saida);

        let mut cab = String::new();
        leitor.read_line(&mut cab)?;
        let cab = cab
            .trim_start_matches('\u{feff}')
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if cab.trim().is_empty() {
            return Ok(e.resumo);
        }
        let nomes = campos(&cab);
        let i_lat = nomes.iter().position(|n| n == "Latitude");
        let i_lon = nomes.iter().position(|n| n == "Longitude");

        let candidatas: Vec<usize> = nomes
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, n)| {
                let n = n.to_lowercase();
                n.contains("date") || n.contains("time")
            })
            .map(|(i, _)| i)
            .collect();
        let mut sem_fuso: std::collections::BTreeSet<String> = Default::default();
        let mut numero_linha = 1u64;
        let mut inicio_logica = 2u64;
        let mut logica = String::new();
        let mut buf = String::new();
        loop {
            buf.clear();
            if leitor.read_line(&mut buf)? == 0 {
                break;
            }
            numero_linha += 1;
            if logica.is_empty() {
                inicio_logica = numero_linha;
            }
            logica.push_str(&buf);
            if logica.matches('"').count() % 2 == 1 && logica.len() < MAX_LINHA {
                continue;
            }
            let texto = std::mem::take(&mut logica);
            // O ALEAPP abre o arquivo com `codecs.open(..., 'a', 'utf-8-sig')`, que grava o
            // BOM de novo a cada gravação: o cabeçalho repetido vem com ele.
            let texto = texto
                .trim_start_matches('\u{feff}')
                .trim_end_matches(['\r', '\n']);
            if texto.is_empty() || texto == cab {
                // Linha vazia ou o cabeçalho repetido (o arquivo é gravado em acréscimo).
                continue;
            }
            let id_registro = format!("linha={inicio_logica}");
            let v = campos(texto);
            if v.len() != nomes.len() {
                e.avisar(
                    format!(
                        "linha com {} coluna(s) em vez de {}: não interpretada",
                        v.len(),
                        nomes.len()
                    ),
                    PAPEL,
                    Some(id_registro),
                );
                continue;
            }
            let brutos: Map<String, Value> = nomes
                .iter()
                .cloned()
                .zip(v.iter().map(|s| json!(s)))
                .collect();
            let origem = ReferenciaRegistro {
                papel_arquivo: PAPEL.into(),
                tabela: Some(artefato.clone()),
                id_registro: Some(id_registro),
                consulta: None,
                deslocamento: None,
                valores_brutos: brutos,
            };
            let mut a = NovoArtefato::novo(TipoArtefato::Outro, origem);
            a.app = Some(app.clone());
            a.subtipo = Some(artefato.clone());
            a.conteudo = resumo(&nomes, &v);

            // Data do registro: a primeira coluna (como na linha do tempo do xLEAPP) ou,
            // se ela não for data, a primeira coluna com "date"/"time" no nome que for.
            let escolhida = std::iter::once(0)
                .chain(candidatas.iter().copied())
                .map(|i| (i, ler_data(&v[i])))
                .find(|(_, d)| *d != Data::Nenhuma);
            let mut coluna_data = None;
            if let Some((i, d)) = escolhida {
                let original = v[i].trim();
                let utc_ms = match d {
                    Data::Convertida(ms) => Some(ms),
                    _ => {
                        sem_fuso.insert(nomes[i].clone());
                        None
                    }
                };
                a.data_hora = Some(DataHora {
                    utc_ms,
                    original: original.into(),
                    formato: FormatoData::Texto,
                });
                coluna_data = Some(nomes[i].clone());
            }
            if let (Some(la), Some(lo)) = (i_lat, i_lon) {
                if let (Some(la), Some(lo)) = (coordenada(&v[la], 90.0), coordenada(&v[lo], 180.0))
                {
                    if la != 0.0 || lo != 0.0 {
                        a.latitude = Some(la);
                        a.longitude = Some(lo);
                    }
                }
            }
            let mut m = Map::new();
            m.insert("artefato".into(), json!(artefato));
            if let Some(c) = coluna_data {
                m.insert("coluna_data".into(), json!(c));
            }
            a.metadados = m;
            e.emitir(a)?;
        }
        if !sem_fuso.is_empty() {
            e.avisar(
                format!(
                    "datas sem fuso na(s) coluna(s) {sem_fuso:?}: ficaram só com o valor \
                     original (sem entrar na linha do tempo)"
                ),
                PAPEL,
                None,
            );
        }
        Ok(e.resumo)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn campos_do_csv_do_python() {
        assert_eq!(campos("a\tb\t\tc"), vec!["a", "b", "", "c"]);
        assert_eq!(
            campos("\"x\ty\"\t\"diz \"\"oi\"\"\"\tz"),
            vec!["x\ty", "diz \"oi\"", "z"]
        );
        assert_eq!(
            campos("\"linha 1\r\nlinha 2\"\t1"),
            vec!["linha 1\r\nlinha 2", "1"]
        );
        assert_eq!(campos("5\" tela\t1"), vec!["5\" tela", "1"]);
    }

    #[test]
    fn datas_do_python() {
        assert_eq!(
            ler_data("2026-01-02 13:00:00+00:00"),
            Data::Convertida(1_767_358_800_000)
        );
        assert_eq!(
            ler_data("2026-01-02 10:00:00.250000-03:00"),
            Data::Convertida(1_767_358_800_250)
        );
        assert_eq!(
            ler_data("2026-01-02T13:00:00Z"),
            Data::Convertida(1_767_358_800_000)
        );
        assert_eq!(ler_data("2026-01-02 13:00:00"), Data::SemFuso);
        assert_eq!(ler_data("João"), Data::Nenhuma);
        assert_eq!(ler_data("2026-02-30 13:00:00+00:00"), Data::Nenhuma);
    }

    #[test]
    fn resumo_e_nome() {
        let n = vec!["A".to_string(), "B".to_string()];
        assert_eq!(
            resumo(&n, &["1".into(), " ".into()]).as_deref(),
            Some("A: 1")
        );
        assert_eq!(resumo(&n, &["".into(), "".into()]), None);
        let longo = "é".repeat(3_000);
        let r = resumo(&n, &[longo, "x".into()]).unwrap();
        assert!(r.len() <= MAX_CONTEUDO + 4 && r.ends_with('…'));
        assert_eq!(nome_artefato("SMS messages.tsv"), "SMS messages");
    }
}
