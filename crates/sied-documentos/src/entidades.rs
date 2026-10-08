//! Candidatos sintáticos locais e determinísticos, nunca identidades ou fatos.
//!
//! Este perfil conservador procura e-mail ASCII, URL HTTP(S), domínio ASCII,
//! endereço IP aceito por `std::net`, alguns formatos explícitos de telefone e
//! IMEI contextualizado com dígito verificador Luhn válido.
//! Não consulta TAC, DNS, não abre URLs e não executa conteúdo. Nomes, endereços,
//! datas, valores e números soltos não são inferidos. Um formato reconhecido
//! não comprova existência, titularidade, comunicação ou relevância forense.
//!
//! O literal e os offsets em bytes referem-se ao texto recebido, sem converter
//! caixa, decodificar escapes ou normalizar Unicode. Os limites são explícitos:
//! texto acima de 2 MiB não é examinado; tokens acima de 512 bytes são omitidos;
//! no máximo 1.000 ocorrências são publicadas. `limite_atingido` indica qualquer
//! dessas omissões e não representa uma contagem do que deixou de ser visto.
//!
//! O reconhecimento é linear no texto, com operações internas limitadas a um
//! token. Ocorrências repetidas permanecem separadas. Revisão humana, origem,
//! página e hash pertencem ao chamador; esta análise não confirma candidatos.

use std::net::{IpAddr, Ipv6Addr};

pub const MOTOR_VERSAO: &str = "candidatos_sintaticos/2";
pub const MAX_TEXTO_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_TOKEN_BYTES: usize = 512;
pub const MAX_CANDIDATOS: usize = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidato {
    pub tipo: String,
    pub literal: String,
    /// Início inclusivo, em bytes UTF-8, não em caracteres ou colunas visuais.
    pub inicio: u32,
    /// Fim exclusivo, em bytes UTF-8.
    pub fim: u32,
    /// Identificador versionado da regra; não é um nível de confiança.
    pub regra: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnaliseEntidades {
    pub candidatos: Vec<Candidato>,
    pub limite_atingido: bool,
}

/// Analisa somente o texto entregue pelo chamador, sem E/S nem persistência.
///
/// IMEI exige rótulo explícito e checksum; números soltos não viram IMEI. A
/// prioridade dos demais tokens é URL, e-mail, IP, domínio e telefone. O host dentro
/// de uma URL/e-mail não gera uma segunda ocorrência; contagens não equivalem
/// a pessoas, endereços distintos nem entidades únicas. Pontuação editorial
/// externa não integra o candidato. Telefones têm regras deliberadamente
/// limitadas: `+` seguido de 10–15 dígitos; `(DD) NNNN-NNNN`/`NNNNN-NNNN`;
/// ou o mesmo formato com prefixo explícito `+55`. Outros formatos são omitidos.
pub fn analisar(texto: &str) -> AnaliseEntidades {
    let mut resultado = AnaliseEntidades::default();
    if texto.len() > MAX_TEXTO_BYTES {
        resultado.limite_atingido = true;
        return resultado;
    }

    let mut cursor = 0;
    while cursor < texto.len() {
        let Some((deslocamento, _)) = texto[cursor..]
            .char_indices()
            .find(|(_, caractere)| !caractere.is_whitespace())
        else {
            break;
        };
        let inicio_token = cursor + deslocamento;
        let fim_token = texto[inicio_token..]
            .char_indices()
            .find(|(_, caractere)| caractere.is_whitespace())
            .map_or(texto.len(), |(deslocamento, _)| inicio_token + deslocamento);
        cursor = fim_token;
        if fim_token - inicio_token > MAX_TOKEN_BYTES {
            resultado.limite_atingido = true;
            continue;
        }

        if let Some((inicio, fim)) = imei_contextualizado(texto, inicio_token) {
            if publicar(
                &mut resultado,
                texto,
                inicio,
                fim,
                "imei",
                "imei-rotulado-luhn-v1",
            ) {
                break;
            }
            cursor = fim;
            continue;
        }

        // Um telefone com DDD explícito pode ocupar alguns tokens, mas sua
        // leitura nunca ultrapassa o limite de um token a partir do início.
        let inicio_telefone = pular_abertura_telefone(texto, inicio_token, fim_token);
        if let Some(fim) = telefone_formatado(texto, inicio_telefone) {
            if publicar(
                &mut resultado,
                texto,
                inicio_telefone,
                fim,
                "telefone",
                "telefone-br-formatado-v1",
            ) {
                break;
            }
            cursor = fim;
            continue;
        }

        let (inicio, fim) = aparar_pontuacao(texto, inicio_token, fim_token);
        if inicio == fim {
            continue;
        }
        let literal = &texto[inicio..fim];
        let candidato = if url_http(literal) {
            Some(("url", "url-http-host-sintatico-v1"))
        } else if email_ascii(literal) {
            Some(("email", "email-ascii-sintatico-v1"))
        } else if literal.parse::<IpAddr>().is_ok() {
            Some(("ip", "ip-std-net-v1"))
        } else if dominio_ascii(literal) && !parece_arquivo(literal) {
            Some(("dominio", "dominio-ascii-sintatico-v1"))
        } else if telefone_internacional(literal) {
            Some(("telefone", "telefone-e164-sintatico-v1"))
        } else {
            None
        };
        if let Some((tipo, regra)) = candidato {
            if publicar(&mut resultado, texto, inicio, fim, tipo, regra) {
                break;
            }
        }
    }
    resultado
}

/// Verifica exclusivamente 15 dígitos ASCII e o checksum Luhn. Placeholders com
/// os primeiros 14 dígitos todos iguais são recusados, mesmo que o checksum
/// feche. Isto não confirma TAC, fabricante, modelo, existência ou titularidade.
/// A descoberta no texto exige também um rótulo IMEI; esta função pura serve a
/// contratos que já recebem um campo explicitamente identificado como IMEI.
pub fn imei_luhn_valido(literal: &str) -> bool {
    let bytes = literal.as_bytes();
    if bytes.len() != 15 || !bytes.iter().all(u8::is_ascii_digit) || digitos_repetidos(&bytes[..14])
    {
        return false;
    }
    let soma: u32 = bytes
        .iter()
        .enumerate()
        .map(|(indice, byte)| {
            let digito = u32::from(*byte - b'0');
            // Em um número de 15 dígitos, dobra-se a posição de índice ímpar;
            // o último dígito permanece o verificador observado na fonte.
            let valor = if indice % 2 == 1 { digito * 2 } else { digito };
            if valor > 9 {
                valor - 9
            } else {
                valor
            }
        })
        .sum();
    soma.is_multiple_of(10)
}

fn imei_contextualizado(texto: &str, mut inicio: usize) -> Option<(usize, usize)> {
    let bytes = texto.as_bytes();
    // Rótulos simples ou chaves JSON entre aspas; nunca procurar um rótulo
    // distante nem transformar um identificador que apenas contenha "IMEI".
    while bytes
        .get(inicio)
        .is_some_and(|byte| matches!(byte, b'(' | b'[' | b'{' | b'<' | b'"' | b'\''))
    {
        inicio += 1;
    }
    if !texto.get(inicio..inicio + 4)?.eq_ignore_ascii_case("imei") {
        return None;
    }
    let mut posicao = inicio + 4;
    if bytes
        .get(posicao)
        .is_some_and(|byte| matches!(byte, b'1' | b'2'))
    {
        posicao += 1;
    }
    if bytes
        .get(posicao)
        .is_some_and(|byte| matches!(byte, b'"' | b'\''))
    {
        posicao += 1;
    }
    let mut separador = false;
    for _ in 0..8 {
        if bytes
            .get(posicao)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            posicao += 1;
            separador = true;
        } else {
            break;
        }
    }
    if bytes
        .get(posicao)
        .is_some_and(|byte| matches!(byte, b':' | b'='))
    {
        posicao += 1;
        separador = true;
        for _ in 0..8 {
            if bytes
                .get(posicao)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            {
                posicao += 1;
            } else {
                break;
            }
        }
    }
    if !separador {
        return None;
    }
    if bytes
        .get(posicao)
        .is_some_and(|byte| matches!(byte, b'"' | b'\''))
    {
        posicao += 1;
    }
    let fim = posicao.checked_add(15)?;
    let literal = texto.get(posicao..fim)?;
    if !imei_luhn_valido(literal)
        || texto[fim..].chars().next().is_some_and(|caractere| {
            caractere.is_alphanumeric() || matches!(caractere, '_' | '-' | '+')
        })
        || (bytes.get(fim) == Some(&b'.') && bytes.get(fim + 1).is_some_and(u8::is_ascii_digit))
    {
        return None;
    }
    Some((posicao, fim))
}

/// Retorna `true` somente quando há uma ocorrência adicional omitida.
fn publicar(
    resultado: &mut AnaliseEntidades,
    texto: &str,
    inicio: usize,
    fim: usize,
    tipo: &str,
    regra: &str,
) -> bool {
    if resultado.candidatos.len() == MAX_CANDIDATOS {
        resultado.limite_atingido = true;
        return true;
    }
    resultado.candidatos.push(Candidato {
        tipo: tipo.to_owned(),
        literal: texto[inicio..fim].to_owned(),
        // A entrada foi limitada a 2 MiB, muito abaixo de u32::MAX.
        inicio: inicio as u32,
        fim: fim as u32,
        regra: regra.to_owned(),
    });
    false
}

fn aparar_pontuacao(texto: &str, mut inicio: usize, mut fim: usize) -> (usize, usize) {
    while inicio < fim
        && matches!(
            texto.as_bytes()[inicio],
            b'(' | b'[' | b'{' | b'<' | b'"' | b'\'' | b',' | b';' | b'!'
        )
    {
        inicio += 1;
    }
    let url_com_ipv6 = prefixo_http(&texto[inicio..fim])
        .is_some_and(|tamanho| texto[inicio + tamanho..fim].starts_with('['));
    while inicio < fim
        && (matches!(
            texto.as_bytes()[fim - 1],
            b')' | b'}' | b'>' | b'"' | b'\'' | b',' | b';' | b'!' | b'.'
        ) || (texto.as_bytes()[fim - 1] == b']' && !url_com_ipv6))
    {
        fim -= 1;
    }
    (inicio, fim)
}

fn dominio_ascii(dominio: &str) -> bool {
    if !(3..=253).contains(&dominio.len()) || !dominio.is_ascii() {
        return false;
    }
    let Some((_, sufixo)) = dominio.rsplit_once('.') else {
        return false;
    };
    // Sem tabela de sufixos públicos ou DNS: isto é somente sintaxe. TLD
    // numérico não é aceito; nomes internacionais exigem a forma ASCII IDNA.
    if sufixo.len() < 2
        || !(sufixo.bytes().all(|byte| byte.is_ascii_alphabetic())
            || sufixo.to_ascii_lowercase().starts_with("xn--"))
    {
        return false;
    }
    dominio.split('.').all(|rotulo| {
        !rotulo.is_empty()
            && rotulo.len() <= 63
            && rotulo.as_bytes()[0].is_ascii_alphanumeric()
            && rotulo.as_bytes()[rotulo.len() - 1].is_ascii_alphanumeric()
            && rotulo
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn parece_arquivo(dominio: &str) -> bool {
    if dominio
        .get(..4)
        .is_some_and(|prefixo| prefixo.eq_ignore_ascii_case("www."))
    {
        return false;
    }
    let sufixo = dominio.rsplit('.').next().unwrap_or_default();
    [
        "txt", "pdf", "doc", "docx", "csv", "json", "xml", "jpg", "jpeg", "png", "gif", "zip",
        "rar", "db", "sqlite", "log", "html", "htm", "xlsx", "exe", "dll",
    ]
    .iter()
    .any(|extensao| sufixo.eq_ignore_ascii_case(extensao))
}

fn email_ascii(literal: &str) -> bool {
    if literal.len() > 254 {
        return false;
    }
    let Some((local, dominio)) = literal.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
        && dominio_ascii(dominio)
}

fn prefixo_http(literal: &str) -> Option<usize> {
    if literal
        .get(..8)
        .is_some_and(|prefixo| prefixo.eq_ignore_ascii_case("https://"))
    {
        Some(8)
    } else if literal
        .get(..7)
        .is_some_and(|prefixo| prefixo.eq_ignore_ascii_case("http://"))
    {
        Some(7)
    } else {
        None
    }
}

fn url_http(literal: &str) -> bool {
    let Some(prefixo) = prefixo_http(literal) else {
        return false;
    };
    if literal.chars().any(|caractere| {
        caractere.is_control()
            || caractere.is_whitespace()
            || matches!(caractere, '\\' | '<' | '>' | '"' | '\'')
    }) {
        return false;
    }
    let resto = &literal[prefixo..];
    let fim_autoridade = resto.find(['/', '?', '#']).unwrap_or(resto.len());
    let autoridade = &resto[..fim_autoridade];
    // Credenciais e host vazio não entram neste perfil. Nenhuma URL é visitada.
    if autoridade.is_empty() || autoridade.contains('@') {
        return false;
    }
    let host_valido = if let Some(ipv6_porta) = autoridade.strip_prefix('[') {
        let Some((ip, porta)) = ipv6_porta.split_once(']') else {
            return false;
        };
        ip.parse::<Ipv6Addr>().is_ok()
            && (porta.is_empty() || porta.strip_prefix(':').is_some_and(porta_valida))
    } else {
        let (host, porta) = autoridade
            .split_once(':')
            .map_or((autoridade, None), |(host, porta)| (host, Some(porta)));
        (dominio_ascii(host) || host.parse::<std::net::Ipv4Addr>().is_ok())
            && porta.is_none_or(porta_valida)
    };
    if !host_valido {
        return false;
    }
    let bytes = literal.as_bytes();
    let mut indice = prefixo + fim_autoridade;
    while indice < bytes.len() {
        if bytes[indice] == b'%' {
            if indice + 2 >= bytes.len()
                || !bytes[indice + 1].is_ascii_hexdigit()
                || !bytes[indice + 2].is_ascii_hexdigit()
            {
                return false;
            }
            indice += 3;
        } else {
            indice += 1;
        }
    }
    true
}

fn porta_valida(porta: &str) -> bool {
    !porta.is_empty()
        && porta.bytes().all(|byte| byte.is_ascii_digit())
        && porta.parse::<u16>().is_ok_and(|numero| numero > 0)
}

fn telefone_internacional(literal: &str) -> bool {
    let Some(digitos) = literal.strip_prefix('+') else {
        return false;
    };
    (10..=15).contains(&digitos.len())
        && digitos.as_bytes()[0] != b'0'
        && digitos.bytes().all(|byte| byte.is_ascii_digit())
        && !digitos_repetidos(&digitos.as_bytes()[digitos.len() - 7..])
}

fn digitos_repetidos(digitos: &[u8]) -> bool {
    digitos
        .first()
        .is_some_and(|primeiro| digitos.iter().all(|digito| digito == primeiro))
}

fn pular_abertura_telefone(texto: &str, mut inicio: usize, fim: usize) -> usize {
    while inicio < fim && matches!(texto.as_bytes()[inicio], b'[' | b'{' | b'<' | b'"' | b'\'') {
        inicio += 1;
    }
    inicio
}

fn pular_espacos(bytes: &[u8], mut posicao: usize) -> usize {
    // Sem atravessar linhas ou aceitar quantidades ilimitadas de espaços.
    for _ in 0..3 {
        if bytes
            .get(posicao)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
        {
            posicao += 1;
        } else {
            break;
        }
    }
    posicao
}

fn telefone_formatado(texto: &str, inicio: usize) -> Option<usize> {
    let bytes = texto.as_bytes();
    let mut posicao = inicio;
    if texto[inicio..].starts_with("+55") {
        posicao = pular_espacos(bytes, inicio + 3);
    }
    if bytes.get(posicao) != Some(&b'(')
        || !bytes.get(posicao + 1)?.is_ascii_digit()
        || bytes[posicao + 1] == b'0'
        || !bytes.get(posicao + 2)?.is_ascii_digit()
        || bytes.get(posicao + 3) != Some(&b')')
    {
        return None;
    }
    posicao = pular_espacos(bytes, posicao + 4);
    let inicio_assinante = posicao;
    while bytes.get(posicao).is_some_and(u8::is_ascii_digit) {
        posicao += 1;
        if posicao - inicio_assinante > 5 {
            return None;
        }
    }
    let quantidade = posicao - inicio_assinante;
    if !((quantidade == 5 && bytes[inicio_assinante] == b'9')
        || (quantidade == 4 && (b'2'..=b'5').contains(&bytes[inicio_assinante])))
        || bytes.get(posicao) != Some(&b'-')
    {
        return None;
    }
    posicao += 1;
    let final_assinante = posicao;
    for _ in 0..4 {
        if !bytes.get(posicao)?.is_ascii_digit() {
            return None;
        }
        posicao += 1;
    }
    if posicao - inicio > MAX_TOKEN_BYTES
        || texto[posicao..].chars().next().is_some_and(|caractere| {
            caractere.is_alphanumeric() || matches!(caractere, '_' | '-' | '+')
        })
    {
        return None;
    }
    let primeiro = bytes[inicio_assinante];
    if bytes[inicio_assinante..final_assinante - 1]
        .iter()
        .chain(bytes[final_assinante..posicao].iter())
        .all(|digito| *digito == primeiro)
    {
        return None;
    }
    Some(posicao)
}
