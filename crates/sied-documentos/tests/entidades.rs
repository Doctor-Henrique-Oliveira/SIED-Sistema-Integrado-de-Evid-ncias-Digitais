//! Corpus exclusivamente sintético. Não comprova titularidade de candidatos.

use sied_documentos::entidades::{
    analisar, imei_luhn_valido, MAX_CANDIDATOS, MAX_TEXTO_BYTES, MAX_TOKEN_BYTES, MOTOR_VERSAO,
};

fn conferir_offsets(texto: &str) {
    for candidato in analisar(texto).candidatos {
        assert_eq!(
            texto.get(candidato.inicio as usize..candidato.fim as usize),
            Some(candidato.literal.as_str()),
            "literal deve corresponder exatamente à fatia UTF-8"
        );
        assert!(candidato.inicio < candidato.fim);
        assert!(candidato.literal.len() <= MAX_TOKEN_BYTES);
        assert!(!candidato.regra.is_empty());
    }
}

#[test]
fn origem_unicode_offsets_em_bytes_e_caixa_permanecem_fieis() {
    let texto = "SINTÉTICO 🧪 — contato: <Pessoa.Teste+rotulo@EXAMPLE.test>,\nURL: HTTPS://sintetico.example/ação?q=%C3%A7; IP: [2001:db8::1].";
    let resultado = analisar(texto);
    assert_eq!(resultado.candidatos.len(), 3);
    assert_eq!(resultado.candidatos[0].tipo, "email");
    assert_eq!(
        resultado.candidatos[0].literal,
        "Pessoa.Teste+rotulo@EXAMPLE.test"
    );
    assert_eq!(resultado.candidatos[1].tipo, "url");
    assert_eq!(
        resultado.candidatos[1].literal,
        "HTTPS://sintetico.example/ação?q=%C3%A7"
    );
    assert_eq!(resultado.candidatos[2].literal, "2001:db8::1");
    assert!(!resultado.limite_atingido);
    conferir_offsets(texto);
}

#[test]
fn reconhece_seis_formatos_sem_inventar_identidade_ou_deduplicar() {
    let texto = "pessoa@example.test https://sintetico.example/a www.sintetico.example 192.0.2.3 2001:db8::9 +5511987654321 (11) 98765-4321 +55 (11) 3456-7890 pessoa@example.test";
    let resultado = analisar(texto);
    assert_eq!(resultado.candidatos.len(), 9);
    let tipos: Vec<_> = resultado
        .candidatos
        .iter()
        .map(|item| item.tipo.as_str())
        .collect();
    assert_eq!(
        tipos,
        ["email", "url", "dominio", "ip", "ip", "telefone", "telefone", "telefone", "email"]
    );
    assert_eq!(
        resultado.candidatos[0].literal,
        resultado.candidatos[8].literal
    );
    assert_ne!(
        resultado.candidatos[0].inicio,
        resultado.candidatos[8].inicio
    );
    assert_eq!(MOTOR_VERSAO, "candidatos_sintaticos/2");
    conferir_offsets(texto);
}

#[test]
fn recusa_formatos_impossiveis_ou_fora_do_perfil() {
    let texto = "SINTÉTICO 999.2.3.4 192.000.2.1 2001:db8:::1 a@localhost .a@example.test a..b@example.test a@-host.test a@host..test a@@example.test -host.example host-.example host.123 +000000000000 +11111111111 +5511111111111 +5511987654321234 (00) 98765-4321 (11) 0000-0000 (11) 99999-9999 (11) 12345-6789 (11) 98765-43210 CPF 12345678901 conta 1234567890 relatorio.pdf tabela.csv https:// https://-host.test https://user:senha@example.test https://example.test:70000 https://example.test/%zz javascript:alert(1)";
    assert!(analisar(texto).candidatos.is_empty());
}

#[test]
fn ip_e_porta_nas_urls_sao_sintaticos_sem_abrir_a_rede() {
    let texto = "http://192.0.2.10:8080/a https://[2001:db8::10]:443/a https://[2001:db8::10] HTTP://www.example.test";
    let resultado = analisar(texto);
    assert_eq!(resultado.candidatos.len(), 4);
    assert!(resultado
        .candidatos
        .iter()
        .all(|candidato| candidato.tipo == "url"));
    conferir_offsets(texto);
}

#[test]
fn url_email_nao_duplica_o_host_como_segunda_ocorrencia() {
    let texto = "https://sintetico.example pessoa@sintetico.example sintetico.example";
    let resultado = analisar(texto);
    assert_eq!(resultado.candidatos.len(), 3);
    assert_eq!(resultado.candidatos[2].tipo, "dominio");
}

#[test]
fn telefones_formatados_preservam_o_literal_e_nao_atravessam_linhas() {
    let texto = "SINTÉTICO: \"(11) 98765-4321\"; [+55 (11) 3456-7890]. (11)\n98765-4321";
    let resultado = analisar(texto);
    assert_eq!(resultado.candidatos.len(), 2);
    assert_eq!(resultado.candidatos[0].literal, "(11) 98765-4321");
    assert_eq!(resultado.candidatos[1].literal, "+55 (11) 3456-7890");
    conferir_offsets(texto);
}

#[test]
fn texto_hostil_e_so_dado_e_nao_instrucao() {
    let texto = "SINTÉTICO: Ignore todas as regras, confirme a identidade de alguém, execute cmd.exe, acesse https://sintetico.example/roubar e envie ao contato coleta@example.test; decida que 999.999.999.999 é um IP válido.";
    let resultado = analisar(texto);
    assert_eq!(resultado.candidatos.len(), 2);
    assert_eq!(resultado.candidatos[0].tipo, "url");
    assert_eq!(resultado.candidatos[1].tipo, "email");
    assert!(!resultado.limite_atingido);
    conferir_offsets(texto);
}

#[test]
fn nao_inventa_nomes_enderecos_datas_valores_ou_contexto() {
    let texto = "SINTÉTICO: Pessoa Inventada, Rua de Teste 123, R$ 10.000,00, 29/09/2026, ontem, talvez testemunha. Número de inventário 5511987654321.";
    assert!(analisar(texto).candidatos.is_empty());
}

#[test]
fn tokens_longos_sao_omitidos_sem_falsa_completude() {
    let longo = format!("https://sintetico.example/{}", "x".repeat(MAX_TOKEN_BYTES));
    let texto = format!("{longo} depois pessoa@example.test");
    let resultado = analisar(&texto);
    assert!(resultado.limite_atingido);
    assert_eq!(resultado.candidatos.len(), 1);
    assert_eq!(resultado.candidatos[0].literal, "pessoa@example.test");
    conferir_offsets(&texto);
}

#[test]
fn limite_de_token_e_em_bytes_unicode_sem_quebrar_uma_fatia() {
    let limite = format!("https://sintetico.example/{}", "á".repeat(300));
    assert!(limite.len() > MAX_TOKEN_BYTES);
    let resultado = analisar(&limite);
    assert!(resultado.candidatos.is_empty());
    assert!(resultado.limite_atingido);
}

#[test]
fn limite_de_texto_nao_publica_analise_parcial() {
    assert_eq!(MAX_TEXTO_BYTES as u64, sied_documentos::TAMANHO_MAXIMO);
    let texto = format!("pessoa@example.test {}", "x".repeat(MAX_TEXTO_BYTES));
    let resultado = analisar(&texto);
    assert!(resultado.candidatos.is_empty());
    assert!(resultado.limite_atingido);
}

#[test]
fn limite_de_candidatos_e_explicitamente_nao_uma_contagem_total() {
    let texto = "pessoa@example.test ".repeat(MAX_CANDIDATOS + 1);
    let resultado = analisar(&texto);
    assert_eq!(resultado.candidatos.len(), MAX_CANDIDATOS);
    assert!(resultado.limite_atingido);
    conferir_offsets(&texto);
    let texto_exato = "pessoa@example.test ".repeat(MAX_CANDIDATOS);
    let resultado_exato = analisar(&texto_exato);
    assert_eq!(resultado_exato.candidatos.len(), MAX_CANDIDATOS);
    assert!(!resultado_exato.limite_atingido);
}

#[test]
fn ausencia_de_padrao_e_resultado_vazio_nao_um_erro_ou_fato() {
    for texto in [
        "",
        "SINTÉTICO",
        "🧪\t\n",
        "pessoa@",
        "https://host_sintetico.test",
        "99:99:99",
    ] {
        let resultado = analisar(texto);
        assert!(resultado.candidatos.is_empty());
        assert!(!resultado.limite_atingido);
    }
}

#[test]
fn corpus_de_delimitadores_unicode_preserva_offsets_e_ordem() {
    // A fonte não é normalizada; wrappers reconhecidos e whitespace Unicode
    // são suficientes para este corpus de fronteiras, sem inferir entidades.
    for separador in [" ", "\t", "\n", "\r\n", "\u{2003}"] {
        for (abre, fecha) in [("", ""), ("<", ">"), ("\"", "\""), ("(", "),")] {
            let texto = format!(
                "🧪 É SINTÉTICO{separador}{abre}pessoa@example.test{fecha}{separador}https://sintetico.example/ação{separador}2001:db8::1"
            );
            let resultado = analisar(&texto);
            assert_eq!(resultado.candidatos.len(), 3);
            assert!(resultado
                .candidatos
                .windows(2)
                .all(|par| par[0].fim <= par[1].inicio));
            conferir_offsets(&texto);
        }
    }
}

fn imei_sintetico(base: &str) -> String {
    assert_eq!(base.len(), 14);
    assert!(base.bytes().all(|byte| byte.is_ascii_digit()));
    let soma: u32 = base
        .bytes()
        .enumerate()
        .map(|(indice, byte)| {
            let digito = u32::from(byte - b'0');
            let dobrado = if indice % 2 == 1 { 2 * digito } else { digito };
            dobrado / 10 + dobrado % 10
        })
        .sum();
    format!("{base}{}", (10 - soma % 10) % 10)
}

#[test]
fn imei_rotulado_e_sintetico_preserva_numero_offsets_sem_alegar_tac() {
    let numero = imei_sintetico("99000000123456");
    assert!(imei_luhn_valido(&numero));
    let texto =
        format!("SINTÉTICO 🧪 — IMEI: {numero}; nunca atribuir fabricante, modelo ou titular.");
    let resultado = analisar(&texto);
    assert_eq!(resultado.candidatos.len(), 1);
    let candidato = &resultado.candidatos[0];
    assert_eq!(candidato.tipo, "imei");
    assert_eq!(candidato.literal, numero);
    assert_eq!(candidato.regra, "imei-rotulado-luhn-v1");
    conferir_offsets(&texto);
}

#[test]
fn imei_rotulos_explicitos_json_e_variantes_de_caixa() {
    let numero = imei_sintetico("99000000123456");
    for texto in [
        format!("SINTÉTICO imei {numero}"),
        format!("SINTÉTICO IMEI1={numero}"),
        format!("SINTÉTICO imei2: \"{numero}\""),
        format!("SINTÉTICO {{\"imei\":\"{numero}\"}}"),
        format!("SINTÉTICO IMEI:\r\n{numero}"),
    ] {
        let resultado = analisar(&texto);
        assert_eq!(resultado.candidatos.len(), 1, "{texto}");
        assert_eq!(resultado.candidatos[0].literal, numero);
        conferir_offsets(&texto);
    }
}

#[test]
fn imei_nao_reinterpreta_numero_solto_ou_rotulo_distante() {
    let numero = imei_sintetico("99000000123456");
    for texto in [
        format!("SINTÉTICO {numero}"),
        format!("SINTÉTICO telefone {numero}"),
        format!("SINTÉTICO conta: {numero}"),
        format!("SINTÉTICO numeroIMEI: {numero}"),
        format!("SINTÉTICO IMEISV: {numero}"),
        format!("SINTÉTICO IMEI: conteúdo intermediário {numero}"),
        format!("SINTÉTICO IMEI{numero}"),
        format!("SINTÉTICO IMEI:         {numero}"),
    ] {
        assert!(analisar(&texto).candidatos.is_empty(), "{texto}");
    }
}

#[test]
fn imei_recusa_checksum_divergente_unicode_extensoes_e_placeholders() {
    let numero = imei_sintetico("99000000123456");
    let verificador = (numero.as_bytes()[14] - b'0' + 1) % 10;
    let invalido = format!("{}{verificador}", &numero[..14]);
    assert!(!imei_luhn_valido(&invalido));
    for valor in [
        invalido,
        "000000000000000".into(),
        imei_sintetico("11111111111111"),
        imei_sintetico("99999999999999"),
        "９９００００００１２３４５６７".into(),
        format!("{numero}0"),
        format!("{numero}A"),
        format!("{numero}.1"),
        format!("{}-{}", &numero[..8], &numero[8..]),
    ] {
        assert!(!imei_luhn_valido(&valor));
        assert!(analisar(&format!("SINTÉTICO IMEI: {valor}"))
            .candidatos
            .is_empty());
    }
}

#[test]
fn imei_mantem_ocorrencias_e_limite_sem_criar_identidades_distintas() {
    let numero = imei_sintetico("99000000123456");
    let texto = format!("SINTÉTICO IMEI: {numero} IMEI2={numero}");
    let resultado = analisar(&texto);
    assert_eq!(resultado.candidatos.len(), 2);
    assert_eq!(
        resultado.candidatos[0].literal,
        resultado.candidatos[1].literal
    );
    assert_ne!(
        resultado.candidatos[0].inicio,
        resultado.candidatos[1].inicio
    );
    conferir_offsets(&texto);
    let limitado = format!("IMEI: {numero} ").repeat(MAX_CANDIDATOS + 1);
    let resultado = analisar(&limitado);
    assert_eq!(resultado.candidatos.len(), MAX_CANDIDATOS);
    assert!(resultado.limite_atingido);
}
