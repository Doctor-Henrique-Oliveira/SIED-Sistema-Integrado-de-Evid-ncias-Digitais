//! Lista de itens de um caso processado pelo IPED (`FileList.csv`, ou "Lista de
//! Arquivos.csv" na versão em português).
//!
//! Formato conferido no código-fonte público do IPED (`ExportCSVTask`, projeto
//! sepinf-inc/IPED, e os arquivos de tradução `iped-engine-messages*.properties`):
//! - UTF-8 com BOM, linhas terminadas em CRLF, todos os campos entre aspas, aspas
//!   internas dobradas e caracteres de controle removidos pelo próprio IPED;
//! - separador `,` (inglês, espanhol, francês, italiano) ou `;` (português, alemão);
//! - 15 colunas, sempre nesta ordem (o nome muda com o idioma): nome, atalho, tamanho,
//!   extensão, marcador, categoria, MD5, SHA1, apagado, recuperado, acesso, modificação,
//!   criação, caminho e TrackId;
//! - marcadores e categorias separados por " | ";
//! - datas no formato de `java.util.Date.toString()` (`EEE MMM dd HH:mm:ss zzz yyyy`,
//!   nomes em inglês) no fuso da máquina que processou o caso;
//! - atalho: `=HYPERLINK("<caminho>","Open")` (ou `=HIPERLINK(...;"Abrir")`) quando o
//!   caso tem relatório com o arquivo exportado; senão, o identificador interno do item.
//!
//! Um registro "arquivo" por linha, com a linha inteira em `valores_brutos`. O fuso da
//! data só é convertido quando é inequívoco (UTC/GMT, deslocamento explícito como
//! `GMT-03:00`, ou os fusos de Brasília `BRT`/`BRST` e de Fernando de Noronha `FNT`);
//! qualquer outra abreviação fica só no valor original, com aviso.

use serde_json::{json, Map, Value};
use sied_dominio::artefato::{
    DataHora, EstadoRegistro, FormatoData, NovoArtefato, ReferenciaRegistro, TipoArtefato,
};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

use super::comum::Emissor;
use crate::parser::{EntradaParser, Parser, ResumoParser, SaidaParser};
use crate::{ErroIngestao, InfoComponente, Resultado};

pub const ID: &str = "iped_lista_arquivos";
pub const PAPEL: &str = "lista_iped";
/// Valor de `app` dos registros: a origem é a lista de itens do IPED.
pub const APP: &str = "iped";
/// Nomes do arquivo nas traduções do IPED (inglês, português, alemão, espanhol,
/// francês e italiano).
pub const NOMES_CSV: &[&str] = &[
    "FileList.csv",
    "Lista de Arquivos.csv",
    "DateiListe.csv",
    "ListaDeArchivos.csv",
    "Liste de fichiers.csv",
    "ElencoFile.csv",
];
const COLUNAS: usize = 15;
/// Linha lógica maior que isso não é de uma lista do IPED (proteção contra arquivo
/// que não termina aspas).
const MAX_LINHA: usize = 1 << 20;

pub struct ParserListaIped;

/// Posições fixas das colunas (a ordem vem de `ExportCSVTask.process`).
mod col {
    pub const NOME: usize = 0;
    pub const ATALHO: usize = 1;
    pub const TAMANHO: usize = 2;
    pub const EXTENSAO: usize = 3;
    pub const MARCADOR: usize = 4;
    pub const CATEGORIA: usize = 5;
    pub const MD5: usize = 6;
    pub const SHA1: usize = 7;
    pub const APAGADO: usize = 8;
    pub const RECUPERADO: usize = 9;
    pub const ACESSO: usize = 10;
    pub const MODIFICACAO: usize = 11;
    pub const CRIACAO: usize = 12;
    pub const CAMINHO: usize = 13;
    pub const TRACK_ID: usize = 14;
}

/// Separa uma linha lógica em campos (aspas duplas, `""` = aspas dentro do campo).
fn campos(linha: &str, sep: char) -> Vec<String> {
    let mut v = Vec::with_capacity(COLUNAS);
    let mut atual = String::new();
    let mut aspas = false;
    let mut chars = linha.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if aspas && chars.peek() == Some(&'"') => {
                atual.push('"');
                chars.next();
            }
            '"' => aspas = !aspas,
            c if c == sep && !aspas => v.push(std::mem::take(&mut atual)),
            c => atual.push(c),
        }
    }
    v.push(atual);
    v
}

/// Mês de `Date.toString()` (sempre em inglês).
fn mes(m: &str) -> Option<u32> {
    let i = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ]
    .iter()
    .position(|x| *x == m)?;
    Some(i as u32 + 1)
}

/// Deslocamento, em minutos, de um fuso inequívoco. `None` = não reconhecido.
fn deslocamento(fuso: &str) -> Option<i32> {
    match fuso {
        "UTC" | "GMT" | "UT" | "Z" => return Some(0),
        "BRT" => return Some(-180),
        "BRST" | "FNT" => return Some(-120),
        _ => {}
    }
    // GMT-03:00, GMT+0530, UTC-3
    let resto = fuso
        .strip_prefix("GMT")
        .or_else(|| fuso.strip_prefix("UTC"))?;
    let (sinal, num) = match resto.chars().next()? {
        '+' => (1, &resto[1..]),
        '-' => (-1, &resto[1..]),
        _ => return None,
    };
    let (h, m) = match num.split_once(':') {
        Some((h, m)) => (h, m),
        None if num.len() == 4 => num.split_at(2),
        None => (num, "0"),
    };
    let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
    (h <= 14 && m < 60).then_some(sinal * (h * 60 + m))
}

/// Resultado da leitura de uma data do IPED.
enum Data {
    Vazia,
    Convertida(i64),
    /// Formato reconhecido, fuso não: fica só o valor original.
    FusoDesconhecido(String),
    Ilegivel,
}

/// Separador e nomes das colunas, se a linha for o cabeçalho da lista de itens do IPED
/// (15 colunas com MD5, SHA1 e TrackId nas posições fixas; esses três nomes não mudam
/// com o idioma).
pub fn ler_cabecalho(linha: &str) -> Option<(char, Vec<String>)> {
    let cab = linha
        .trim_start_matches('\u{feff}')
        .trim_end_matches(['\r', '\n']);
    let sep = if cab.contains("\";\"") { ';' } else { ',' };
    let nomes = campos(cab, sep);
    let reconhecido = nomes.len() == COLUNAS
        && nomes[col::MD5] == "MD5"
        && nomes[col::SHA1] == "SHA1"
        && nomes[col::TRACK_ID] == "TrackId";
    reconhecido.then_some((sep, nomes))
}

/// `EEE MMM dd HH:mm:ss zzz yyyy` → ms UTC.
fn ler_data(s: &str) -> Data {
    let s = s.trim();
    if s.is_empty() {
        return Data::Vazia;
    }
    let p: Vec<&str> = s.split_whitespace().collect();
    let [_, m, d, hms, fuso, a] = p[..] else {
        return Data::Ilegivel;
    };
    let hms: Vec<&str> = hms.split(':').collect();
    let [h, mi, se] = hms[..] else {
        return Data::Ilegivel;
    };
    let (Some(mes), Ok(dia), Ok(ano)) = (mes(m), d.parse::<u32>(), a.parse::<i32>()) else {
        return Data::Ilegivel;
    };
    let (Ok(h), Ok(mi), Ok(se)) = (h.parse::<u32>(), mi.parse::<u32>(), se.parse::<u32>()) else {
        return Data::Ilegivel;
    };
    let Some(local) =
        chrono::NaiveDate::from_ymd_opt(ano, mes, dia).and_then(|x| x.and_hms_opt(h, mi, se))
    else {
        return Data::Ilegivel;
    };
    match deslocamento(fuso) {
        Some(min) => Data::Convertida(local.and_utc().timestamp_millis() - i64::from(min) * 60_000),
        None => Data::FusoDesconhecido(fuso.to_string()),
    }
}

/// Caminho de dentro de `=HYPERLINK("caminho","Open")` / `=HIPERLINK("caminho";"Abrir")`.
fn caminho_do_atalho(v: &str) -> Option<String> {
    let v = v.trim();
    if !v.starts_with('=') {
        return None;
    }
    let ini = v.find("(\"")? + 2;
    let fim = ini + v[ini..].find('"')?;
    let c = v[ini..fim].replace('\\', "/");
    (!c.is_empty()).then_some(c)
}

fn booleano(v: &str) -> Option<bool> {
    match v.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn lista(v: &str) -> Vec<String> {
    v.split(" | ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

impl Parser for ParserListaIped {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: ID.into(),
            nome: "Lista de itens do IPED".into(),
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
            parser: ID.into(),
            mensagem: format!("arquivo com papel '{PAPEL}' não informado"),
        })?;
        let mut leitor = BufReader::new(File::open(&arq.copia_verificada)?);
        let mut e = Emissor::novo(saida);

        // Cabeçalho: define o separador e os nomes das colunas (idioma do IPED).
        let mut cab = String::new();
        leitor.read_line(&mut cab)?;
        let Some((sep, nomes)) = ler_cabecalho(&cab) else {
            return Err(ErroIngestao::FormatoNaoReconhecido(format!(
                "{}: cabeçalho não é o da lista de itens do IPED (15 colunas, MD5, SHA1 e TrackId)",
                arq.origem
            )));
        };

        let mut fusos_desconhecidos = BTreeSet::new();
        let mut numero_linha = 1u64;
        let mut logica = String::new();
        let mut inicio_logica = 2u64;
        let mut buf = String::new();
        loop {
            buf.clear();
            let n = leitor.read_line(&mut buf)?;
            if n == 0 {
                break;
            }
            numero_linha += 1;
            if logica.is_empty() {
                inicio_logica = numero_linha;
            }
            logica.push_str(&buf);
            // Aspas abertas: o campo continua na próxima linha física.
            if logica.matches('"').count() % 2 == 1 && logica.len() < MAX_LINHA {
                continue;
            }
            let texto = std::mem::take(&mut logica);
            let texto = texto.trim_end_matches(['\r', '\n']);
            if texto.trim().is_empty() {
                continue;
            }
            let id_registro = format!("linha={inicio_logica}");
            let v = campos(texto, sep);
            if v.len() != COLUNAS {
                e.avisar(
                    format!(
                        "linha com {} coluna(s) em vez de {COLUNAS}: não interpretada",
                        v.len()
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
                tabela: None,
                id_registro: Some(id_registro.clone()),
                consulta: None,
                deslocamento: None,
                valores_brutos: brutos,
            };
            let mut a = NovoArtefato::novo(TipoArtefato::Arquivo, origem);
            a.app = Some(APP.into());
            let nao_vazio = |i: usize| {
                Some(v[i].trim())
                    .filter(|s| !s.is_empty())
                    .map(String::from)
            };
            a.nome_arquivo = nao_vazio(col::NOME);
            a.id_externo = nao_vazio(col::TRACK_ID);
            match v[col::TAMANHO].trim() {
                "" => {}
                t => match t.parse::<u64>() {
                    Ok(n) => a.tamanho = Some(n),
                    Err(_) => e.avisar(
                        format!("tamanho não numérico: {t}"),
                        PAPEL,
                        Some(id_registro.clone()),
                    ),
                },
            }
            let apagado = booleano(&v[col::APAGADO]);
            let recuperado = booleano(&v[col::RECUPERADO]);
            if apagado == Some(true) {
                a.estado = EstadoRegistro::Apagado;
            }
            a.subtipo = Some(
                if recuperado == Some(true) {
                    "recuperado"
                } else {
                    "item"
                }
                .into(),
            );
            if let Some(c) = caminho_do_atalho(&v[col::ATALHO]) {
                a.anexo_caminho = Some(c);
            }

            let mut m = Map::new();
            for (chave, i) in [
                ("caminho", col::CAMINHO),
                ("extensao", col::EXTENSAO),
                ("md5", col::MD5),
                ("sha1", col::SHA1),
                ("atalho", col::ATALHO),
            ] {
                if let Some(s) = nao_vazio(i) {
                    m.insert(chave.into(), json!(s));
                }
            }
            for (chave, i) in [
                ("marcadores", col::MARCADOR),
                ("categorias", col::CATEGORIA),
            ] {
                let l = lista(&v[i]);
                if !l.is_empty() {
                    m.insert(chave.into(), json!(l));
                }
            }
            if let Some(b) = apagado {
                m.insert("apagado".into(), json!(b));
            }
            if let Some(b) = recuperado {
                m.insert("recuperado".into(), json!(b));
            }
            for (chave, i) in [
                ("modificacao", col::MODIFICACAO),
                ("acesso", col::ACESSO),
                ("criacao", col::CRIACAO),
            ] {
                let original = v[i].trim();
                let ms = match ler_data(original) {
                    Data::Vazia => continue,
                    Data::Convertida(ms) => DataHora::ms_plausivel(ms),
                    Data::FusoDesconhecido(f) => {
                        fusos_desconhecidos.insert(f);
                        None
                    }
                    Data::Ilegivel => {
                        e.avisar(
                            format!("data de {chave} em formato não reconhecido: {original}"),
                            PAPEL,
                            Some(id_registro.clone()),
                        );
                        None
                    }
                };
                m.insert(format!("{chave}_original"), json!(original));
                if let Some(ms) = ms {
                    m.insert(format!("{chave}_ms"), json!(ms));
                }
                if chave == "modificacao" {
                    a.data_hora = Some(DataHora {
                        utc_ms: ms,
                        original: original.into(),
                        formato: FormatoData::Texto,
                    });
                }
            }
            a.metadados = m;
            e.emitir(a)?;
        }
        if !fusos_desconhecidos.is_empty() {
            e.avisar(
                format!(
                    "datas com fuso {:?}: a abreviação é ambígua ou desconhecida, então essas datas \
                     ficaram só com o valor original (sem entrar na linha do tempo)",
                    fusos_desconhecidos
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
    fn campos_com_aspas_e_separadores() {
        assert_eq!(
            campos(r#""a;b";"c ""x""";"";"d""#, ';'),
            vec!["a;b", "c \"x\"", "", "d"]
        );
        assert_eq!(campos(r#""1","2""#, ','), vec!["1", "2"]);
    }

    #[test]
    fn datas_do_java() {
        let ms = |s: &str| match ler_data(s) {
            Data::Convertida(ms) => Some(ms),
            _ => None,
        };
        // 02/01/2026 10:00:00 em Brasília = 13:00:00 UTC.
        assert_eq!(ms("Fri Jan 02 10:00:00 BRT 2026"), Some(1_767_358_800_000));
        assert_eq!(
            ms("Fri Jan 02 10:00:00 GMT-03:00 2026"),
            Some(1_767_358_800_000)
        );
        assert_eq!(ms("Fri Jan 02 13:00:00 UTC 2026"), Some(1_767_358_800_000));
        assert_eq!(
            ms("Fri Jan 02 18:30:00 GMT+0530 2026"),
            Some(1_767_358_800_000)
        );
        assert!(
            matches!(ler_data("Fri Jan 02 10:00:00 CST 2026"), Data::FusoDesconhecido(f) if f == "CST")
        );
        assert!(matches!(ler_data(""), Data::Vazia));
        assert!(matches!(ler_data("02/01/2026 10:00"), Data::Ilegivel));
        assert!(matches!(
            ler_data("Fri Foo 02 10:00:00 UTC 2026"),
            Data::Ilegivel
        ));
        assert!(matches!(
            ler_data("Fri Feb 30 10:00:00 UTC 2026"),
            Data::Ilegivel
        ));
    }

    #[test]
    fn atalho_e_listas() {
        assert_eq!(
            caminho_do_atalho(r#"=HIPERLINK("Exportados\arquivos\a\b.jpg";"Abrir")"#).as_deref(),
            Some("Exportados/arquivos/a/b.jpg")
        );
        assert_eq!(
            caminho_do_atalho(r#"=HYPERLINK("Exported/files/x.pdf","Open")"#).as_deref(),
            Some("Exported/files/x.pdf")
        );
        assert_eq!(caminho_do_atalho("12345"), None);
        assert_eq!(lista("Imagens | Fotos"), vec!["Imagens", "Fotos"]);
        assert!(lista("").is_empty());
    }
}
