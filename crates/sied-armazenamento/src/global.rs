//! Banco global da estação: catálogo de casos e configurações.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sied_dominio::{Caso, CasoId};
use std::path::{Path, PathBuf};
use ts_rs::TS;

use crate::caso::NOME_ARQUIVO_CASO;
use crate::conexao::{self, Abertura};
use crate::migracoes::{self, BANCO_GLOBAL};
use crate::Resultado;

pub const NOME_ARQUIVO_GLOBAL: &str = "sied-global.db";

/// Entrada do catálogo, suficiente para listar casos sem abrir cada banco.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CasoCatalogado {
    pub caso_id: CasoId,
    pub pasta: String,
    pub numero_procedimento: Option<String>,
    pub numero_bo: Option<String>,
    pub unidade: String,
    pub responsavel_analise: String,
    pub criado_em: String,
    pub ultimo_acesso_em: Option<String>,
    /// Falso quando a pasta não está acessível (disco removido, caso movido).
    pub disponivel: bool,
}

/// Provedor registrado na estação. Este é somente um manifesto de configuração:
/// o SIED não armazena segredos nem executa chamadas externas a partir dele.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvedorEstacao {
    pub id: String,
    pub nome: String,
    pub tipo: String,
    pub modo: String,
    pub habilitado: bool,
    pub caminho: Option<String>,
    pub versao: Option<String>,
    pub observacao: Option<String>,
    pub criado_em: String,
    pub atualizado_em: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NovoProvedorEstacao {
    pub id: String,
    pub nome: String,
    pub tipo: String,
    pub modo: String,
    pub habilitado: bool,
    pub caminho: Option<String>,
    pub versao: Option<String>,
    pub observacao: Option<String>,
}

pub struct CatalogoGlobal {
    conn: Connection,
    caminho: PathBuf,
}

impl CatalogoGlobal {
    /// Abre (ou cria) o banco global dentro da pasta de dados do aplicativo.
    pub fn abrir(pasta_dados: &Path) -> Resultado<Self> {
        std::fs::create_dir_all(pasta_dados)?;
        let caminho = pasta_dados.join(NOME_ARQUIVO_GLOBAL);
        // Um arquivo vazio (queda durante a criação) é tratado como banco novo.
        let mut conn = conexao::abrir(&caminho, &BANCO_GLOBAL, Abertura::Criar)?;
        let versao = migracoes::preparar(&mut conn, &caminho, &BANCO_GLOBAL)?;
        tracing::info!(caminho = %caminho.display(), versao, "banco global aberto");
        Ok(CatalogoGlobal { conn, caminho })
    }

    pub fn caminho(&self) -> &Path {
        &self.caminho
    }

    /// Inclui ou atualiza um caso no catálogo (ex.: após criar ou abrir uma pasta de caso).
    pub fn registrar_caso(&self, caso: &Caso, pasta: &Path) -> Resultado<()> {
        let agora = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO caso_catalogo(caso_id, pasta, numero_procedimento, numero_bo, unidade,
                                       responsavel_analise, criado_em, registrado_em, ultimo_acesso_em)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(caso_id) DO UPDATE SET
                pasta = excluded.pasta,
                numero_procedimento = excluded.numero_procedimento,
                numero_bo = excluded.numero_bo,
                unidade = excluded.unidade,
                responsavel_analise = excluded.responsavel_analise,
                ultimo_acesso_em = excluded.ultimo_acesso_em",
            params![
                caso.id.to_string(),
                pasta.to_string_lossy(),
                caso.dados.numero_procedimento,
                caso.dados.numero_bo,
                caso.dados.unidade,
                caso.dados.responsavel_analise,
                caso.criado_em.to_rfc3339(),
                agora,
            ],
        )?;
        Ok(())
    }

    pub fn marcar_acesso(&self, caso_id: CasoId) -> Resultado<()> {
        self.conn.execute(
            "UPDATE caso_catalogo SET ultimo_acesso_em = ?1 WHERE caso_id = ?2",
            params![Utc::now().to_rfc3339(), caso_id.to_string()],
        )?;
        Ok(())
    }

    pub fn pasta_do_caso(&self, caso_id: CasoId) -> Resultado<Option<PathBuf>> {
        Ok(self
            .conn
            .query_row(
                "SELECT pasta FROM caso_catalogo WHERE caso_id = ?1",
                [caso_id.to_string()],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .map(PathBuf::from))
    }

    /// Remove somente a referência desta estação. A pasta do caso e todo o seu
    /// conteúdo permanecem intocados e podem ser catalogados novamente ao abrir.
    pub fn remover_caso(&self, caso_id: CasoId) -> Resultado<bool> {
        let removidos = self.conn.execute(
            "DELETE FROM caso_catalogo WHERE caso_id = ?1",
            [caso_id.to_string()],
        )?;
        Ok(removidos > 0)
    }

    /// Casos mais recentemente acessados primeiro.
    pub fn listar_casos(&self) -> Resultado<Vec<CasoCatalogado>> {
        let mut stmt = self.conn.prepare(
            "SELECT caso_id, pasta, numero_procedimento, numero_bo, unidade, responsavel_analise,
                    criado_em, ultimo_acesso_em
             FROM caso_catalogo
             ORDER BY COALESCE(ultimo_acesso_em, registrado_em) DESC",
        )?;
        let linhas = stmt.query_map([], |r| {
            let id: String = r.get(0)?;
            let pasta: String = r.get(1)?;
            Ok((
                id,
                pasta,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })?;
        let mut casos = Vec::new();
        for l in linhas {
            let (
                id,
                pasta,
                numero_procedimento,
                numero_bo,
                unidade,
                responsavel_analise,
                criado_em,
                ultimo_acesso_em,
            ) = l?;
            let caso_id = id.parse().map_err(|_| {
                crate::ErroArmazenamento::Inconsistente(format!(
                    "id de caso inválido no catálogo: {id}"
                ))
            })?;
            let disponivel = Path::new(&pasta).join(NOME_ARQUIVO_CASO).is_file();
            casos.push(CasoCatalogado {
                caso_id,
                pasta,
                numero_procedimento,
                numero_bo,
                unidade,
                responsavel_analise,
                criado_em,
                ultimo_acesso_em,
                disponivel,
            });
        }
        Ok(casos)
    }

    /// Grava o WAL no arquivo principal. Chamado ao encerrar o aplicativo, que pode
    /// terminar o processo sem liberar a conexão.
    pub fn consolidar(&self) -> Resultado<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    pub fn configuracao(&self, chave: &str) -> Resultado<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT valor FROM configuracao WHERE chave = ?1",
                [chave],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn definir_configuracao(&self, chave: &str, valor: &str) -> Resultado<()> {
        self.conn.execute(
            "INSERT INTO configuracao(chave, valor, alterado_em) VALUES (?1, ?2, ?3)
             ON CONFLICT(chave) DO UPDATE SET valor = excluded.valor, alterado_em = excluded.alterado_em",
            params![chave, valor, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Lista o manifesto de provedores sem tocar nos caminhos configurados.
    pub fn listar_provedores(&self) -> Resultado<Vec<ProvedorEstacao>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, nome, tipo, modo, habilitado, caminho, versao, observacao,
                    criado_em, atualizado_em
             FROM provedor_estacao ORDER BY nome COLLATE NOCASE, id",
        )?;
        let linhas = stmt.query_map([], |r| {
            Ok(ProvedorEstacao {
                id: r.get(0)?,
                nome: r.get(1)?,
                tipo: r.get(2)?,
                modo: r.get(3)?,
                habilitado: r.get::<_, i64>(4)? != 0,
                caminho: r.get(5)?,
                versao: r.get(6)?,
                observacao: r.get(7)?,
                criado_em: r.get(8)?,
                atualizado_em: r.get(9)?,
            })
        })?;
        linhas
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Registra ou atualiza a configuração de um provedor.
    ///
    /// A regra de segurança é deliberadamente simples: somente provedores
    /// `local` podem ser habilitados. Provedores `externo` ficam desabilitados
    /// até que exista uma ação explícita de integração e política de saída.
    pub fn salvar_provedor(&self, entrada: NovoProvedorEstacao) -> Resultado<ProvedorEstacao> {
        let id = entrada.id.trim();
        let nome = entrada.nome.trim();
        let tipo = entrada.tipo.trim();
        let modo = entrada.modo.trim();
        if id.is_empty()
            || id.len() > 80
            || nome.is_empty()
            || nome.len() > 160
            || tipo.is_empty()
            || tipo.len() > 80
        {
            return Err(crate::ErroArmazenamento::Recusado(
                "id, nome e tipo do provedor são obrigatórios e devem ser curtos".into(),
            ));
        }
        if modo != "local" && modo != "externo" {
            return Err(crate::ErroArmazenamento::Recusado(
                "modo do provedor deve ser local ou externo".into(),
            ));
        }
        if modo == "externo" && entrada.habilitado {
            return Err(crate::ErroArmazenamento::Recusado(
                "provedores externos permanecem desabilitados nesta versão".into(),
            ));
        }
        let agora = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO provedor_estacao
                (id, nome, tipo, modo, habilitado, caminho, versao, observacao, criado_em, atualizado_em)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(id) DO UPDATE SET
                nome = excluded.nome, tipo = excluded.tipo, modo = excluded.modo,
                habilitado = excluded.habilitado, caminho = excluded.caminho,
                versao = excluded.versao, observacao = excluded.observacao,
                atualizado_em = excluded.atualizado_em",
            params![
                id,
                nome,
                tipo,
                modo,
                i64::from(entrada.habilitado),
                entrada.caminho.as_deref().map(str::trim).filter(|v| !v.is_empty()),
                entrada.versao.as_deref().map(str::trim).filter(|v| !v.is_empty()),
                entrada.observacao.as_deref().map(str::trim).filter(|v| !v.is_empty()),
                agora,
            ],
        )?;
        self.conn
            .query_row(
                "SELECT id, nome, tipo, modo, habilitado, caminho, versao, observacao,
                        criado_em, atualizado_em
                 FROM provedor_estacao WHERE id = ?1",
                [id],
                |r| {
                    Ok(ProvedorEstacao {
                        id: r.get(0)?,
                        nome: r.get(1)?,
                        tipo: r.get(2)?,
                        modo: r.get(3)?,
                        habilitado: r.get::<_, i64>(4)? != 0,
                        caminho: r.get(5)?,
                        versao: r.get(6)?,
                        observacao: r.get(7)?,
                        criado_em: r.get(8)?,
                        atualizado_em: r.get(9)?,
                    })
                },
            )
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn provedor_externo_nunca_e_habilitado_automaticamente() {
        let pasta = tempfile::tempdir().unwrap();
        let catalogo = CatalogoGlobal::abrir(pasta.path()).unwrap();
        let erro = catalogo
            .salvar_provedor(NovoProvedorEstacao {
                id: "imei-provider".into(),
                nome: "Consulta IMEI externa".into(),
                tipo: "imei_enrichment".into(),
                modo: "externo".into(),
                habilitado: true,
                caminho: None,
                versao: None,
                observacao: None,
            })
            .unwrap_err();
        assert!(erro.to_string().contains("desabilitados"));
    }

    #[test]
    fn provedor_local_e_persistido_sem_segredo() {
        let pasta = tempfile::tempdir().unwrap();
        let catalogo = CatalogoGlobal::abrir(pasta.path()).unwrap();
        let salvo = catalogo
            .salvar_provedor(NovoProvedorEstacao {
                id: "ocr-local".into(),
                nome: "OCR local".into(),
                tipo: "ocr".into(),
                modo: "local".into(),
                habilitado: true,
                caminho: Some("C:\\Ferramentas\\ocr.exe".into()),
                versao: Some("1.2".into()),
                observacao: Some("instalação da estação".into()),
            })
            .unwrap();
        assert!(salvo.habilitado);
        assert_eq!(catalogo.listar_provedores().unwrap(), vec![salvo]);
    }
}
