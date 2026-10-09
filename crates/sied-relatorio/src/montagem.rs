//! Monta os dados do relatório a partir do banco do caso.
//!
//! Aqui acontece a conferência: o hash de cada evidência é comparado com o que foi
//! registrado quando ela foi selecionada, e os arquivos (e as imagens anexas) são
//! relidos na pasta da extração, somente leitura, com o SHA-256 conferido.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use sied_armazenamento::auditoria::VerificacaoAuditoria;
use sied_armazenamento::conversas::Conversa;
use sied_armazenamento::evidencias::{hash_do_trecho, Evidencia, TipoEvidencia};
use sied_armazenamento::extracoes::{
    ArtefatoDetalhe, ArtefatoResumo, ConferenciaRegistro, Extracao,
};
use sied_armazenamento::transcricoes::EstadoTranscricao;
use sied_armazenamento::{BancoCaso, Resultado};
use sied_dominio::{Dispositivo, SistemaOperacional};
use sied_midia::Pedido;

use crate::modelo::*;

/// Figuras no máximo por relatório (cada uma é uma imagem reduzida a 1600 px).
pub const MAX_FIGURAS: usize = 60;
/// Valores brutos maiores que isto são resumidos no apêndice (a íntegra fica no banco).
const MAX_VALOR_BRUTO: usize = 600;

pub const TITULO_PADRAO: &str = "Relatório de análise de evidências digitais";

/// Resultado da montagem: os dados, as imagens e a conferência de cada evidência.
pub struct Montado {
    pub documento: Documento,
    pub imagens: HashMap<String, Vec<u8>>,
    /// (id, número, tipo, hash registrado, situação) de cada evidência.
    pub conferencias: Vec<Conferencia>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Conferencia {
    pub evidencia_id: i64,
    pub numero: i64,
    pub tipo: TipoEvidencia,
    pub hash_no_momento: String,
    pub hash_conferido: Option<String>,
    pub situacao: String,
}

struct Contexto<'a> {
    banco: &'a BancoCaso,
    fuso: Tz,
    extracoes: HashMap<String, Extracao>,
    imagens: HashMap<String, Vec<u8>>,
    figuras_omitidas: usize,
}

pub fn fuso_do_caso(nome: &str) -> Tz {
    nome.parse().unwrap_or(chrono_tz::America::Sao_Paulo)
}

impl Contexto<'_> {
    fn quando(&self, t: DateTime<Utc>) -> String {
        self.fuso
            .from_utc_datetime(&t.naive_utc())
            .format("%d/%m/%Y %H:%M:%S")
            .to_string()
    }

    fn quando_texto(&self, rfc3339: &str) -> String {
        DateTime::parse_from_rfc3339(rfc3339)
            .map(|t| self.quando(t.with_timezone(&Utc)))
            .unwrap_or_else(|_| rfc3339.to_string())
    }

    fn quando_ms(&self, ms: Option<i64>) -> String {
        ms.and_then(|ms| Utc.timestamp_millis_opt(ms).single())
            .map(|t| self.quando(t))
            .unwrap_or_else(|| "sem data".into())
    }

    fn dia_hora(&self, ms: Option<i64>) -> (String, String) {
        match ms.and_then(|ms| Utc.timestamp_millis_opt(ms).single()) {
            Some(t) => {
                let l = self.fuso.from_utc_datetime(&t.naive_utc());
                (
                    l.format("%d/%m/%Y").to_string(),
                    l.format("%H:%M:%S").to_string(),
                )
            }
            None => ("Sem data".into(), "sem hora".into()),
        }
    }

    /// Figura verificada de um arquivo do inventário, ou o motivo de não haver.
    fn figura(&mut self, arquivo_id: i64, legenda: String) -> Result<FiguraDoc, String> {
        if self.imagens.len() >= MAX_FIGURAS {
            self.figuras_omitidas += 1;
            return Err(format!(
                "Imagem não incluída: limite de {MAX_FIGURAS} figuras por relatório."
            ));
        }
        let a = self
            .banco
            .arquivo_com_origem(arquivo_id)
            .map_err(|e| e.to_string())?;
        let pedido = pedido(self.banco, &a.arquivo, &a.origem_caminho);
        match sied_midia::imagem_para_relatorio(&pedido) {
            Ok(jpeg) => {
                let nome = format!("{}.jpg", self.imagens.len() + 1);
                self.imagens.insert(nome.clone(), jpeg);
                Ok(FiguraDoc {
                    arquivo: nome,
                    legenda,
                })
            }
            Err(m) => Err(m.motivo.unwrap_or_else(|| "Imagem indisponível.".into())),
        }
    }
}

fn pedido(
    banco: &BancoCaso,
    a: &sied_armazenamento::extracoes::ArquivoExtracao,
    origem: &str,
) -> Pedido {
    Pedido {
        pasta_extracao: PathBuf::from(origem),
        caminho: a.caminho.clone(),
        sha256: a.sha256.clone(),
        tamanho: a.tamanho.max(0) as u64,
        categoria: a.categoria.clone(),
        pasta_caso: banco.pasta().to_path_buf(),
    }
}

pub fn tamanho(n: i64) -> String {
    let unidades = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < unidades.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    let t = if i == 0 || v >= 100.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}").replace('.', ",")
    };
    format!("{t} {} ({} bytes)", unidades[i], milhar(n))
}

fn milhar(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn tempo_transcricao(ms: i64) -> String {
    let total = (ms.max(0) / 1000) as u64;
    let horas = total / 3600;
    let minutos = (total % 3600) / 60;
    let segundos = total % 60;
    if horas > 0 {
        format!("{horas}:{minutos:02}:{segundos:02}")
    } else {
        format!("{minutos}:{segundos:02}")
    }
}

fn estado_transcricao(estado: EstadoTranscricao) -> &'static str {
    match estado {
        EstadoTranscricao::Processando => "processando",
        EstadoTranscricao::Pronta => "pronta",
        EstadoTranscricao::SemSuporte => "sem_suporte",
        EstadoTranscricao::Cancelada => "cancelada",
        EstadoTranscricao::Erro => "erro",
    }
}

/// Insere pontos de quebra invisíveis em sequências longas sem espaço (base64,
/// hexadecimal), para que caibam na página. Só nos valores brutos do apêndice.
fn quebravel(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + t.len() / 40);
    let mut corrida = 0;
    for c in t.chars() {
        if c.is_whitespace() {
            corrida = 0;
        } else {
            corrida += 1;
            if corrida > 48 {
                out.push('\u{200B}');
                corrida = 1;
            }
        }
        out.push(c);
    }
    out
}

fn valor_bruto(v: &Value) -> String {
    let texto = match v {
        Value::Null => "NULL".to_string(),
        Value::String(s) => format!("\"{s}\""),
        Value::Object(o) if o.len() == 1 && o.contains_key("blob_hex") => {
            let hex = o["blob_hex"].as_str().unwrap_or("");
            let n = hex.len() / 2;
            if hex.len() <= 64 {
                format!("blob de {n} bytes: {hex}")
            } else {
                format!("blob de {n} bytes (início: {}…)", &hex[..64])
            }
        }
        outro => outro.to_string(),
    };
    let total = texto.chars().count();
    if total > MAX_VALOR_BRUTO {
        let inicio: String = texto.chars().take(MAX_VALOR_BRUTO).collect();
        format!(
            "{inicio}… [mais {} caracteres; íntegra no banco do caso]",
            total - MAX_VALOR_BRUTO
        )
    } else {
        texto
    }
}

fn valores_brutos(v: &Value) -> String {
    match v {
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| quebravel(&format!("{k} = {}", valor_bruto(v))))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        outro => quebravel(&valor_bruto(outro)),
    }
}

fn opt(campos: &mut Vec<Campo>, rotulo: &str, valor: Option<impl Into<String>>) {
    if let Some(v) = valor {
        let v = v.into();
        if !v.trim().is_empty() {
            campos.push(Campo::novo(rotulo, v));
        }
    }
}

fn nome_app(app: Option<&str>) -> String {
    match app {
        Some("sms") => "SMS".into(),
        Some("whatsapp") => "WhatsApp".into(),
        Some("whatsapp_business") => "WhatsApp Business".into(),
        Some("telegram") => "Telegram".into(),
        Some("telefone") => "Telefone".into(),
        Some("contatos_android") => "Agenda (Android)".into(),
        Some("iped") => "IPED (lista de itens)".into(),
        Some("aleapp") => "ALEAPP".into(),
        Some("ileapp") => "iLEAPP".into(),
        Some("xleapp") => "ALEAPP/iLEAPP".into(),
        Some(outro) => outro.into(),
        None => "Não informado".into(),
    }
}

fn nome_subtipo(s: &str) -> &str {
    match s {
        "texto" => "Texto",
        "imagem" => "Imagem",
        "audio" => "Áudio",
        "video" => "Vídeo",
        "documento" => "Documento",
        "gif" => "GIF",
        "figurinha" => "Figurinha",
        "contato" => "Contato",
        "contatos" => "Contatos",
        "localizacao" => "Localização",
        "localizacao_tempo_real" => "Localização em tempo real",
        "sistema" => "Mensagem do sistema",
        "apagada" => "Apagada",
        "desconhecido" => "Tipo desconhecido",
        "recebida" => "Recebida",
        "efetuada" => "Efetuada",
        "perdida" => "Perdida",
        "correio_de_voz" => "Correio de voz",
        "rejeitada" => "Rejeitada",
        "bloqueada" => "Bloqueada",
        "atendida_em_outro_aparelho" => "Atendida em outro aparelho",
        "mms_nao_baixado" => "MMS não baixado",
        "item" => "Item do caso",
        "recuperado" => "Recuperado (carving)",
        outro => outro,
    }
}

fn nome_tipo_artefato(s: &str) -> &str {
    match s {
        "mensagem" => "Mensagem",
        "conversa" => "Registro da conversa",
        "contato" => "Contato",
        "chamada" => "Chamada",
        "arquivo" => "Arquivo",
        "outro" => "Outro registro",
        outro => outro,
    }
}

fn nome_estado(s: &str) -> &str {
    match s {
        "ativo" => "Ativo",
        "apagado" => "Apagado",
        "recuperado_wal" => "Recuperado do WAL",
        "desconhecido" => "Desconhecido",
        outro => outro,
    }
}

fn nome_tipo_conversa(s: &str) -> &str {
    match s {
        "individual" => "Individual",
        "grupo" => "Grupo",
        "status" => "Status",
        "transmissao" => "Lista de transmissão",
        "canal" => "Canal",
        outro => outro,
    }
}

fn nome_vinculo(s: &str) -> &str {
    match s {
        "caminho_exato" => "arquivo na extração (mesmo caminho)",
        "sufixo_caminho" => "arquivo na extração (mesmo caminho final e tamanho)",
        "nome_e_tamanho" => "arquivo na extração (mesmo nome e tamanho)",
        "ambiguo" => "mais de um arquivo possível: nenhum foi ligado",
        "tamanho_diferente" => "arquivo com o mesmo nome, mas tamanho diferente: não ligado",
        "nao_encontrado" => "não encontrado na extração",
        outro => outro,
    }
}

fn nome_sistema(s: SistemaOperacional) -> &'static str {
    match s {
        SistemaOperacional::Android => "Android",
        SistemaOperacional::Ios => "iOS",
        SistemaOperacional::Outro => "Outro",
        SistemaOperacional::Desconhecido => "Não informado",
    }
}

fn dispositivo(d: &Dispositivo) -> DispositivoDoc {
    let x = &d.dados;
    let mut c = Vec::new();
    opt(&mut c, "Fabricante", x.fabricante.clone());
    opt(&mut c, "Modelo", x.modelo.clone());
    opt(&mut c, "Nome comercial", x.nome_comercial.clone());
    let sistema = match &x.versao_sistema {
        Some(v) => format!("{} {v}", nome_sistema(x.sistema)),
        None => nome_sistema(x.sistema).to_string(),
    };
    c.push(Campo::novo("Sistema", sistema));
    opt(&mut c, "IMEI 1", x.imei1.clone());
    opt(&mut c, "IMEI 2", x.imei2.clone());
    opt(&mut c, "Número de série", x.serial.clone());
    opt(&mut c, "Número da linha", x.numero.clone());
    opt(&mut c, "Operadora", x.operadora.clone());
    opt(
        &mut c,
        "Capacidade",
        x.capacidade_bytes.map(|b| tamanho(b as i64)),
    );
    opt(&mut c, "Observações", x.observacoes.clone());
    DispositivoDoc {
        nome: x.apelido.clone(),
        campos: c,
    }
}

impl Contexto<'_> {
    fn extracao(&self, x: &Extracao, dispositivos: &[Dispositivo]) -> Resultado<ExtracaoDoc> {
        let mut c = Vec::new();
        let disp = dispositivos
            .iter()
            .find(|d| d.id == x.dispositivo_id)
            .map(|d| d.dados.apelido.clone())
            .unwrap_or_else(|| x.dispositivo_id.to_string());
        c.push(Campo::novo("Dispositivo", disp));
        c.push(Campo::novo(
            "Ferramenta de origem",
            format!("{} (importador {})", x.importador_nome, x.importador_versao),
        ));
        opt(&mut c, "Versão da ferramenta", x.ferramenta_versao.clone());
        opt(&mut c, "Tipo de extração", x.tipo_extracao.clone());
        c.push(Campo::novo("Pasta da extração", x.origem_caminho.clone()));
        c.push(Campo::novo(
            "Importada em",
            format!(
                "{} por {} na estação {}",
                self.quando_texto(&x.iniciada_em),
                x.usuario,
                x.maquina
            ),
        ));
        opt(
            &mut c,
            "Concluída em",
            x.concluida_em.as_deref().map(|t| self.quando_texto(t)),
        );
        c.push(Campo::novo("Situação", status_extracao(&x.status)));
        opt(
            &mut c,
            "Arquivos no inventário",
            x.total_arquivos
                .map(|n| format!("{} ({})", milhar(n), tamanho(x.total_bytes.unwrap_or(0)))),
        );
        if let Some(n) = x.arquivos_ilegiveis.filter(|n| *n > 0) {
            c.push(Campo::novo("Arquivos ilegíveis", milhar(n)));
        }
        opt(
            &mut c,
            "Registros interpretados",
            x.total_artefatos.map(milhar),
        );
        opt(
            &mut c,
            "Hash do manifesto (SHA-256)",
            x.hash_manifesto.clone(),
        );
        let execucoes = self.banco.execucoes(x.id)?;
        let parsers = execucoes
            .iter()
            .map(|e| {
                format!(
                    "{} {} ({} registros{})",
                    e.parser_nome,
                    e.parser_versao,
                    milhar(e.artefatos.unwrap_or(0)),
                    match e.avisos {
                        Some(a) if a > 0 => format!(", {a} avisos"),
                        _ => String::new(),
                    }
                )
            })
            .collect::<Vec<_>>();
        if !parsers.is_empty() {
            c.push(Campo::novo("Parsers executados", parsers.join("; ")));
        }
        c.push(Campo::novo(
            "Versão do SIED na importação",
            x.sied_versao.clone(),
        ));
        Ok(ExtracaoDoc {
            nome: format!("Extração {}", x.id),
            campos: c,
        })
    }
}

fn status_extracao(s: &str) -> String {
    match s {
        "em_andamento" => "Em andamento",
        "concluida" => "Concluída",
        "cancelada" => "Cancelada",
        "falhou" => "Falhou",
        "interrompida" => "Interrompida",
        outro => outro,
    }
    .to_string()
}

/// Nomes dos participantes (identificador → nome), resolvidos na própria extração.
fn nomes(c: &Conversa) -> HashMap<String, String> {
    c.participantes
        .iter()
        .filter_map(|p| p.nome.clone().map(|n| (p.identificador.clone(), n)))
        .collect()
}

fn cabecalho_conversa(ctx: &Contexto, c: Option<&Conversa>, msgs: &[ArtefatoResumo]) -> Vec<Campo> {
    let mut campos = Vec::new();
    let app = msgs.first().and_then(|m| m.app.clone());
    campos.push(Campo::novo("Aplicativo", nome_app(app.as_deref())));
    if let Some(c) = c {
        campos.push(Campo::novo(
            "Conversa",
            match &c.titulo {
                Some(t) => format!("{t} ({})", c.id_externo),
                None => c.id_externo.clone(),
            },
        ));
        opt(
            &mut campos,
            "Tipo",
            c.tipo.as_deref().map(|t| nome_tipo_conversa(t).to_string()),
        );
        let lista = c
            .participantes
            .iter()
            .take(12)
            .map(|p| match &p.nome {
                Some(n) => format!("{n} ({})", p.identificador),
                None => p.identificador.clone(),
            })
            .collect::<Vec<_>>();
        if !lista.is_empty() {
            let mut t = lista.join("; ");
            if c.total_participantes > lista.len() as i64 {
                t.push_str(&format!(
                    "; e mais {}",
                    c.total_participantes - lista.len() as i64
                ));
            }
            campos.push(Campo::novo("Participantes", t));
        }
    } else if let Some(id) = msgs.first().and_then(|m| m.conversa_id_externo.clone()) {
        campos.push(Campo::novo("Conversa", id));
    }
    if msgs.len() > 1 {
        campos.push(Campo::novo(
            "Período do trecho",
            format!(
                "{} a {} ({})",
                ctx.quando_ms(msgs.first().and_then(|m| m.data_hora_ms)),
                ctx.quando_ms(msgs.last().and_then(|m| m.data_hora_ms)),
                ctx.fuso.name()
            ),
        ));
        campos.push(Campo::novo(
            "Mensagens no trecho",
            milhar(msgs.len() as i64),
        ));
    }
    campos
}

fn mensagem(
    ctx: &mut Contexto,
    m: &ArtefatoResumo,
    nomes: &HashMap<String, String>,
) -> MensagemDoc {
    let (dia, hora) = ctx.dia_hora(m.data_hora_ms);
    let sistema = m.subtipo.as_deref() == Some("sistema");
    let de_mim = m.de_mim == Some(true);
    let autor = if de_mim {
        "Aparelho examinado (enviada)".to_string()
    } else {
        match m.remetente.as_deref() {
            Some(r) => match nomes.get(r) {
                Some(n) => format!("{n} · {r}"),
                None => r.to_string(),
            },
            None => "Remetente não informado".into(),
        }
    };
    let mut detalhes = Vec::new();
    if let Some(s) = m.subtipo.as_deref() {
        if s != "texto" && Some(s) != m.app.as_deref() && !sistema {
            detalhes.push(nome_subtipo(s).to_string());
        }
    }
    if m.estado_registro != "ativo" {
        detalhes.push(format!("Registro: {}", nome_estado(&m.estado_registro)));
    }
    if let Some(r) = &m.resposta_a_id_externo {
        detalhes.push(format!("Em resposta à mensagem de id {r} no app"));
    }
    if let (Some(la), Some(lo)) = (m.latitude, m.longitude) {
        detalhes.push(format!("Localização: {la:.6}, {lo:.6}"));
    }
    let mut figura = None;
    let tem_anexo = m.nome_arquivo.is_some() || m.anexo_arquivo_id.is_some() || m.mime.is_some();
    if tem_anexo {
        let nome = m.nome_arquivo.clone().unwrap_or_else(|| "sem nome".into());
        let mut t = format!("Anexo: {nome}");
        if let Some(mime) = &m.mime {
            t.push_str(&format!(" ({mime})"));
        }
        if let Some(n) = m.tamanho {
            t.push_str(&format!(", {}", tamanho(n)));
        }
        detalhes.push(t);
        match m.anexo_arquivo_id {
            Some(id) => {
                let info = ctx.banco.arquivo_com_origem(id).ok();
                if let Some(a) = &info {
                    detalhes.push(format!(
                        "Arquivo na extração: {} · SHA-256 {}",
                        a.arquivo.caminho,
                        a.arquivo.sha256.as_deref().unwrap_or("não calculado")
                    ));
                }
                if info.as_ref().map(|a| a.arquivo.categoria.as_str()) == Some("imagem") {
                    let legenda = format!(
                        "Anexo da mensagem {} ({nome}), SHA-256 conferido na geração.",
                        m.id
                    );
                    match ctx.figura(id, legenda) {
                        Ok(f) => figura = Some(f),
                        Err(motivo) => detalhes.push(motivo),
                    }
                }
            }
            None => {
                if let Some(v) = &m.anexo_vinculo {
                    detalhes.push(format!("Arquivo do anexo: {}", nome_vinculo(v)));
                }
            }
        }
    }
    let texto = match (&m.conteudo, m.subtipo.as_deref()) {
        (Some(t), _) => Some(t.clone()),
        (None, Some("apagada")) => Some("(mensagem apagada: o texto não está no banco)".into()),
        (None, Some("sistema")) => Some("Mensagem do sistema".into()),
        (None, _) if !tem_anexo => Some("(sem texto)".into()),
        _ => None,
    };
    MensagemDoc {
        id: m.id,
        de_mim,
        sistema,
        autor,
        dia,
        hora,
        texto,
        detalhes,
        figura,
    }
}

/// Duração em segundos como "1 min 05 s" (ou "42 s").
fn duracao(s: i64) -> String {
    match (s / 3600, (s % 3600) / 60, s % 60) {
        (0, 0, seg) => format!("{seg} s"),
        (0, min, seg) => format!("{min} min {seg:02} s"),
        (h, min, seg) => format!("{h} h {min:02} min {seg:02} s"),
    }
}

fn meta_texto(meta: &serde_json::Value, chave: &str) -> Option<String> {
    meta.get(chave)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// Número de uma chamada: quem ligou (recebida) ou para quem se ligou (efetuada).
fn numero_chamada(m: &ArtefatoResumo) -> Option<String> {
    m.remetente
        .clone()
        .or_else(|| m.destinatarios.first().cloned())
}

fn campos_chamada(ctx: &Contexto, m: &ArtefatoResumo, meta: &serde_json::Value) -> Vec<Campo> {
    let mut c = Vec::new();
    c.push(Campo::novo(
        "Tipo",
        format!(
            "Chamada · {}",
            nome_subtipo(m.subtipo.as_deref().unwrap_or("desconhecido"))
        ),
    ));
    c.push(Campo::novo(
        "Número",
        numero_chamada(m).unwrap_or_else(|| "(não apresentado)".into()),
    ));
    opt(
        &mut c,
        "Nome gravado com a chamada",
        meta_texto(meta, "nome_no_registro"),
    );
    c.push(Campo::novo(
        "Data e hora",
        format!("{} ({})", ctx.quando_ms(m.data_hora_ms), ctx.fuso.name()),
    ));
    opt(
        &mut c,
        "Duração",
        meta.get("duracao_s")
            .and_then(serde_json::Value::as_i64)
            .map(duracao),
    );
    opt(
        &mut c,
        "Apresentação do número",
        meta_texto(meta, "apresentacao").filter(|a| a != "permitido"),
    );
    opt(
        &mut c,
        "Conta telefônica",
        meta_texto(meta, "conta_telefonica"),
    );
    if let Some(r) = meta.get("recursos").and_then(serde_json::Value::as_array) {
        let r: Vec<&str> = r.iter().filter_map(serde_json::Value::as_str).collect();
        if !r.is_empty() {
            c.push(Campo::novo("Recursos", r.join(", ")));
        }
    }
    c.push(Campo::novo(
        "Estado do registro",
        nome_estado(&m.estado_registro),
    ));
    c
}

fn campos_contato(d: &ArtefatoDetalhe) -> Vec<Campo> {
    let (m, meta) = (&d.resumo, &d.metadados);
    let mut c = vec![Campo::novo(
        "Tipo",
        format!("Contato · {}", nome_app(m.app.as_deref())),
    )];
    c.push(Campo::novo(
        "Nome",
        m.conteudo.clone().unwrap_or_else(|| "(sem nome)".into()),
    ));
    let lista = |chave: &str, campo: &str| -> Vec<String> {
        meta.get(chave)
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|t| {
                let valor = t.get(campo)?.as_str()?;
                let tipo = t
                    .get("rotulo")
                    .or_else(|| t.get("tipo_nome"))
                    .and_then(serde_json::Value::as_str);
                Some(match tipo {
                    Some(tp) => format!("{valor} ({})", tp.replace('_', " ")),
                    None => valor.to_string(),
                })
            })
            .collect()
    };
    let mut telefones = lista("telefones", "numero");
    if telefones.is_empty() {
        telefones.extend(meta_texto(meta, "numero"));
    }
    if !telefones.is_empty() {
        c.push(Campo::novo("Telefones", telefones.join("; ")));
    }
    let emails = lista("emails", "endereco");
    if !emails.is_empty() {
        c.push(Campo::novo("E-mails", emails.join("; ")));
    }
    opt(&mut c, "Empresa", meta_texto(meta, "empresa"));
    opt(&mut c, "Conta", d.conta.clone());
    opt(&mut c, "Tipo da conta", meta_texto(meta, "tipo_conta"));
    opt(&mut c, "Identificador no app", m.id_externo.clone());
    c.push(Campo::novo(
        "Estado do registro",
        nome_estado(&m.estado_registro),
    ));
    c
}

fn campos_registro(ctx: &Contexto, d: &ArtefatoDetalhe) -> Vec<Campo> {
    let m = &d.resumo;
    match m.tipo.as_str() {
        "chamada" => return campos_chamada(ctx, m, &d.metadados),
        "contato" => return campos_contato(d),
        _ => {}
    }
    let mut c = Vec::new();
    let tipo = match &m.subtipo {
        Some(s) => format!("{} · {}", nome_tipo_artefato(&m.tipo), nome_subtipo(s)),
        None => nome_tipo_artefato(&m.tipo).to_string(),
    };
    c.push(Campo::novo("Tipo", tipo));
    opt(
        &mut c,
        "Aplicativo",
        m.app.as_deref().map(|a| nome_app(Some(a))),
    );
    opt(&mut c, "Remetente", m.remetente.clone());
    if !m.destinatarios.is_empty() {
        c.push(Campo::novo("Destinatários", m.destinatarios.join("; ")));
    }
    if let Some(d) = m.de_mim {
        c.push(Campo::novo(
            "Direção",
            if d { "Enviada" } else { "Recebida" },
        ));
    }
    c.push(Campo::novo(
        "Data e hora",
        format!("{} ({})", ctx.quando_ms(m.data_hora_ms), ctx.fuso.name()),
    ));
    opt(&mut c, "Conteúdo", m.conteudo.clone());
    opt(&mut c, "Arquivo informado pelo app", m.nome_arquivo.clone());
    if let (Some(la), Some(lo)) = (m.latitude, m.longitude) {
        c.push(Campo::novo("Localização", format!("{la:.6}, {lo:.6}")));
    }
    c.push(Campo::novo(
        "Estado do registro",
        nome_estado(&m.estado_registro),
    ));
    c
}

/// Origem técnica de um registro (apêndice).
fn origem_registro(ctx: &Contexto, titulo: String, d: &ArtefatoDetalhe) -> OrigemDoc {
    let mut campos = vec![
        Campo::novo("Registro no caso", d.resumo.id.to_string()),
        Campo::novo("Extração", d.extracao_id.to_string()),
        Campo::novo(
            "Importador",
            match ctx.extracoes.get(&d.extracao_id.to_string()) {
                Some(x) => format!("{} {}", x.importador_nome, x.importador_versao),
                None => format!("{} {}", d.importador, d.importador_versao),
            },
        ),
        Campo::novo("Pasta da extração", d.origem_caminho.clone()),
    ];
    opt(&mut campos, "Hash do manifesto", d.hash_manifesto.clone());
    campos.push(Campo::novo(
        "Parser",
        format!("{} {}", d.parser, d.parser_versao),
    ));
    opt(&mut campos, "Id do registro no app", d.id_externo.clone());
    if let Some(o) = &d.data_hora_original {
        let t = match &d.formato_data {
            Some(f) => format!("{o} ({f})"),
            None => o.clone(),
        };
        campos.push(Campo::novo("Data como gravada no app", t));
    }
    campos.push(Campo::novo(
        "Confiança da interpretação",
        d.confianca.clone(),
    ));
    campos.push(Campo::novo(
        "hash_registro (SHA-256)",
        d.hash_registro.clone(),
    ));
    OrigemDoc {
        titulo,
        campos,
        referencias: d.referencias.iter().map(referencia).collect(),
        linhas: Vec::new(),
    }
}

fn referencia(r: &sied_armazenamento::extracoes::ReferenciaOrigemDetalhe) -> ReferenciaDoc {
    let mut c = vec![Campo::novo(
        if r.principal {
            "Origem principal"
        } else {
            "Origem adicional"
        },
        r.papel_arquivo.clone(),
    )];
    c.push(Campo::novo("Arquivo", r.arquivo_caminho.clone()));
    c.push(Campo::novo("SHA-256 do arquivo", r.arquivo_sha256.clone()));
    opt(&mut c, "Tabela", r.tabela.clone());
    opt(&mut c, "Id do registro na tabela", r.id_registro.clone());
    opt(&mut c, "Consulta", r.consulta.clone());
    opt(
        &mut c,
        "Deslocamento",
        r.deslocamento.map(|d| d.to_string()),
    );
    ReferenciaDoc {
        campos: c,
        valores_brutos: valores_brutos(&r.valores_brutos),
    }
}

fn situacao(
    conferido: Option<&str>,
    registrado: &str,
    // Por que o hash não pôde ser conferido (origem inacessível).
    falha: Option<String>,
    ok: &str,
) -> (String, String) {
    match (conferido, falha) {
        (Some(h), _) if h == registrado => ("conferida".into(), ok.into()),
        (Some(h), _) => (
            "divergente".into(),
            format!(
                "ATENÇÃO: o hash conferido agora ({h}) difere do registrado na seleção. A evidência não deve ser usada sem nova verificação."
            ),
        ),
        (None, Some(t)) => ("indisponivel".into(), t),
        (None, None) => (
            "indisponivel".into(),
            "Não foi possível conferir o hash na geração.".into(),
        ),
    }
}

/// Monta o documento. `agora` é o instante da geração (vai para a capa e o PDF).
pub fn montar(
    banco: &BancoCaso,
    pedido: &PedidoRelatorio,
    agora: DateTime<Utc>,
    auditoria: &VerificacaoAuditoria,
) -> Resultado<Montado> {
    let caso = banco.caso();
    let fuso = fuso_do_caso(&caso.fuso_horario);
    let extracoes = banco.extracoes()?;
    let mut ctx = Contexto {
        banco,
        fuso,
        extracoes: extracoes
            .iter()
            .map(|x| (x.id.to_string(), x.clone()))
            .collect(),
        imagens: HashMap::new(),
        figuras_omitidas: 0,
    };
    let dispositivos = banco.listar_dispositivos()?;
    let secoes = pedido.secoes.clone().unwrap_or_default();

    let d = &caso.dados;
    let mut procedimento = Vec::new();
    opt(
        &mut procedimento,
        "Tipo de procedimento",
        d.procedimento.clone(),
    );
    opt(
        &mut procedimento,
        "Número do procedimento",
        d.numero_procedimento.clone(),
    );
    opt(&mut procedimento, "Número do BO", d.numero_bo.clone());
    opt(&mut procedimento, "Investigado", d.investigado.clone());
    opt(&mut procedimento, "Vítima", d.vitima.clone());
    opt(&mut procedimento, "Delegado(a)", d.delegado.clone());
    procedimento.push(Campo::novo(
        "Responsável pela análise",
        d.responsavel_analise.clone(),
    ));
    procedimento.push(Campo::novo("Unidade", d.unidade.clone()));
    opt(
        &mut procedimento,
        "Observações do caso",
        d.observacoes.clone(),
    );
    procedimento.push(Campo::novo("Identificador do caso", caso.id.to_string()));
    procedimento.push(Campo::novo("Caso criado em", ctx.quando(caso.criado_em)));

    let capa: Vec<Campo> = procedimento
        .iter()
        .filter(|c| {
            matches!(
                c.rotulo.as_str(),
                "Tipo de procedimento"
                    | "Número do procedimento"
                    | "Número do BO"
                    | "Unidade"
                    | "Responsável pela análise"
            )
        })
        .cloned()
        .collect();

    let mut docs_extracoes = Vec::new();
    for x in &extracoes {
        docs_extracoes.push(ctx.extracao(x, &dispositivos)?);
    }

    let transcricoes = banco
        .transcricoes_caso()?
        .into_iter()
        .map(|t| {
            let segmentos = banco
                .segmentos_transcricao_relatorio(t.id, 500)?
                .into_iter()
                .map(|s| SegmentoTranscricaoDoc {
                    inicio: tempo_transcricao(s.inicio_ms),
                    fim: tempo_transcricao(s.fim_ms),
                    texto: s.texto,
                    falante: s.falante,
                })
                .collect();
            Ok(TranscricaoDoc {
                origem: t.caminho,
                estado: estado_transcricao(t.estado).into(),
                motor: format!("{} v{}", t.motor_id, t.motor_versao),
                hash_origem: t.sha256_origem,
                segmentos,
            })
        })
        .collect::<Resultado<Vec<_>>>()?;

    let lista = banco.evidencias()?;
    let mut evidencias = Vec::new();
    let mut conferencias = Vec::new();
    for e in &lista {
        let (doc, conf) = evidencia(&mut ctx, e)?;
        evidencias.push(doc);
        conferencias.push(conf);
    }

    let divergentes = conferencias
        .iter()
        .filter(|c| c.situacao == "divergente")
        .count();
    let indisponiveis = conferencias
        .iter()
        .filter(|c| c.situacao == "indisponivel")
        .count();
    let mut integridade = vec![
        Campo::novo(
            "Trilha de auditoria do caso",
            format!(
                "{} registros, encadeamento por hash conferido integralmente antes desta geração",
                milhar(auditoria.registros)
            ),
        ),
        Campo::novo("Hash do último registro conferido", auditoria.hash_final.clone()),
        Campo::novo(
            "Registro desta geração",
            // Sem número exato: uma importação que termine durante a geração também
            // grava na trilha, e o registro deste relatório pode não ser o seguinte.
            format!(
                "A geração deste relatório é gravada na trilha em um registro posterior ao nº {} (ação relatorio_gerado), com o SHA-256 deste PDF.",
                milhar(auditoria.registros)
            ),
        ),
        Campo::novo(
            "SHA-256 deste PDF",
            "Um arquivo não pode conter o próprio hash: o SHA-256 fica no arquivo .sha256 gravado ao lado do PDF, no banco do caso e na trilha de auditoria.",
        ),
        Campo::novo(
            "Evidências conferidas",
            format!(
                "{} de {}{}{}",
                conferencias.len() - divergentes - indisponiveis,
                conferencias.len(),
                if divergentes > 0 {
                    format!("; {divergentes} com hash divergente")
                } else {
                    String::new()
                },
                if indisponiveis > 0 {
                    format!("; {indisponiveis} sem conferência (origem indisponível)")
                } else {
                    String::new()
                }
            ),
        ),
    ];
    if ctx.figuras_omitidas > 0 {
        integridade.push(Campo::novo(
            "Figuras omitidas",
            format!(
                "{} imagens não foram incluídas (limite de {MAX_FIGURAS} por relatório); os hashes delas constam nas evidências.",
                ctx.figuras_omitidas
            ),
        ));
    }
    let tabela = conferencias
        .iter()
        .map(|c| {
            vec![
                format!("E-{}", c.numero),
                c.hash_no_momento.clone(),
                match c.situacao.as_str() {
                    "conferida" => "Confere".to_string(),
                    "divergente" => "DIVERGENTE".to_string(),
                    _ => "Não conferida".to_string(),
                },
            ]
        })
        .collect();

    let sessao = banco.sessao();
    let documento = Documento {
        titulo: pedido
            .titulo
            .clone()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| TITULO_PADRAO.into()),
        identificacao: caso.identificacao(),
        gerado_em: ctx.quando(agora),
        fuso: fuso_rotulo(fuso, agora),
        sied_versao: env!("CARGO_PKG_VERSION").into(),
        analista: sessao.usuario.clone(),
        estacao: sessao.maquina.clone(),
        secoes,
        capa,
        procedimento,
        dispositivos: dispositivos.iter().map(dispositivo).collect(),
        extracoes: docs_extracoes,
        metodologia: metodologia(),
        transcricoes,
        evidencias,
        observacoes: limpo(pedido.observacoes.clone()),
        conclusao: limpo(pedido.conclusao.clone()),
        integridade: IntegridadeDoc {
            campos: integridade,
            evidencias: tabela,
        },
    };
    Ok(Montado {
        documento,
        imagens: ctx.imagens,
        conferencias,
    })
}

fn limpo(t: Option<String>) -> Option<String> {
    t.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

pub fn fuso_rotulo(fuso: Tz, agora: DateTime<Utc>) -> String {
    let local = fuso.from_utc_datetime(&agora.naive_utc());
    format!("{}, UTC{}", fuso.name(), local.format("%:z"))
}

fn metodologia() -> Vec<String> {
    vec![
        "As extrações foram importadas pelo SIED em modo somente leitura: nenhum arquivo da pasta de origem foi aberto para escrita. Na importação, cada arquivo teve o SHA-256 calculado e registrado no inventário, e o conjunto foi resumido num manifesto cujo hash consta na seção de extrações.".into(),
        "Os bancos de dados reconhecidos foram copiados para a pasta do caso; a cópia foi conferida pelo SHA-256 e somente ela foi lida pelos parsers. Cada registro interpretado guarda o arquivo, a tabela e o identificador de origem, o parser e a versão que o leram, os valores brutos da linha e um hash próprio (hash_registro). Os registros não são alterados depois de gravados.".into(),
        "As evidências deste relatório foram selecionadas pelo analista no SIED. No momento da seleção foi gravado o hash do alvo: o hash_registro de um registro, o SHA-256 de um arquivo do inventário ou, para um trecho de conversa, o SHA-256 da lista de identificadores e hashes das mensagens, na ordem da conversa. Na geração deste relatório esses hashes foram conferidos de novo: o hash_registro de cada registro foi recalculado a partir do conteúdo gravado no banco do caso (e não apenas lido), e os arquivos e as imagens anexas foram relidos na pasta da extração, com o SHA-256 recalculado. O resultado aparece em cada evidência e na seção de integridade.".into(),
        "Datas e horas são exibidas no fuso horário do caso. O valor exatamente como gravado pelo aplicativo aparece no apêndice técnico.".into(),
        "Caracteres que as fontes deste relatório não desenham (emojis e alguns alfabetos) e caracteres invisíveis ou de controle aparecem pelo código Unicode entre colchetes, na ordem original (ex.: [U+1F600]); o seletor de estilo de emoji (U+FE0F) depois de um caractere desenhado é omitido. O valor exato está nos valores brutos do apêndice técnico e no banco do caso.".into(),
        "Toda ação no caso (criação, cadastro de dispositivos, importações, seleção de evidências e geração de relatórios) é gravada numa trilha de auditoria encadeada por hashes. A trilha foi verificada integralmente antes desta geração.".into(),
    ]
}

/// Por que o conteúdo gravado de um registro não confere com o `hash_registro`.
fn motivo_alterado(c: &ConferenciaRegistro) -> String {
    match &c.recalculado {
        Ok(h) => format!(
            "registro {}: hash recalculado {h}, gravado na importação {}",
            c.artefato_id, c.gravado
        ),
        Err(m) => format!("registro {}: {m}", c.artefato_id),
    }
}

fn texto_alterados(alterados: &[String]) -> String {
    const MAX: usize = 10;
    let mut t = alterados
        .iter()
        .take(MAX)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    if alterados.len() > MAX {
        t.push_str(&format!("; e mais {}", alterados.len() - MAX));
    }
    format!(
        "ATENÇÃO: o conteúdo gravado no banco do caso não corresponde ao hash_registro calculado na importação ({t}). O banco do caso foi alterado fora do SIED; a evidência não deve ser usada sem nova verificação."
    )
}

fn evidencia(ctx: &mut Contexto, e: &Evidencia) -> Resultado<(EvidenciaDoc, Conferencia)> {
    let mut origem = Vec::new();
    // Registros cujo conteúdo gravado não confere com o próprio hash_registro.
    let mut alterados: Vec<String> = Vec::new();
    let (tipo, titulo_padrao, conteudo, conferido, falha, ok) = match e.tipo {
        TipoEvidencia::Registro => {
            let r = e
                .registro
                .as_ref()
                .expect("evidência de registro com registro");
            let det = ctx.banco.artefato(r.id)?;
            // Recalculado do conteúdo gravado, não lido da coluna hash_registro.
            let conf_registro = ctx.banco.conferir_registro(r.id)?;
            if !conf_registro.confere() {
                alterados.push(motivo_alterado(&conf_registro));
            }
            let conversa = match e.conversa_id {
                Some(id) => Some(ctx.banco.conversa(id)?),
                None => None,
            };
            let conteudo = if r.tipo == "mensagem" {
                let nomes = conversa.as_ref().map(nomes).unwrap_or_default();
                let msgs = vec![r.clone()];
                ConteudoDoc::Conversa {
                    cabecalho: cabecalho_conversa(ctx, conversa.as_ref(), &msgs),
                    mensagens: vec![mensagem(ctx, r, &nomes)],
                }
            } else {
                ConteudoDoc::Registro {
                    campos: campos_registro(ctx, &det),
                }
            };
            origem.push(origem_registro(ctx, format!("Registro {}", r.id), &det));
            let titulo = match r.tipo.as_str() {
                "mensagem" => format!(
                    "Mensagem de {} em {}",
                    nome_app(r.app.as_deref()),
                    ctx.quando_ms(r.data_hora_ms)
                ),
                "chamada" => {
                    let numero =
                        numero_chamada(r).unwrap_or_else(|| "número não apresentado".into());
                    let quando = ctx.quando_ms(r.data_hora_ms);
                    match r.subtipo.as_deref() {
                        Some("efetuada") => format!("Chamada efetuada para {numero} em {quando}"),
                        Some(s @ ("recebida" | "perdida" | "rejeitada" | "bloqueada")) => format!(
                            "Chamada {} de {numero} em {quando}",
                            nome_subtipo(s).to_lowercase()
                        ),
                        _ => format!("Chamada ({numero}) em {quando}"),
                    }
                }
                "contato" => match &r.conteudo {
                    Some(n) => format!("Contato: {n}"),
                    None => "Contato sem nome".to_string(),
                },
                t => nome_tipo_artefato(t).to_string(),
            };
            (
                "Registro".to_string(),
                titulo,
                conteudo,
                conf_registro.recalculado.ok(),
                None,
                "O hash_registro foi recalculado a partir do conteúdo gravado no banco do caso e confere com o registrado na seleção.",
            )
        }
        TipoEvidencia::Trecho => {
            let a = e.registro.as_ref().expect("trecho com início");
            let b = e.registro_fim.as_ref().expect("trecho com fim");
            let t = ctx.banco.trecho(a.id, b.id)?;
            // O hash de cada mensagem é recalculado do conteúdo gravado.
            let mut recalculados = Vec::with_capacity(t.mensagens.len());
            for (m, _) in &t.mensagens {
                let c = ctx.banco.conferir_registro(m.id)?;
                if !c.confere() {
                    alterados.push(motivo_alterado(&c));
                }
                recalculados.push((m.id, c.recalculado.unwrap_or_default()));
            }
            let hash = hash_do_trecho(recalculados.iter().map(|(id, h)| (*id, h.as_str())));
            let conversa = match t.conversa_id {
                Some(id) => Some(ctx.banco.conversa(id)?),
                None => None,
            };
            let nomes = conversa.as_ref().map(nomes).unwrap_or_default();
            let msgs: Vec<ArtefatoResumo> = t.mensagens.iter().map(|(m, _)| m.clone()).collect();
            let cabecalho = cabecalho_conversa(ctx, conversa.as_ref(), &msgs);
            let mensagens = msgs.iter().map(|m| mensagem(ctx, m, &nomes)).collect();
            // Apêndice: origem completa da primeira e da última mensagem e uma linha por mensagem.
            let primeira = ctx.banco.artefato(a.id)?;
            let ultima = ctx.banco.artefato(b.id)?;
            let mut o = origem_registro(
                ctx,
                format!(
                    "Trecho de {} mensagens: registros {} a {}",
                    msgs.len(),
                    a.id,
                    b.id
                ),
                &primeira,
            );
            o.campos.retain(|c| {
                !matches!(
                    c.rotulo.as_str(),
                    "Registro no caso"
                        | "Id do registro no app"
                        | "Data como gravada no app"
                        | "hash_registro (SHA-256)"
                )
            });
            o.campos.push(Campo::novo(
                "Hash do trecho (SHA-256)",
                format!("{hash} (SHA-256 das linhas \"id:hash_registro\" abaixo, na ordem)"),
            ));
            o.referencias = Vec::new();
            let mut linhas = Vec::with_capacity(t.mensagens.len());
            for (m, h) in &t.mensagens {
                let det = if m.id == a.id {
                    primeira.clone()
                } else if m.id == b.id {
                    ultima.clone()
                } else {
                    ctx.banco.artefato(m.id)?
                };
                // O arquivo de cada mensagem só aparece quando difere do da primeira.
                let arquivo_primeira = primeira
                    .referencias
                    .iter()
                    .find(|r| r.principal)
                    .map(|r| r.arquivo_caminho.clone());
                let local = det
                    .referencias
                    .iter()
                    .find(|r| r.principal)
                    .map(|r| {
                        let mut t = format!(
                            "{} {}",
                            r.tabela.as_deref().unwrap_or("—"),
                            r.id_registro.as_deref().unwrap_or("—")
                        );
                        if Some(&r.arquivo_caminho) != arquivo_primeira.as_ref() {
                            t = format!("{} · {t}", r.arquivo_caminho);
                        }
                        t
                    })
                    .unwrap_or_else(|| "—".into());
                linhas.push(vec![
                    m.id.to_string(),
                    ctx.quando_ms(m.data_hora_ms),
                    local,
                    h.clone(),
                ]);
            }
            o.linhas = linhas;
            origem.push(o);
            origem.push(origem_registro(
                ctx,
                format!("Primeira mensagem (registro {})", a.id),
                &primeira,
            ));
            origem.push(origem_registro(
                ctx,
                format!("Última mensagem (registro {})", b.id),
                &ultima,
            ));
            let titulo = match conversa.as_ref().and_then(|c| c.titulo.clone()) {
                Some(n) => format!("Trecho da conversa {n}"),
                None => "Trecho de conversa".to_string(),
            };
            (
                format!("Trecho de conversa ({} mensagens)", msgs.len()),
                titulo,
                ConteudoDoc::Conversa {
                    cabecalho,
                    mensagens,
                },
                Some(hash),
                None,
                "O hash de cada mensagem foi recalculado a partir do conteúdo gravado no banco do caso; na ordem da conversa, identificadores e hashes conferem com o registrado na seleção.",
            )
        }
        TipoEvidencia::Arquivo => {
            let f = e
                .arquivo
                .as_ref()
                .expect("evidência de arquivo com arquivo");
            let a = ctx.banco.arquivo_com_origem(f.id)?;
            let p = pedido(ctx.banco, &a.arquivo, &a.origem_caminho);
            let (conferido, falha) = match sied_midia::sha256_original(&p) {
                Ok(sha) => (Some(sha), None),
                Err(m) => (
                    None,
                    Some(format!(
                        "Não foi possível reler o arquivo na geração: {} Vale o SHA-256 do inventário, gravado na importação e na seleção.",
                        m.motivo.unwrap_or_default()
                    )),
                ),
            };
            let mut campos = vec![
                Campo::novo("Caminho na extração", a.arquivo.caminho.clone()),
                Campo::novo("Tamanho", tamanho(a.arquivo.tamanho)),
                Campo::novo(
                    "SHA-256 do inventário",
                    a.arquivo.sha256.clone().unwrap_or_default(),
                ),
            ];
            opt(&mut campos, "Tipo (MIME)", a.arquivo.mime.clone());
            opt(&mut campos, "Assinatura", a.arquivo.assinatura.clone());
            opt(
                &mut campos,
                "Modificado em",
                a.arquivo
                    .modificado_em
                    .as_deref()
                    .map(|t| ctx.quando_texto(t)),
            );
            opt(
                &mut campos,
                "Criado em",
                a.arquivo.criado_em.as_deref().map(|t| ctx.quando_texto(t)),
            );
            let nome = a
                .arquivo
                .caminho
                .rsplit('/')
                .next()
                .unwrap_or(&a.arquivo.caminho)
                .to_string();
            let (figura, sem_figura) = if a.arquivo.categoria != "imagem" {
                (None, None)
            } else if conferido.as_deref() != Some(e.hash_no_momento.as_str()) {
                (
                    None,
                    Some(
                        "A imagem não foi incluída porque o arquivo não pôde ser conferido."
                            .to_string(),
                    ),
                )
            } else {
                match ctx.figura(f.id, format!("{nome}, SHA-256 conferido na geração.")) {
                    Ok(fig) => (Some(fig), None),
                    Err(m) => (None, Some(m)),
                }
            };
            let x = ctx.extracoes.get(&f.extracao_id);
            let mut oc = vec![
                Campo::novo("Extração", f.extracao_id.clone()),
                Campo::novo("Pasta da extração", a.origem_caminho.clone()),
                Campo::novo("Caminho no inventário", a.arquivo.caminho.clone()),
                Campo::novo("Id no inventário do caso", f.id.to_string()),
            ];
            if let Some(x) = x {
                oc.push(Campo::novo(
                    "Importador",
                    format!("{} {}", x.importador_nome, x.importador_versao),
                ));
                opt(&mut oc, "Hash do manifesto", x.hash_manifesto.clone());
            }
            origem.push(OrigemDoc {
                titulo: format!("Arquivo {nome}"),
                campos: oc,
                referencias: Vec::new(),
                linhas: Vec::new(),
            });
            (
                "Arquivo".to_string(),
                nome,
                ConteudoDoc::Arquivo {
                    campos,
                    figura,
                    sem_figura,
                },
                conferido,
                falha,
                "O arquivo foi relido na pasta da extração e o SHA-256 confere com o registrado na seleção.",
            )
        }
    };

    let (sit, sit_texto) = if alterados.is_empty() {
        situacao(conferido.as_deref(), &e.hash_no_momento, falha, ok)
    } else {
        ("divergente".to_string(), texto_alterados(&alterados))
    };

    let mut custodia = vec![
        Campo::novo(
            "Selecionada por",
            format!("{} em {}", e.criada_por, ctx.quando_texto(&e.criada_em)),
        ),
        Campo::novo("Hash registrado na seleção", e.hash_no_momento.clone()),
        Campo::novo(
            "Hash conferido nesta geração",
            conferido.clone().unwrap_or_else(|| "não conferido".into()),
        ),
    ];
    if e.atualizada_em != e.criada_em {
        custodia.push(Campo::novo(
            "Textos editados em",
            ctx.quando_texto(&e.atualizada_em),
        ));
    }

    let doc = EvidenciaDoc {
        numero: e.numero,
        referencia: format!("E-{}", e.numero),
        tipo,
        titulo: e.titulo.clone().unwrap_or(titulo_padrao),
        categoria: e.categoria.clone(),
        observacao: e.observacao.clone(),
        descricao: e.descricao_contextual.clone(),
        custodia,
        situacao: sit.clone(),
        situacao_texto: sit_texto,
        conteudo,
        origem,
    };
    let conf = Conferencia {
        evidencia_id: e.id,
        numero: e.numero,
        tipo: e.tipo,
        hash_no_momento: e.hash_no_momento.clone(),
        hash_conferido: conferido,
        situacao: sit,
    };
    Ok((doc, conf))
}
