//! Leitura derivada de uma página de PDF nativo, depois da conferência da origem.
//!
//! O parser é `lopdf`, não um reconhecedor artesanal de bytes de PDF. O envelope
//! inicial é deliberadamente explícito: páginas com fontes Type1 Latin Base14 e
//! codificações nomeadas conhecidas, sem CMap/ToUnicode, Form XObject ou objetos
//! comprimidos. Não há renderização, OCR, senha, JavaScript, rede ou gravação.
//!
//! A carga limita a expansão de cada xref stream; ObjStm é descartado pelo filtro
//! ANTES de sua descompressão e qualquer referência comprimida recusa o documento.
//! Conteúdo de página tem orçamento agregado de 256 KiB, incluindo separadores.
//! Fontes admitidas não decodificam streams de fontes. Os limites de objetos e
//! páginas são verificados depois da carga. Não constituem quota de RSS/processo
//! nem timeout preemptivo do parser: documentos não confiáveis ainda precisam de
//! isolamento de processo para uma garantia rígida de disponibilidade.

use super::{ControleDocumento, EstadoDocumento, ResultadoDocumento, TAMANHO_MAXIMO};
use flate2::{Decompress, FlushDecompress, Status};
use lopdf::content::{Content, Operation};
use lopdf::{
    DecompressError, Dictionary, Document, Encoding, Error, LoadOptions, Object, ObjectId,
};
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

pub(super) const MOTOR_ID: &str = "pdf_nativo_lopdf";
pub(super) const MOTOR_VERSAO: &str = "0.45.0/sied-1";
pub(super) const MAXIMO_PAGINAS: u32 = 512;

const MAXIMO_OBJETOS: usize = 4096;
const MAXIMO_FONTES: usize = 64;
const MAXIMO_PROFUNDIDADE: usize = 64;
const MAXIMO_STREAM_CARGA: usize = 64 * 1024;
const MAXIMO_CONTEUDO_PAGINA: usize = 256 * 1024;

/// Recebe somente os bytes cuja identidade, tamanho e SHA-256 já foram conferidos.
/// Nenhuma saída parcial é retornada. `pagina` é um índice real, começando em 1.
pub(super) fn extrair(
    bytes: &[u8],
    pagina: u32,
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<(String, u32), Box<ResultadoDocumento>> {
    verificar_controle(controle, inicio)?;
    if pagina == 0 || pagina > MAXIMO_PAGINAS {
        return Err(falhou("A página solicitada do PDF é inválida."));
    }
    if bytes.len() as u64 > TAMANHO_MAXIMO {
        return Err(limite("O PDF excede o limite de 2 MiB para leitura."));
    }
    if !bytes.starts_with(b"%PDF-") {
        return Err(falhou("O conteúdo não possui um cabeçalho PDF válido."));
    }

    let documento = Document::load_mem_with_options(
        bytes,
        LoadOptions {
            strict: true,
            max_decompressed_size: Some(MAXIMO_STREAM_CARGA),
            filter: Some(ignorar_objetos_comprimidos),
            ..LoadOptions::default()
        },
    )
    .map_err(erro_motor)?;
    verificar_controle(controle, inicio)?;

    if !matches!(
        documento.reference_table.cross_reference_type,
        lopdf::xref::XrefType::CrossReferenceTable
    ) {
        return Err(sem_suporte(
            "O PDF utiliza xref streams, ainda sem validação estrita neste motor. São admitidas tabelas xref clássicas.",
        ));
    }
    for objeto in documento.objects.values() {
        if let Object::Stream(stream) = objeto {
            validar_filtros(&stream.dict)?;
        }
    }

    if documento.is_encrypted() || documento.encryption_state.is_some() {
        return Err(sem_suporte(
            "PDF protegido ou criptografado não é admitido neste motor. Nenhuma senha é solicitada.",
        ));
    }
    if documento
        .reference_table
        .entries
        .values()
        .any(lopdf::xref::XrefEntry::is_compressed)
    {
        return Err(sem_suporte(
            "O PDF utiliza objetos comprimidos (ObjStm), ainda sem suporte neste motor.",
        ));
    }
    if documento.objects.len() > MAXIMO_OBJETOS
        || documento.reference_table.entries.len() > MAXIMO_OBJETOS
    {
        return Err(limite("O PDF excede o limite de 4096 objetos."));
    }

    let catalogo_id = documento
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(erro_motor)?;
    let catalogo = documento.get_dictionary(catalogo_id).map_err(erro_motor)?;
    if catalogo.get(b"Type").and_then(Object::as_name).ok() != Some(b"Catalog".as_slice()) {
        return Err(falhou("O catálogo do PDF é inválido."));
    }
    let raiz_id = catalogo
        .get(b"Pages")
        .and_then(Object::as_reference)
        .map_err(erro_motor)?;
    let mut paginas = Vec::new();
    validar_arvore_paginas(
        &documento,
        raiz_id,
        None,
        0,
        &mut HashSet::new(),
        &mut paginas,
        controle,
        inicio,
    )?;
    verificar_controle(controle, inicio)?;
    if paginas.is_empty() {
        return Err(falhou("O PDF não possui uma árvore de páginas válida."));
    }
    let total = paginas.len() as u32;
    let Some(&pagina_id) = paginas.get((pagina - 1) as usize) else {
        return Err(falhou("A página solicitada não existe neste PDF."));
    };
    // A árvore validada e a enumeração oficial do motor devem concordar. Não
    // publicamos contagens provenientes apenas de /Count ou de nós ignorados.
    let enumeradas = documento.get_pages();
    if enumeradas.len() != paginas.len()
        || paginas
            .iter()
            .enumerate()
            .any(|(indice, id)| enumeradas.get(&(indice as u32 + 1)) != Some(id))
    {
        return Err(falhou("A enumeração de páginas do PDF é inconsistente."));
    }

    let fontes = fontes_da_pagina(&documento, pagina_id)?;
    let conteudo = conteudo_estrito(&documento, pagina_id, controle, inicio)?;
    let conteudo_motor = documento
        .get_page_content_with_limit(pagina_id, MAXIMO_CONTEUDO_PAGINA)
        .map_err(erro_motor)?;
    if conteudo != conteudo_motor {
        return Err(falhou(
            "A resolução do conteúdo da página do PDF é inconsistente.",
        ));
    }
    let operacoes = Content::decode_strict(&conteudo).map_err(erro_motor)?;
    verificar_controle(controle, inicio)?;
    validar_operacoes(
        &documento,
        pagina_id,
        &operacoes.operations,
        &fontes,
        controle,
        inicio,
    )?;
    verificar_controle(controle, inicio)?;

    // Esta é a extração da biblioteca publicada. A validação anterior impede
    // seus fallbacks lenientes de publicar dados de filtros/fontes inválidos.
    let texto = documento
        .extract_text_with_limit(&[pagina], MAXIMO_CONTEUDO_PAGINA)
        .map_err(erro_motor)?;
    verificar_controle(controle, inicio)?;
    if texto.len() as u64 > TAMANHO_MAXIMO {
        return Err(limite(
            "O texto derivado da página excede o limite de 2 MiB.",
        ));
    }
    Ok((texto, total))
}

fn ignorar_objetos_comprimidos(id: ObjectId, objeto: &mut Object) -> Option<(ObjectId, Object)> {
    if matches!(objeto, Object::Stream(stream) if stream.dict.has_type(b"ObjStm")) {
        // O carregador chama o filtro antes de ObjectStream::new_with_limit.
        // Não existe expansão de ObjStm, nem mutação/gravação da evidência.
        None
    } else {
        Some((id, objeto.clone()))
    }
}

#[allow(clippy::too_many_arguments)]
fn validar_arvore_paginas(
    documento: &Document,
    id: ObjectId,
    pai: Option<ObjectId>,
    profundidade: usize,
    vistos: &mut HashSet<ObjectId>,
    paginas: &mut Vec<ObjectId>,
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<(), Box<ResultadoDocumento>> {
    verificar_controle(controle, inicio)?;
    if profundidade > MAXIMO_PROFUNDIDADE {
        return Err(limite(
            "A árvore de páginas do PDF excede o limite de profundidade.",
        ));
    }
    if !vistos.insert(id) {
        return Err(falhou(
            "A árvore de páginas do PDF contém um ciclo ou uma página repetida.",
        ));
    }
    let dicionario = documento.get_dictionary(id).map_err(erro_motor)?;
    match pai {
        Some(pai)
            if dicionario
                .get(b"Parent")
                .and_then(Object::as_reference)
                .ok()
                != Some(pai) =>
        {
            return Err(falhou("A hierarquia de páginas do PDF é inconsistente."));
        }
        None if dicionario.has(b"Parent") => {
            return Err(falhou(
                "A raiz de páginas do PDF possui uma hierarquia inválida.",
            ));
        }
        _ => {}
    }
    validar_recurso(documento, dicionario)?;
    match dicionario
        .get(b"Type")
        .and_then(Object::as_name)
        .map_err(erro_motor)?
    {
        b"Page" if pai.is_some() => {
            if paginas.len() >= MAXIMO_PAGINAS as usize {
                return Err(limite("O PDF excede o limite de 512 páginas."));
            }
            paginas.push(id);
        }
        b"Pages" => {
            let antes = paginas.len();
            let filhos = dicionario
                .get(b"Kids")
                .and_then(Object::as_array)
                .map_err(erro_motor)?;
            for filho in filhos {
                let filho_id = filho.as_reference().map_err(erro_motor)?;
                validar_arvore_paginas(
                    documento,
                    filho_id,
                    Some(id),
                    profundidade + 1,
                    vistos,
                    paginas,
                    controle,
                    inicio,
                )?;
            }
            let quantidade = dicionario
                .get(b"Count")
                .and_then(Object::as_i64)
                .map_err(erro_motor)?;
            if quantidade < 0 || quantidade as usize != paginas.len() - antes {
                return Err(falhou(
                    "A contagem declarada de páginas do PDF não corresponde à árvore real.",
                ));
            }
        }
        _ => return Err(falhou("A árvore de páginas do PDF contém um nó inválido.")),
    }
    Ok(())
}

fn validar_recurso(
    documento: &Document,
    dicionario: &Dictionary,
) -> Result<(), Box<ResultadoDocumento>> {
    if let Ok(recurso) = dicionario.get(b"Resources") {
        resolver_dicionario(documento, recurso)?;
    }
    Ok(())
}

fn resolver_dicionario<'a>(
    documento: &'a Document,
    objeto: &'a Object,
) -> Result<&'a Dictionary, Box<ResultadoDocumento>> {
    match objeto {
        Object::Dictionary(dicionario) => Ok(dicionario),
        Object::Reference(id) => documento.get_dictionary(*id).map_err(erro_motor),
        _ => Err(falhou("O PDF possui um recurso com estrutura inválida.")),
    }
}

fn recursos_da_pagina(
    documento: &Document,
    pagina_id: ObjectId,
) -> Result<Vec<&Dictionary>, Box<ResultadoDocumento>> {
    let (direto, ids) = documento
        .get_page_resources(pagina_id)
        .map_err(erro_motor)?;
    let mut recursos = Vec::new();
    if let Some(direto) = direto {
        recursos.push(direto);
    }
    for id in ids {
        recursos.push(documento.get_dictionary(id).map_err(erro_motor)?);
    }
    Ok(recursos)
}

fn fontes_da_pagina(
    documento: &Document,
    pagina_id: ObjectId,
) -> Result<BTreeMap<Vec<u8>, &Dictionary>, Box<ResultadoDocumento>> {
    let mut fontes = BTreeMap::new();
    for recurso in recursos_da_pagina(documento, pagina_id)? {
        if let Ok(objeto) = recurso.get(b"Font") {
            let dicionario = resolver_dicionario(documento, objeto)?;
            for (nome, objeto) in dicionario.iter() {
                let fonte = resolver_dicionario(documento, objeto)?;
                // Coincide com a resolução oficial: a fonte mais próxima vence.
                let limite_atingido = fontes.len() >= MAXIMO_FONTES;
                if let std::collections::btree_map::Entry::Vacant(entrada) =
                    fontes.entry(nome.clone())
                {
                    validar_fonte(fonte)?;
                    if limite_atingido {
                        return Err(limite("A página do PDF excede o limite de 64 fontes."));
                    }
                    entrada.insert(fonte);
                }
            }
        }
    }
    Ok(fontes)
}

fn validar_fonte(fonte: &Dictionary) -> Result<(), Box<ResultadoDocumento>> {
    let base = fonte.get(b"BaseFont").and_then(Object::as_name).ok();
    let base_admitida = matches!(
        base,
        Some(
            b"Courier"
                | b"Courier-Bold"
                | b"Courier-Oblique"
                | b"Courier-BoldOblique"
                | b"Helvetica"
                | b"Helvetica-Bold"
                | b"Helvetica-Oblique"
                | b"Helvetica-BoldOblique"
                | b"Times-Roman"
                | b"Times-Bold"
                | b"Times-Italic"
                | b"Times-BoldItalic"
        )
    );
    let codificacao = match fonte.get(b"Encoding") {
        Ok(Object::Name(nome)) => matches!(
            nome.as_slice(),
            b"StandardEncoding" | b"WinAnsiEncoding" | b"MacRomanEncoding"
        ),
        Err(Error::DictKey(_)) => true, // Type1 Base14 Latin: StandardEncoding da especificação.
        _ => false,
    };
    if fonte.get(b"Type").and_then(Object::as_name).ok() != Some(b"Font".as_slice())
        || fonte.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Type1".as_slice())
        || !base_admitida
        || !codificacao
        || fonte.has(b"ToUnicode")
        || fonte.has(b"FontDescriptor")
    {
        return Err(sem_suporte(
            "A página usa fontes ou mapeamentos ainda sem suporte. Este motor admite Type1 Latin Base14 com StandardEncoding, WinAnsiEncoding ou MacRomanEncoding, sem ToUnicode.",
        ));
    }
    Ok(())
}

fn validar_filtros(dicionario: &Dictionary) -> Result<(), Box<ResultadoDocumento>> {
    match dicionario.get(b"Filter") {
        Err(Error::DictKey(_)) => {}
        Ok(Object::Name(_)) => {}
        Ok(Object::Array(filtros))
            if !filtros.is_empty()
                && filtros.len() <= 4
                && filtros
                    .iter()
                    .all(|filtro| matches!(filtro, Object::Name(_))) => {}
        Ok(Object::Array(filtros)) if filtros.len() > 4 => {
            return Err(limite(
                "O PDF excede o limite de quatro filtros por stream.",
            ));
        }
        _ => return Err(falhou("O PDF possui uma declaração de filtros inválida.")),
    }
    // Preditores/parametrizações ainda não têm envelope estrito definido aqui.
    // Evita reinterpretar parâmetros inválidos como ausência de parâmetros.
    if dicionario.has(b"DecodeParms") {
        return Err(sem_suporte(
            "O PDF utiliza parâmetros de decodificação de streams ainda sem suporte.",
        ));
    }
    Ok(())
}

fn conteudo_estrito(
    documento: &Document,
    pagina_id: ObjectId,
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<Vec<u8>, Box<ResultadoDocumento>> {
    let pagina = documento.get_dictionary(pagina_id).map_err(erro_motor)?;
    let mut conteudo = Vec::new();
    let mut ids = Vec::new();
    match pagina.get(b"Contents") {
        Ok(Object::Reference(id)) => match documento.get_object(*id).map_err(erro_motor)? {
            Object::Stream(_) => ids.push(*id),
            Object::Array(objetos) => {
                for objeto in objetos {
                    ids.push(objeto.as_reference().map_err(erro_motor)?);
                }
            }
            _ => return Err(falhou("O conteúdo da página do PDF é inválido.")),
        },
        Ok(Object::Array(objetos)) => {
            for objeto in objetos {
                ids.push(objeto.as_reference().map_err(erro_motor)?);
            }
        }
        Err(Error::DictKey(_)) => return Ok(conteudo),
        _ => return Err(falhou("O conteúdo da página do PDF é inválido.")),
    }
    for id in ids {
        verificar_controle(controle, inicio)?;
        let restante = MAXIMO_CONTEUDO_PAGINA.saturating_sub(conteudo.len());
        if restante == 0 {
            return Err(limite("O conteúdo descomprimido da página excede 256 KiB."));
        }
        let stream = documento
            .get_object(id)
            .and_then(Object::as_stream)
            .map_err(erro_motor)?;
        let bytes = descomprimir_conteudo_estrito(stream, restante - 1)?;
        verificar_controle(controle, inicio)?;
        conteudo.extend_from_slice(&bytes);
        conteudo.push(b'\n');
    }
    Ok(conteudo)
}

fn descomprimir_conteudo_estrito(
    stream: &lopdf::Stream,
    limite_bytes: usize,
) -> Result<Vec<u8>, Box<ResultadoDocumento>> {
    validar_filtros(&stream.dict)?;
    let filtro = match stream.dict.get(b"Filter") {
        Err(Error::DictKey(_)) => None,
        Ok(Object::Name(nome)) => Some(nome.as_slice()),
        Ok(Object::Array(filtros)) if filtros.len() == 1 => {
            Some(filtros[0].as_name().map_err(erro_motor)?)
        }
        Ok(Object::Array(_)) => {
            return Err(sem_suporte(
                "O conteúdo da página usa encadeamento de filtros, ainda sem suporte estrito.",
            ))
        }
        _ => return Err(falhou("O PDF possui uma declaração de filtros inválida.")),
    };
    match filtro {
        None => {
            if stream.content.len() > limite_bytes {
                return Err(limite("O conteúdo descomprimido da página excede 256 KiB."));
            }
            Ok(stream.content.clone())
        }
        Some(b"FlateDecode") => {
            // Codec publicado, não parser PDF: exige StreamEnd/checksum válidos
            // e consumo integral, sem o raw-deflate/output-parcial fallback do
            // lopdf. Buffer fixo inclui só um sentinela para detectar o excesso.
            let mut decoder = Decompress::new(true);
            let mut bytes = vec![0_u8; limite_bytes + 1];
            let status = decoder.decompress(&stream.content, &mut bytes, FlushDecompress::Finish);
            if decoder.total_out() > limite_bytes as u64 {
                return Err(limite("O conteúdo descomprimido da página excede 256 KiB."));
            }
            let status = status.map_err(|_| falhou(
                "O stream FlateDecode do PDF é inválido. Nenhum texto parcial foi publicado.",
            ))?;
            if status != Status::StreamEnd || decoder.total_in() != stream.content.len() as u64 {
                return Err(falhou("O stream FlateDecode do PDF é incompleto ou inconsistente. Nenhum texto parcial foi publicado."));
            }
            bytes.truncate(decoder.total_out() as usize);
            Ok(bytes)
        }
        Some(_) => Err(sem_suporte(
            "O conteúdo da página usa um filtro ainda sem suporte estrito. São admitidos somente conteúdo sem filtro ou FlateDecode.",
        )),
    }
}

fn validar_operacoes(
    documento: &Document,
    pagina_id: ObjectId,
    operacoes: &[Operation],
    fontes: &BTreeMap<Vec<u8>, &Dictionary>,
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<(), Box<ResultadoDocumento>> {
    let mut fonte_ativa: Option<&[u8]> = None;
    let mut fonte_motor: Option<&[u8]> = None;
    let mut estados_graficos = Vec::new();
    for operacao in operacoes {
        verificar_controle(controle, inicio)?;
        match operacao.operator.as_str() {
            "BI" => {
                // O parser de conteúdo também recupera inline images lenientes
                // em decode_strict. Recusar impede que um falso delimitador EI
                // faça bytes opacos serem interpretados como texto documental.
                return Err(sem_suporte(
                    "A página contém imagem inline, ainda sem suporte neste motor. Nenhum texto parcial foi publicado.",
                ));
            }
            "q" => {
                if estados_graficos.len() >= MAXIMO_PROFUNDIDADE {
                    return Err(limite("A página excede o limite de estados gráficos."));
                }
                estados_graficos.push(fonte_ativa);
            }
            "Q" => {
                fonte_ativa = estados_graficos.pop().ok_or_else(|| {
                    falhou("A página possui uma pilha de estados gráficos inválida.")
                })?;
            }
            "Tf" => {
                let nome = operacao
                    .operands
                    .first()
                    .and_then(|objeto| objeto.as_name().ok());
                if nome.is_none_or(|nome| !fontes.contains_key(nome))
                    || operacao.operands.len() != 2
                    || !numero(&operacao.operands[1])
                {
                    return Err(falhou(
                        "A página referencia uma fonte inexistente ou inválida.",
                    ));
                }
                fonte_ativa = nome;
                fonte_motor = nome;
            }
            "Tj" | "TJ" | "'" | "\"" => {
                let nome = fonte_ativa.ok_or_else(|| {
                    falhou("A página possui texto sem uma fonte válida selecionada.")
                })?;
                if fonte_ativa != fonte_motor {
                    return Err(sem_suporte(
                        "A página restaura fontes por estado gráfico ainda sem suporte. Nenhum texto parcial foi publicado.",
                    ));
                }
                let fonte = fontes.get(nome).ok_or_else(|| {
                    falhou("A página referencia uma fonte inexistente ou inválida.")
                })?;
                let encoding = fonte
                    .get_font_encoding_with_limit(documento, MAXIMO_STREAM_CARGA)
                    .map_err(erro_motor)?;
                validar_texto(operacao, &encoding)?;
            }
            "gs" => validar_estado_grafico(documento, pagina_id, operacao)?,
            "Do" => {
                let nome = operacao
                    .operands
                    .first()
                    .and_then(|objeto| objeto.as_name().ok())
                    .ok_or_else(|| falhou("A página referencia um objeto gráfico inválido."))?;
                let mut encontrado = None;
                for recurso in recursos_da_pagina(documento, pagina_id)? {
                    if let Ok(xobject) = recurso.get(b"XObject") {
                        let objetos = resolver_dicionario(documento, xobject)?;
                        if let Ok(objeto) = objetos.get(nome) {
                            encontrado = Some(objeto);
                            break;
                        }
                    }
                }
                let objeto = encontrado
                    .ok_or_else(|| falhou("A página referencia um objeto gráfico inexistente."))?;
                let stream = match objeto {
                    Object::Reference(id) => documento
                        .get_object(*id)
                        .and_then(Object::as_stream)
                        .map_err(erro_motor)?,
                    Object::Stream(stream) => stream,
                    _ => return Err(falhou("O objeto gráfico da página é inválido.")),
                };
                if stream.dict.get(b"Subtype").and_then(Object::as_name).ok()
                    != Some(b"Image".as_slice())
                {
                    return Err(sem_suporte(
                        "A página usa Form XObject ou outro conteúdo gráfico ainda sem suporte. Nenhum texto parcial foi publicado.",
                    ));
                }
            }
            _ => {}
        }
    }
    if !estados_graficos.is_empty() {
        return Err(falhou("A página possui estados gráficos não encerrados."));
    }
    Ok(())
}

fn numero(objeto: &Object) -> bool {
    matches!(objeto, Object::Integer(_))
        || matches!(objeto, Object::Real(valor) if valor.is_finite())
}

fn validar_texto(
    operacao: &Operation,
    encoding: &Encoding<'_>,
) -> Result<(), Box<ResultadoDocumento>> {
    let mut strings = Vec::new();
    match operacao.operator.as_str() {
        "Tj" | "'" if operacao.operands.len() == 1 => {
            strings.push(operacao.operands[0].as_str().map_err(erro_motor)?);
        }
        "TJ" if operacao.operands.len() == 1 => {
            for objeto in operacao.operands[0].as_array().map_err(erro_motor)? {
                match objeto {
                    Object::String(bytes, _) => strings.push(bytes.as_slice()),
                    Object::Integer(_) | Object::Real(_) => {}
                    _ => return Err(falhou("A página possui uma operação de texto inválida.")),
                }
            }
        }
        "\"" if operacao.operands.len() == 3
            && numero(&operacao.operands[0])
            && numero(&operacao.operands[1]) =>
        {
            strings.push(operacao.operands[2].as_str().map_err(erro_motor)?);
        }
        _ => return Err(falhou("A página possui uma operação de texto inválida.")),
    }
    // A API OneByteEncoding ignora silenciosamente códigos não mapeados. Eles
    // são recusados antes da extração, sem substituição/perda de caracteres.
    let Encoding::OneByteEncoding(mapa) = encoding else {
        return Err(sem_suporte(
            "A codificação da fonte não pertence ao envelope admitido.",
        ));
    };
    for bytes in strings {
        for byte in bytes {
            let Some(glifo) = mapa.get(*byte as usize).copied().flatten() else {
                return Err(sem_suporte(
                    "A página contém códigos de caracteres sem mapeamento. Nenhum texto com perdas foi publicado.",
                ));
            };
            if char::decode_utf16([glifo.utf16_code_unit()]).any(|caractere| caractere.is_err()) {
                return Err(sem_suporte(
                    "O mapeamento da fonte contém um caractere inválido.",
                ));
            }
        }
    }
    Ok(())
}

fn validar_estado_grafico(
    documento: &Document,
    pagina_id: ObjectId,
    operacao: &Operation,
) -> Result<(), Box<ResultadoDocumento>> {
    let nome = operacao
        .operands
        .first()
        .and_then(|objeto| objeto.as_name().ok())
        .ok_or_else(|| falhou("A página referencia um estado gráfico inválido."))?;
    for recurso in recursos_da_pagina(documento, pagina_id)? {
        if let Ok(objeto) = recurso.get(b"ExtGState") {
            let estados = resolver_dicionario(documento, objeto)?;
            if let Ok(objeto) = estados.get(nome) {
                let estado = resolver_dicionario(documento, objeto)?;
                if estado.has(b"Font") {
                    return Err(sem_suporte(
                        "A página seleciona fontes por ExtGState, ainda sem suporte. Nenhum texto parcial foi publicado.",
                    ));
                }
                return Ok(());
            }
        }
    }
    Err(falhou("A página referencia um estado gráfico inexistente."))
}

fn verificar_controle(
    controle: &ControleDocumento,
    inicio: Instant,
) -> Result<(), Box<ResultadoDocumento>> {
    match controle.interrupcao(inicio) {
        Some(resultado) => Err(Box::new(resultado)),
        None => Ok(()),
    }
}

fn erro_motor(erro: Error) -> Box<ResultadoDocumento> {
    match erro {
        Error::Decompress(DecompressError::MemoryLimitExceeded { .. }) => {
            limite("O PDF excede o limite de descompressão permitido pelo motor.")
        }
        Error::RecursionLimit | Error::ReferenceLimit => {
            limite("O PDF excede o limite de referências ou profundidade permitido pelo motor.")
        }
        Error::Decryption(_) | Error::InvalidPassword | Error::UnsupportedSecurityHandler(_) => {
            sem_suporte("PDF protegido ou criptografado não é admitido neste motor.")
        }
        _ => falhou("O motor não conseguiu interpretar o PDF com segurança. Nenhum texto parcial foi publicado."),
    }
}

fn falhou(motivo: &str) -> Box<ResultadoDocumento> {
    Box::new(ResultadoDocumento::sem(EstadoDocumento::Falhou, motivo))
}

fn sem_suporte(motivo: &str) -> Box<ResultadoDocumento> {
    Box::new(ResultadoDocumento::sem(EstadoDocumento::SemSuporte, motivo))
}

fn limite(motivo: &str) -> Box<ResultadoDocumento> {
    Box::new(ResultadoDocumento::sem(
        EstadoDocumento::LimiteExcedido,
        motivo,
    ))
}
