//! Exercita os contratos de importador e parser com dados **SINTÉTICOS**.
//!
//! ATENÇÃO: a "ferramenta", os arquivos e o formato abaixo são inventados para teste.
//! Não representam a estrutura do Avilla nem de nenhuma ferramenta real.

use serde_json::{json, Map};
use sied_dominio::artefato::{NovoArtefato, ReferenciaRegistro, TipoArtefato};
use sied_ingestao::fonte::{CaminhoFonte, EntradaFonte, FonteSomenteLeitura, LeitorFonte};
use sied_ingestao::importador::{
    ArquivoParaParser, Deteccao, Importador, PlanoImportacao, Suporte, TarefaParser,
};
use sied_ingestao::importadores::avilla::AvillaImporter;
use sied_ingestao::parser::{
    ArquivoDeTrabalho, AvisoParser, EntradaParser, Parser, ResumoParser, SaidaParser,
};
use sied_ingestao::{ErroIngestao, InfoComponente, Registro, Resultado};
use std::collections::BTreeMap;
use std::io::Cursor;

// ---------- Fonte em memória (fixture sintética) ----------

struct FonteSintetica {
    arquivos: BTreeMap<CaminhoFonte, Vec<u8>>,
}

impl FonteSintetica {
    fn nova(arquivos: &[(&str, &str)]) -> Self {
        FonteSintetica {
            arquivos: arquivos
                .iter()
                .map(|(c, conteudo)| (CaminhoFonte::novo(c).unwrap(), conteudo.as_bytes().to_vec()))
                .collect(),
        }
    }
}

impl FonteSomenteLeitura for FonteSintetica {
    fn descricao(&self) -> String {
        "FIXTURE SINTÉTICA (memória)".into()
    }
    fn listar(&self) -> Box<dyn Iterator<Item = Resultado<EntradaFonte>> + '_> {
        Box::new(self.arquivos.iter().map(|(c, d)| {
            Ok(EntradaFonte {
                caminho: c.clone(),
                diretorio: false,
                tamanho: d.len() as u64,
                modificado_em: None,
                criado_em: None,
            })
        }))
    }
    fn entrada(&self, caminho: &CaminhoFonte) -> Resultado<Option<EntradaFonte>> {
        Ok(self
            .listar()
            .filter_map(Result::ok)
            .find(|e| &e.caminho == caminho))
    }
    fn abrir(&self, caminho: &CaminhoFonte) -> Resultado<Box<dyn LeitorFonte>> {
        let dados = self
            .arquivos
            .get(caminho)
            .ok_or_else(|| ErroIngestao::CaminhoInvalido(caminho.to_string()))?;
        Ok(Box::new(Cursor::new(dados.clone())))
    }
}

// ---------- Importador de uma ferramenta fictícia ----------

struct ImportadorSintetico;

impl Importador for ImportadorSintetico {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: "sintetico".into(),
            nome: "Ferramenta sintética de teste".into(),
            versao: "1.0.0".into(),
        }
    }
    fn detectar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<Deteccao> {
        let marca = CaminhoFonte::novo("SINTETICO.txt").unwrap();
        let achou = fonte.entrada(&marca)?.is_some();
        Ok(Deteccao {
            importador: self.info(),
            confianca: if achou { 1.0 } else { 0.0 },
            suporte: if achou {
                Suporte::Completo
            } else {
                Suporte::NaoReconhecido
            },
            motivos: vec![format!(
                "arquivo marcador SINTETICO.txt {}",
                if achou { "presente" } else { "ausente" }
            )],
        })
    }
    fn planejar(&self, fonte: &dyn FonteSomenteLeitura) -> Resultado<PlanoImportacao> {
        let mut plano = PlanoImportacao::default();
        for e in fonte.listar() {
            let e = e?;
            if e.caminho.extensao().as_deref() == Some("chat") {
                plano.tarefas.push(TarefaParser {
                    parser_id: "chat_sintetico".into(),
                    arquivos: vec![ArquivoParaParser {
                        papel: "conversa".into(),
                        caminho: e.caminho,
                    }],
                });
            }
        }
        Ok(plano)
    }
}

// ---------- Parser de um formato fictício: "remetente|texto" por linha ----------

struct ParserChatSintetico;

impl Parser for ParserChatSintetico {
    fn info(&self) -> InfoComponente {
        InfoComponente {
            id: "chat_sintetico".into(),
            nome: "Chat sintético".into(),
            versao: "1.0.0".into(),
        }
    }
    fn papeis(&self) -> &'static [&'static str] {
        &["conversa"]
    }
    fn processar(
        &self,
        entrada: &EntradaParser,
        saida: &mut dyn SaidaParser,
    ) -> Resultado<ResumoParser> {
        let arq = entrada.arquivo("conversa").expect("papel conversa");
        let texto = std::fs::read_to_string(&arq.copia_verificada)?;
        let mut resumo = ResumoParser::default();
        for (n, linha) in texto.lines().enumerate() {
            let linha_num = n + 1;
            match linha.split_once('|') {
                Some((de, msg)) => {
                    let mut brutos = Map::new();
                    brutos.insert("linha".into(), json!(linha));
                    let mut a = NovoArtefato::novo(
                        TipoArtefato::Mensagem,
                        ReferenciaRegistro {
                            papel_arquivo: arq.papel.clone(),
                            tabela: None,
                            id_registro: Some(format!("linha={linha_num}")),
                            consulta: None,
                            deslocamento: None,
                            valores_brutos: brutos,
                        },
                    );
                    a.remetente = Some(de.into());
                    a.conteudo = Some(msg.into());
                    saida.emitir(a)?;
                    resumo.artefatos += 1;
                }
                None => {
                    saida.avisar(AvisoParser {
                        mensagem: "linha sem separador".into(),
                        papel_arquivo: Some(arq.papel.clone()),
                        id_registro: Some(format!("linha={linha_num}")),
                    });
                    resumo.avisos += 1;
                }
            }
        }
        Ok(resumo)
    }
}

#[derive(Default)]
struct Coletor {
    artefatos: Vec<NovoArtefato>,
    avisos: Vec<AvisoParser>,
}

impl SaidaParser for Coletor {
    fn emitir(&mut self, a: NovoArtefato) -> Resultado<()> {
        self.artefatos.push(a);
        Ok(())
    }
    fn avisar(&mut self, a: AvisoParser) {
        self.avisos.push(a);
    }
}

fn fonte() -> FonteSintetica {
    FonteSintetica::nova(&[
        (
            "SINTETICO.txt",
            "fixture sintética — não é estrutura real de ferramenta",
        ),
        (
            "dados/conversa1.chat",
            "Ana|oi\nlinha quebrada\nBruno|tudo bem?",
        ),
        ("dados/foto.bin", "\u{0}\u{1}"),
    ])
}

#[test]
fn detecta_planeja_e_processa_com_rastreabilidade() {
    let mut registro = Registro::padrao();
    registro.adicionar_importador(Box::new(ImportadorSintetico));
    registro.adicionar_parser(Box::new(ParserChatSintetico));

    let f = fonte();
    let deteccoes = registro.detectar(&f).unwrap();
    assert_eq!(
        deteccoes[0].importador.id, "sintetico",
        "o reconhecido vem primeiro"
    );
    assert_eq!(deteccoes[1].importador.id, "avilla");
    assert_eq!(deteccoes[1].suporte, Suporte::NaoImplementado);

    let plano = registro
        .importador("sintetico")
        .unwrap()
        .planejar(&f)
        .unwrap();
    assert_eq!(plano.tarefas.len(), 1);
    let tarefa = &plano.tarefas[0];

    // O pipeline real copiará e conferirá o hash; aqui simulamos a cópia de trabalho.
    let dir = tempfile::tempdir().unwrap();
    let caminho = &tarefa.arquivos[0].caminho;
    let copia = dir.path().join("copia.chat");
    let mut leitor = f.abrir(caminho).unwrap();
    std::io::copy(&mut leitor, &mut std::fs::File::create(&copia).unwrap()).unwrap();

    let entrada = EntradaParser {
        arquivos: vec![ArquivoDeTrabalho {
            papel: tarefa.arquivos[0].papel.clone(),
            origem: caminho.clone(),
            sha256: "sintetico".into(),
            copia_verificada: copia,
        }],
        fuso_horario: "America/Sao_Paulo".into(),
    };
    let mut saida = Coletor::default();
    let resumo = registro
        .parser(&tarefa.parser_id)
        .unwrap()
        .processar(&entrada, &mut saida)
        .unwrap();

    assert_eq!(
        resumo,
        ResumoParser {
            artefatos: 2,
            avisos: 1
        }
    );
    assert_eq!(saida.artefatos[1].conteudo.as_deref(), Some("tudo bem?"));
    assert_eq!(
        saida.artefatos[1].origem.id_registro.as_deref(),
        Some("linha=3")
    );
    assert_eq!(saida.avisos[0].id_registro.as_deref(), Some("linha=2"));

    // Entrada e saída do parser atravessam fronteira de processo em JSON sem perda.
    let json = serde_json::to_string(&entrada).unwrap();
    assert_eq!(
        serde_json::from_str::<EntradaParser>(&json).unwrap(),
        entrada
    );
}

#[test]
fn avilla_nao_presume_estrutura() {
    let f = fonte();
    let d = AvillaImporter.detectar(&f).unwrap();
    assert_eq!(d.suporte, Suporte::NaoImplementado);
    assert_eq!(d.confianca, 0.0);
    assert!(matches!(
        AvillaImporter.planejar(&f),
        Err(ErroIngestao::NaoImplementado { .. })
    ));
}
