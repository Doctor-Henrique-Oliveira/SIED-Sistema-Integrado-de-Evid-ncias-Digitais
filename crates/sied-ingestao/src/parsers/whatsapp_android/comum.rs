//! Utilidades comuns aos dois esquemas do `msgstore.db` e ao `wa.db`.

use serde_json::Value;
use sied_dominio::artefato::{DataHora, FormatoData};

use crate::fonte::CaminhoFonte;
pub use crate::parsers::comum::{faltando, inteiro, referencia, texto, Auxiliar, Emissor, Linha};

pub fn real(l: &Linha, c: &str) -> Option<f64> {
    match l.get(c)? {
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

/// Latitude e longitude interpretadas só quando as duas são números no intervalo
/// válido (±90 e ±180). Senão nenhuma é interpretada (os valores continuam nos valores
/// brutos da origem) e o segundo item diz que havia algo descartado.
pub fn coordenadas(l: &Linha) -> (Option<(f64, f64)>, bool) {
    let (la, lo) = (real(l, "latitude"), real(l, "longitude"));
    match (la, lo) {
        (Some(la), Some(lo)) if la.abs() <= 90.0 && lo.abs() <= 180.0 => (Some((la, lo)), false),
        (None, None) => (
            None,
            !matches!(
                (l.get("latitude"), l.get("longitude")),
                (None | Some(Value::Null), None | Some(Value::Null))
            ),
        ),
        _ => (None, true),
    }
}

/// Data em milissegundos UTC (como o WhatsApp grava). Zero ou vazio = sem data.
pub fn data_ms(l: &Linha, c: &str) -> Option<DataHora> {
    let v = l.get(c)?;
    let ms = inteiro(l, c).filter(|&ms| ms > 0)?;
    Some(DataHora {
        utc_ms: DataHora::ms_plausivel(ms),
        original: match v {
            Value::String(s) => s.clone(),
            outro => outro.to_string(),
        },
        formato: FormatoData::UnixMilissegundos,
    })
}

/// Tipo da conversa pelo servidor do jid (parte depois do `@`).
pub fn tipo_conversa(jid: &str) -> &'static str {
    let (usuario, servidor) = jid.rsplit_once('@').unwrap_or((jid, ""));
    match servidor {
        "s.whatsapp.net" | "c.us" | "lid" => "individual",
        "g.us" => "grupo",
        "broadcast" if usuario == "status" => "status",
        "broadcast" => "transmissao",
        "newsletter" => "canal",
        _ => "desconhecido",
    }
}

/// Código do tipo de mensagem (`message_type` no esquema novo, `media_wa_type` no
/// antigo) → subtipo. Só os códigos da referência pública; os demais ficam como
/// "desconhecido", com o código bruto preservado nos metadados.
pub fn subtipo_mensagem(codigo: i64) -> Option<&'static str> {
    Some(match codigo {
        0 => "texto",
        1 => "imagem",
        2 => "audio",
        3 => "video",
        4 => "contato",
        5 => "localizacao",
        7 => "sistema",
        9 => "documento",
        13 => "gif",
        14 => "contatos",
        15 => "apagada",
        16 => "localizacao_tempo_real",
        20 => "figurinha",
        _ => return None,
    })
}

/// Subtipos cujo anexo é um arquivo de mídia ou documento.
pub fn tem_arquivo(subtipo: &str) -> bool {
    matches!(
        subtipo,
        "imagem" | "audio" | "video" | "documento" | "gif" | "figurinha"
    )
}

/// Situação de envio pelo código `status` (referência pública; o valor bruto também
/// vai nos metadados).
pub fn situacao_envio(de_mim: bool, status: i64) -> Option<&'static str> {
    Some(match (de_mim, status) {
        (false, 0) => "recebida",
        (true, 4) => "enviada_ao_servidor",
        (true, 5) => "entregue",
        (true, 13) => "lida",
        (true, 8) => "reproduzida",
        (_, 6) => "controle",
        _ => return None,
    })
}

/// WhatsApp Business usa os mesmos nomes de banco, na pasta do pacote
/// `com.whatsapp.w4b`. Sem essa pista no caminho, o app é o WhatsApp comum.
pub fn app_do_caminho(origem: &CaminhoFonte) -> &'static str {
    if origem
        .como_str()
        .split('/')
        .any(|p| p.eq_ignore_ascii_case("com.whatsapp.w4b"))
    {
        "whatsapp_business"
    } else {
        "whatsapp"
    }
}

/// Último componente de um caminho gravado pelo app (com `/` ou `\`).
pub fn nome_do_caminho(caminho: &str) -> Option<String> {
    caminho
        .rsplit(['/', '\\'])
        .next()
        .filter(|n| !n.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn tipo_da_conversa_pelo_jid() {
        assert_eq!(tipo_conversa("5511900000011@s.whatsapp.net"), "individual");
        assert_eq!(tipo_conversa("120363000000000001@g.us"), "grupo");
        assert_eq!(tipo_conversa("status@broadcast"), "status");
        assert_eq!(tipo_conversa("1234@broadcast"), "transmissao");
        assert_eq!(tipo_conversa("999@newsletter"), "canal");
        assert_eq!(tipo_conversa("-1"), "desconhecido");
    }

    #[test]
    fn inteiro_aceita_texto_numerico_do_esquema_antigo() {
        let mut l = Linha::new();
        l.insert("a".into(), json!("15"));
        l.insert("b".into(), json!(7));
        l.insert("c".into(), json!("x"));
        assert_eq!(inteiro(&l, "a"), Some(15));
        assert_eq!(inteiro(&l, "b"), Some(7));
        assert_eq!(inteiro(&l, "c"), None);
    }

    #[test]
    fn data_zero_nao_vira_data() {
        let mut l = Linha::new();
        l.insert("t".into(), json!(0));
        l.insert("u".into(), json!(1767355200000i64));
        assert!(data_ms(&l, "t").is_none());
        let d = data_ms(&l, "u").unwrap();
        assert_eq!(d.utc_ms, Some(1767355200000));
        assert_eq!(d.original, "1767355200000");
    }

    #[test]
    fn app_business_pela_pasta_do_pacote() {
        let c = CaminhoFonte::novo("dados/com.whatsapp.w4b/databases/msgstore.db").unwrap();
        assert_eq!(app_do_caminho(&c), "whatsapp_business");
        let c = CaminhoFonte::novo("dados/com.whatsapp/databases/msgstore.db").unwrap();
        assert_eq!(app_do_caminho(&c), "whatsapp");
    }

    #[test]
    fn nome_do_arquivo_com_barras_dos_dois_tipos() {
        assert_eq!(
            nome_do_caminho("Media/WhatsApp Images/IMG-1.jpg").as_deref(),
            Some("IMG-1.jpg")
        );
        assert_eq!(nome_do_caminho(r"C:\x\y.pdf").as_deref(), Some("y.pdf"));
        assert_eq!(nome_do_caminho("pasta/"), None);
    }
}
