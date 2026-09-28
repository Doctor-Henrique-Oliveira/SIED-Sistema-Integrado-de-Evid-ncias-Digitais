//! Pipeline com destino em memória e fonte **SINTÉTICA** que muda entre leituras.
//!
//! ATENÇÃO: fonte, importador e conteúdo são inventados para o teste.

use sied_ingestao::fonte::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura, LeitorFonte};
use sied_ingestao::importador::{
    ArquivoParaParser, Deteccao, Importador, PlanoImportacao, Suporte, TarefaParser,
};
use sied_ingestao::inventario::ArquivoInventariado;
use sied_ingestao::parser::{AvisoParser, ResumoParser};
use sied_ingestao::pipeline::{
    self, ArquivoRegistrado, ArtefatoParaGravar, ContextoImportacao, Controle, Destino,
    EstadoExecucao,
};
use sied_ingestao::{InfoComponente, Registro, Resultado};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Devolve um conteúdo na primeira leitura de cada arquivo e outro nas seguintes,
/// simulando um original alterado depois do inventário.
struct FonteQueMuda {
    leituras: AtomicUsize,
}

impl FonteSomenteLeitura for FonteQueMuda {
    fn descricao(&self) -> String {
        "FIXTURE SINTÉTICA".into()
    }
    fn listar(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
        Box::new(std::iter::once(Ok(EntradaFonte {
            caminho: CaminhoFonte::novo("dados/mmssms.db").unwrap(),
            diretorio: false,
            tamanho: 5,
            modificado_em: None,
            criado_em: None,
        })))
    }
    fn entrada(&self, _: &CaminhoFonte) -> Resultado<Option<EntradaFonte>> {
        Ok(self.listar().next().transpose()?)
    }
    fn abrir(&self, _: &CaminhoFonte) -> Resultado<Box<dyn LeitorFonte>> {
        let n = self.leituras.fetch_add(1, Ordering::SeqCst);
        let dados: &[u8] = if n == 0 { b"antes" } else { b"depois" };
        Ok(Box::new(Cursor::new(dados.to_vec())))
    }
}

struct ImportadorFixo(&'static str);

impl Importador for ImportadorFixo {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: "sintetico".into(),
            nome: "Sintético".into(),
            versao: "1".into(),
        }
    }
    fn detectar(&self, _: &dyn FonteSomenteLeitura) -> Resultado<Deteccao> {
        Ok(Deteccao {
            importador: self.info(),
            confianca: 1.0,
            suporte: Suporte::Completo,
            motivos: vec![],
        })
    }
    fn planejar(&self, _: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao> {
        Ok(PlanoImportacao {
            tarefas: vec![TarefaParser {
                parser_id: self.0.into(),
                arquivos: vec![ArquivoParaParser {
                    papel: "mmssms".into(),
                    caminho: CaminhoFonte::novo("dados/mmssms.db").unwrap(),
                }],
            }],
            ..Default::default()
        })
    }
}

#[derive(Default)]
struct Memoria {
    arquivos: Vec<ArquivoInventariado>,
    execucoes: Vec<(String, Option<EstadoExecucao>, Option<String>)>,
    artefatos: usize,
}

impl Destino for Memoria {
    fn gravar_arquivos(&mut self, lote: &[ArquivoInventariado]) -> Resultado<()> {
        self.arquivos.extend_from_slice(lote);
        Ok(())
    }
    fn arquivo(&mut self, c: &CaminhoFonte) -> Resultado<Option<ArquivoRegistrado>> {
        Ok(self
            .arquivos
            .iter()
            .position(|a| &a.entrada.caminho == c)
            .map(|i| ArquivoRegistrado {
                id: i as i64 + 1,
                sha256: self.arquivos[i].sha256.clone(),
                tamanho: self.arquivos[i].entrada.tamanho,
            }))
    }
    fn iniciar_execucao(
        &mut self,
        p: &InfoComponente,
        _: Option<i64>,
        _: Option<&str>,
    ) -> Resultado<i64> {
        self.execucoes.push((p.id.clone(), None, None));
        Ok(self.execucoes.len() as i64)
    }
    fn gravar_artefatos(&mut self, _: i64, lote: &[ArtefatoParaGravar]) -> Resultado<()> {
        self.artefatos += lote.len();
        Ok(())
    }
    fn gravar_aviso(&mut self, _: i64, _: &AvisoParser) -> Resultado<()> {
        Ok(())
    }
    fn concluir_execucao(
        &mut self,
        id: i64,
        estado: EstadoExecucao,
        _: &ResumoParser,
        erro: Option<&str>,
    ) -> Resultado<()> {
        let e = &mut self.execucoes[id as usize - 1];
        e.1 = Some(estado);
        e.2 = erro.map(String::from);
        Ok(())
    }
}

fn contexto(dir: &std::path::Path) -> ContextoImportacao {
    ContextoImportacao {
        pasta_importacao: dir.join("imp"),
        pasta_copias: dir.join("copias"),
        fuso_horario: "America/Sao_Paulo".into(),
    }
}

#[test]
fn original_alterado_depois_do_inventario_nao_e_processado() {
    let dir = tempfile::tempdir().unwrap();
    let fonte = FonteQueMuda {
        leituras: AtomicUsize::new(0),
    };
    let mut destino = Memoria::default();
    let r = pipeline::executar(
        &fonte,
        &ImportadorFixo("android_sms"),
        &Registro::padrao(),
        &mut destino,
        &Controle::default(),
        &contexto(dir.path()),
    )
    .unwrap();
    assert_eq!(r.arquivos, 1);
    assert_eq!(r.execucoes_com_falha, 1);
    let (parser, estado, erro) = &destino.execucoes[0];
    assert_eq!(parser, "android_sms");
    assert_eq!(*estado, Some(EstadoExecucao::Falhou));
    assert!(erro.as_ref().unwrap().contains("mudou desde o inventário"));
    // A cópia divergente foi descartada.
    let copias: Vec<_> = walkdir::WalkDir::new(dir.path().join("copias"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect();
    assert!(copias.is_empty());
    assert!(r
        .observacoes
        .iter()
        .any(|o| o.contains("mudou desde o inventário")));
}

#[test]
fn parser_inexistente_vira_observacao() {
    let dir = tempfile::tempdir().unwrap();
    let fonte = FonteQueMuda {
        leituras: AtomicUsize::new(0),
    };
    let mut destino = Memoria::default();
    let r = pipeline::executar(
        &fonte,
        &ImportadorFixo("nao_existe"),
        &Registro::padrao(),
        &mut destino,
        &Controle::default(),
        &contexto(dir.path()),
    )
    .unwrap();
    assert!(destino.execucoes.is_empty());
    assert!(r.observacoes.iter().any(|o| o.contains("nao_existe")));
}
