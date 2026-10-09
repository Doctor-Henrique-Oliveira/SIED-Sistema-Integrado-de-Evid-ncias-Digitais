//! Ponte de leitura conferida → derivados do caso. Nenhum texto/candidato é
//! aceito do frontend e nenhuma ocorrência cria artefato ou evidência.

use sied_armazenamento::documentos::{
    AchadoDocumento, DocumentoIndexado, EntidadeDocumento, EstadoRevisaoEntidade,
    NovaEntidadeDocumento, NovaRegiaoDocumento, NovoDocumento, RegiaoDocumento, RevisaoEntidade,
};
use sied_armazenamento::extracoes::Pagina;
use sied_documentos::{ControleDocumento, EstadoDocumento, ResultadoDocumento};
use sied_dominio::CasoId;

use crate::erro::{CodigoErro, ErroApp, ResultadoApp};
use crate::servico::Servico;

/// Somente o resultado do leitor Rust entra nesta função. Cancelamento é checado
/// antes/depois dos candidatos e novamente, sob a trava, antes da transação.
pub(crate) fn preparar_derivado(
    resultado: ResultadoDocumento,
    controle: &ControleDocumento,
) -> ResultadoApp<NovoDocumento> {
    conferir_cancelamento(controle)?;
    if resultado.estado != EstadoDocumento::Pronto {
        let codigo = match resultado.estado {
            EstadoDocumento::OrigemDivergente | EstadoDocumento::HashAusente => {
                CodigoErro::Integridade
            }
            EstadoDocumento::SemSuporte => CodigoErro::NaoSuportado,
            EstadoDocumento::Cancelado => CodigoErro::Conflito,
            EstadoDocumento::LimiteExcedido => CodigoErro::Validacao,
            _ => CodigoErro::Interno,
        };
        return Err(ErroApp::novo(
            codigo,
            resultado
                .motivo
                .unwrap_or_else(|| "o leitor documental não publicou texto".into()),
        ));
    }
    let texto = resultado.texto.ok_or_else(|| {
        ErroApp::novo(
            CodigoErro::Integridade,
            "a leitura não contém texto e proveniência íntegros",
        )
    })?;
    let analise = sied_documentos::entidades::analisar(&texto.texto);
    conferir_cancelamento(controle)?;
    Ok(NovoDocumento {
        caso_id: texto.caso_id,
        extracao_id: texto.extracao_id,
        arquivo_id: texto.arquivo_id,
        caminho: texto.caminho.to_string(),
        sha256_origem: texto.sha256,
        sha256_texto: texto.sha256_texto,
        texto: texto.texto,
        pagina: texto.pagina,
        total_paginas: texto.total_paginas,
        largura_px: texto.largura_px,
        altura_px: texto.altura_px,
        regioes: texto
            .regioes
            .into_iter()
            .map(|r| NovaRegiaoDocumento {
                ordem: r.ordem,
                inicio_utf8: r.inicio_utf8,
                fim_utf8: r.fim_utf8,
                texto: r.texto,
                x: r.x,
                y: r.y,
                largura: r.largura,
                altura: r.altura,
                confianca: r.confianca,
            })
            .collect(),
        motor_id: texto.motor_id,
        motor_versao: texto.motor_versao,
        executado_em: texto.executado_em.to_rfc3339(),
        entidades_limitadas: analise.limite_atingido,
        entidades_motor_versao: sied_documentos::entidades::MOTOR_VERSAO.into(),
        entidades: analise
            .candidatos
            .into_iter()
            .map(|e| NovaEntidadeDocumento {
                tipo: e.tipo,
                inicio_utf8: e.inicio,
                fim_utf8: e.fim,
                literal: e.literal,
                regra_versao: e.regra,
            })
            .collect(),
    })
}

pub(crate) fn conferir_cancelamento(controle: &ControleDocumento) -> ResultadoApp<()> {
    if controle.esta_cancelado() {
        Err(ErroApp::novo(
            CodigoErro::Conflito,
            "operação documental cancelada; nenhum novo derivado foi gravado",
        ))
    } else {
        Ok(())
    }
}

impl Servico {
    pub fn guardar_documento(
        &mut self,
        novo: NovoDocumento,
        controle: &ControleDocumento,
    ) -> ResultadoApp<DocumentoIndexado> {
        conferir_cancelamento(controle)?;
        Ok(self
            .banco_documental(novo.caso_id)?
            .guardar_documento(&novo)?)
    }

    pub fn buscar_documentos(
        &mut self,
        caso_id: CasoId,
        termo: &str,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<AchadoDocumento>> {
        Ok(self.banco_documental(caso_id)?.buscar_documentos(
            termo,
            deslocamento.max(0),
            limite.clamp(1, 50),
        )?)
    }

    pub fn obter_documento(
        &mut self,
        caso_id: CasoId,
        documento_id: i64,
    ) -> ResultadoApp<DocumentoIndexado> {
        Ok(self
            .banco_documental(caso_id)?
            .obter_documento(documento_id)?)
    }

    pub fn regioes_documento(
        &mut self,
        caso_id: CasoId,
        documento_id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<RegiaoDocumento>> {
        Ok(self.banco_documental(caso_id)?.regioes_documento(
            documento_id,
            deslocamento.max(0),
            limite.clamp(1, 100),
        )?)
    }

    pub fn localizar_regiao_documento(
        &mut self,
        caso_id: CasoId,
        documento_id: i64,
        inicio_utf8: u32,
        fim_utf8: u32,
    ) -> ResultadoApp<Option<u32>> {
        Ok(self.banco_documental(caso_id)?.localizar_regiao_documento(
            documento_id,
            inicio_utf8,
            fim_utf8,
        )?)
    }

    pub fn entidades_documento(
        &mut self,
        caso_id: CasoId,
        documento_id: i64,
    ) -> ResultadoApp<Vec<EntidadeDocumento>> {
        Ok(self
            .banco_documental(caso_id)?
            .entidades_documento(documento_id)?)
    }

    pub fn revisar_entidade(
        &mut self,
        caso_id: CasoId,
        entidade_id: i64,
        estado: EstadoRevisaoEntidade,
        nota: Option<String>,
    ) -> ResultadoApp<EntidadeDocumento> {
        Ok(self.banco_documental(caso_id)?.revisar_entidade(
            entidade_id,
            estado,
            nota.as_deref(),
        )?)
    }

    pub fn revisoes_entidade(
        &mut self,
        caso_id: CasoId,
        entidade_id: i64,
        deslocamento: i64,
        limite: i64,
    ) -> ResultadoApp<Pagina<RevisaoEntidade>> {
        Ok(self.banco_documental(caso_id)?.revisoes_entidade(
            entidade_id,
            deslocamento.max(0),
            limite.clamp(1, 50),
        )?)
    }
}
