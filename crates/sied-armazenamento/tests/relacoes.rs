//! Relações do caso exclusivamente SINTÉTICO; nenhuma conclusão de causalidade.

#[path = "../../../testes/rust/caso_sintetico.rs"]
mod comum;

use sied_armazenamento::relacoes::ClasseRelacao;
use sied_armazenamento::{ControleDescoberta, ErroArmazenamento};

#[test]
fn relacoes_conferidas_separam_origem_direta_de_proximidade_sugerida() {
    let (_raiz, origem, banco, _extracao) = comum::preparado();
    let antes = comum::retrato(&origem);
    let mensagens = comum::mensagens_ana(&banco);
    let registro = mensagens.first().unwrap();
    let r = banco
        .contexto_relacoes(registro.id, 30, 30, &ControleDescoberta::novo())
        .unwrap();
    assert_eq!(r.caso_id, banco.caso().id);
    assert_eq!(r.origem.artefato.id, registro.id);
    assert!(r.eventos.itens.len() <= 30);
    assert!(r.nos.iter().any(|no| no.artefato_id == Some(registro.id)));
    assert!(r.arestas.iter().any(
        |e| e.classe == ClasseRelacao::Direta && e.mecanismo == "conversa_chave_registrada_v1"
    ));
    for aresta in &r.arestas {
        assert!(r.nos.iter().any(|no| no.chave == aresta.de));
        assert!(r.nos.iter().any(|no| no.chave == aresta.para));
        match aresta.classe {
            ClasseRelacao::Direta => assert!(aresta.score_contexto.is_none()),
            ClasseRelacao::Sugerida => {
                assert_eq!(aresta.mecanismo, "proximidade_utc_linear_janela_v1");
                assert!(aresta.motivo.contains("não causalidade"));
                assert!((0.0..=1.0).contains(&aresta.score_contexto.unwrap()));
            }
        }
    }
    assert_eq!(comum::retrato(&origem), antes);
    banco.verificar_auditoria().unwrap();
}

#[test]
fn contexto_recusa_registro_ausente_janela_fora_da_politica_e_cancelamento() {
    let (_raiz, _origem, banco, _extracao) = comum::preparado();
    let id = comum::mensagens_ana(&banco)[0].id;
    assert!(banco
        .contexto_relacoes(-1, 30, 30, &ControleDescoberta::novo())
        .is_err());
    assert!(banco
        .contexto_relacoes(i64::MAX, 30, 30, &ControleDescoberta::novo())
        .is_err());
    assert!(banco
        .contexto_relacoes(id, 6, 30, &ControleDescoberta::novo())
        .is_err());
    assert!(banco
        .contexto_relacoes(id, 30, 31, &ControleDescoberta::novo())
        .is_err());
    let cancelado = ControleDescoberta::novo();
    assert!(cancelado.cancelar());
    assert!(matches!(
        banco.contexto_relacoes(id, 30, 30, &cancelado),
        Err(ErroArmazenamento::DescobertaCancelada)
    ));
}
