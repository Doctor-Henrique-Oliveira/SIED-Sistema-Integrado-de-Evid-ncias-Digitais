//! Banco global da estação: catálogo de casos e configurações.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sied_dominio::{Caso, CasoId};
use std::path::{Path, PathBuf};
use ts_rs::TS;

use crate::caso::NOME_ARQUIVO_CASO;
use crate::conexao::{self, Abertura};
use crate::migracoes::{self, BANCO_GLOBAL};
use crate::Resultado;
use crate::{auditoria_estacao, ErroArmazenamento, Sessao};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const NOME_ARQUIVO_GLOBAL: &str = "sied-global.db";
pub const ACAO_EXECUTAR_PROVEDOR: &str = "executar_provedor";
pub const ACAO_ENRIQUECER_IMEI: &str = "enriquecer_imei";
pub const ACAO_CRIAR_BACKUP: &str = "criar_backup";
pub const ACAO_RESTAURAR_BACKUP: &str = "restaurar_backup";
const MAX_SAIDA_PROVEDOR: usize = 2_000;
const MAX_ARQUIVO_IMEI: u64 = 4 * 1024 * 1024;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PermissaoEstacao {
    pub usuario: String,
    pub acao: String,
    pub permitido: bool,
    pub criado_em: String,
    pub atualizado_em: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NovaPermissaoEstacao {
    pub usuario: String,
    pub acao: String,
    pub permitido: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultadoExecucaoProvedor {
    pub provedor_id: String,
    pub acao: String,
    pub sucesso: bool,
    pub codigo_saida: Option<i32>,
    pub saida: String,
    pub erro: Option<String>,
    pub duracao_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultadoImeiLocal {
    pub imei: String,
    pub provedor_id: String,
    pub provedor_versao: Option<String>,
    pub encontrado: bool,
    pub fabricante: Option<String>,
    pub modelo: Option<String>,
    pub fonte: String,
    pub observacao: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultadoBackupEstacao {
    pub caminho: String,
    pub sha256: String,
    pub componentes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultadoRestauracaoEstacao {
    pub origem: String,
    pub provedores_restaurados: u64,
    pub permissoes_restauradas: u64,
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

    pub fn listar_permissoes(&self) -> Resultado<Vec<PermissaoEstacao>> {
        let mut stmt = self.conn.prepare(
            "SELECT usuario, acao, permitido, criado_em, atualizado_em
             FROM permissao_estacao ORDER BY usuario COLLATE NOCASE, acao",
        )?;
        let linhas = stmt.query_map([], |r| {
            Ok(PermissaoEstacao {
                usuario: r.get(0)?,
                acao: r.get(1)?,
                permitido: r.get::<_, i64>(2)? != 0,
                criado_em: r.get(3)?,
                atualizado_em: r.get(4)?,
            })
        })?;
        Ok(linhas.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn salvar_permissao(
        &self,
        entrada: NovaPermissaoEstacao,
        sessao: &Sessao,
    ) -> Resultado<PermissaoEstacao> {
        let usuario = entrada.usuario.trim();
        let acao = entrada.acao.trim();
        if usuario.is_empty() || usuario.len() > 160 || !acao_permitida(acao) {
            return Err(ErroArmazenamento::Recusado(
                "usuário ou ação de estação inválidos".into(),
            ));
        }
        let agora = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO permissao_estacao (usuario, acao, permitido, criado_em, atualizado_em)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(usuario, acao) DO UPDATE SET permitido = excluded.permitido,
                 atualizado_em = excluded.atualizado_em",
            params![usuario, acao, i64::from(entrada.permitido), agora],
        )?;
        let permissao = self.conn.query_row(
            "SELECT usuario, acao, permitido, criado_em, atualizado_em
                 FROM permissao_estacao WHERE usuario = ?1 AND acao = ?2",
            params![usuario, acao],
            |r| {
                Ok(PermissaoEstacao {
                    usuario: r.get(0)?,
                    acao: r.get(1)?,
                    permitido: r.get::<_, i64>(2)? != 0,
                    criado_em: r.get(3)?,
                    atualizado_em: r.get(4)?,
                })
            },
        )?;
        auditoria_estacao::registrar(
            &self.conn,
            sessao,
            "permissao_estacao_atualizada",
            json!({ "usuario": usuario, "acao": acao, "permitido": entrada.permitido }),
        )?;
        Ok(permissao)
    }

    pub fn listar_auditoria_estacao(
        &self,
        limite: i64,
    ) -> Resultado<Vec<crate::auditoria::RegistroAuditoria>> {
        auditoria_estacao::listar(&self.conn, limite.clamp(1, 10_000))
    }

    pub fn verificar_auditoria_estacao(&self) -> Resultado<crate::auditoria::VerificacaoAuditoria> {
        auditoria_estacao::verificar(&self.conn)
    }

    fn tem_permissao(&self, usuario: &str, acao: &str) -> Resultado<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT permitido FROM permissao_estacao WHERE usuario = ?1 AND acao = ?2",
                params![usuario, acao],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0)
            != 0)
    }

    pub fn executar_provedor(
        &self,
        provedor_id: &str,
        sessao: &Sessao,
    ) -> Resultado<ResultadoExecucaoProvedor> {
        self.exigir_permissao(sessao, ACAO_EXECUTAR_PROVEDOR)?;
        let provedor = self.provedor(provedor_id)?;
        if !provedor.habilitado || provedor.modo != "local" {
            return Err(ErroArmazenamento::Recusado(
                "somente provedor local habilitado pode ser executado".into(),
            ));
        }
        let caminho = provedor.caminho.as_deref().ok_or_else(|| {
            ErroArmazenamento::Recusado("provedor sem caminho de executável".into())
        })?;
        let arquivo = Path::new(caminho);
        if !arquivo.is_file() {
            return Err(ErroArmazenamento::NaoEncontrado(format!(
                "executável do provedor {provedor_id}"
            )));
        }
        let inicio = Instant::now();
        let mut processo = Command::new(arquivo)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(ErroArmazenamento::Io)?;
        let mut expirou = false;
        let status = loop {
            if let Some(status) = processo.try_wait().map_err(ErroArmazenamento::Io)? {
                break Some(status);
            }
            if inicio.elapsed() >= Duration::from_secs(2) {
                expirou = true;
                processo.kill().map_err(ErroArmazenamento::Io)?;
                break processo.wait().ok();
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        let mut saida = Vec::new();
        let mut erro = Vec::new();
        if let Some(mut s) = processo.stdout.take() {
            s.read_to_end(&mut saida)?;
        }
        if let Some(mut s) = processo.stderr.take() {
            s.read_to_end(&mut erro)?;
        }
        let sucesso = !expirou
            && status
                .as_ref()
                .is_some_and(std::process::ExitStatus::success);
        let resultado = ResultadoExecucaoProvedor {
            provedor_id: provedor.id.clone(),
            acao: "--version".into(),
            sucesso,
            codigo_saida: status.and_then(|s| s.code()),
            saida: limitar_saida(&saida),
            erro: if expirou {
                Some("tempo limite de 2 segundos excedido; processo encerrado".into())
            } else {
                let valor = limitar_saida(&erro);
                (!valor.is_empty()).then_some(valor)
            },
            duracao_ms: inicio.elapsed().as_millis() as u64,
        };
        auditoria_estacao::registrar(
            &self.conn,
            sessao,
            "provedor_executado",
            json!({
                "provedor_id": resultado.provedor_id,
                "acao": resultado.acao,
                "sucesso": resultado.sucesso,
                "codigo_saida": resultado.codigo_saida,
                "duracao_ms": resultado.duracao_ms,
            }),
        )?;
        Ok(resultado)
    }

    pub fn enriquecer_imei_local(
        &self,
        imei: &str,
        sessao: &Sessao,
    ) -> Resultado<ResultadoImeiLocal> {
        self.exigir_permissao(sessao, ACAO_ENRIQUECER_IMEI)?;
        let imei = imei.trim();
        if imei.len() != 15 || !imei.chars().all(|c| c.is_ascii_digit()) {
            return Err(ErroArmazenamento::Recusado(
                "IMEI deve conter exatamente 15 dígitos".into(),
            ));
        }
        let provedor = self
            .listar_provedores()?
            .into_iter()
            .find(|p| p.tipo == "imei_enrichment" && p.modo == "local" && p.habilitado)
            .ok_or_else(|| ErroArmazenamento::NaoEncontrado("provedor local de IMEI".into()))?;
        let caminho = provedor.caminho.clone().ok_or_else(|| {
            ErroArmazenamento::Recusado("provedor local de IMEI sem arquivo de dados".into())
        })?;
        let arquivo = Path::new(&caminho);
        let meta = std::fs::metadata(arquivo)?;
        if !meta.is_file() || meta.len() > MAX_ARQUIVO_IMEI {
            return Err(ErroArmazenamento::Recusado(
                "arquivo do provedor de IMEI inválido ou maior que 4 MiB".into(),
            ));
        }
        let mut texto = String::new();
        std::fs::File::open(arquivo)?
            .take(MAX_ARQUIVO_IMEI)
            .read_to_string(&mut texto)?;
        let (encontrado, fabricante, modelo) = localizar_imei(&texto, imei);
        let resultado = ResultadoImeiLocal {
            imei: imei.into(),
            provedor_id: provedor.id,
            provedor_versao: provedor.versao,
            encontrado,
            fabricante,
            modelo,
            fonte: caminho,
            observacao: if encontrado {
                "correspondência encontrada em arquivo local; não é conclusão pericial".into()
            } else {
                "nenhuma correspondência encontrada no arquivo local".into()
            },
        };
        auditoria_estacao::registrar(
            &self.conn,
            sessao,
            "imei_enriquecido_local",
            json!({ "imei": resultado.imei, "provedor_id": resultado.provedor_id, "encontrado": resultado.encontrado }),
        )?;
        Ok(resultado)
    }

    pub fn criar_backup(
        &self,
        destino: &Path,
        sessao: &Sessao,
    ) -> Resultado<ResultadoBackupEstacao> {
        self.exigir_permissao(sessao, ACAO_CRIAR_BACKUP)?;
        if destino == self.caminho || destino.exists() {
            return Err(ErroArmazenamento::Recusado(
                "escolha um arquivo de backup novo, fora do catálogo ativo".into(),
            ));
        }
        if let Some(parent) = destino.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.conn.execute(
            "SELECT 1", // mantém a validação do banco antes de abrir o destino
            [],
        )?;
        auditoria_estacao::registrar(
            &self.conn,
            sessao,
            "backup_estacao_solicitado",
            json!({ "destino": destino.display().to_string() }),
        )?;
        if let Err(e) = self
            .conn
            .execute("VACUUM INTO ?1", [destino.to_string_lossy()])
        {
            let _ = auditoria_estacao::registrar(
                &self.conn,
                sessao,
                "backup_estacao_falhou",
                json!({ "destino": destino.display().to_string() }),
            );
            return Err(e.into());
        }
        let resultado = ResultadoBackupEstacao {
            caminho: destino.display().to_string(),
            sha256: sha256_arquivo(destino)?,
            componentes: vec!["catálogo global e configurações da estação".into()],
        };
        auditoria_estacao::registrar(
            &self.conn,
            sessao,
            "backup_estacao_concluido",
            json!({ "caminho": resultado.caminho, "sha256": resultado.sha256 }),
        )?;
        Ok(resultado)
    }

    pub fn restaurar_backup(
        &self,
        origem: &Path,
        sessao: &Sessao,
    ) -> Resultado<ResultadoRestauracaoEstacao> {
        self.exigir_permissao(sessao, ACAO_RESTAURAR_BACKUP)?;
        let leitura = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
        let externo = Connection::open_with_flags(origem, leitura)?;
        let tem_provedores = tabela_existe(&externo, "provedor_estacao")?;
        let tem_permissoes = tabela_existe(&externo, "permissao_estacao")?;
        if !tem_provedores || !tem_permissoes {
            return Err(ErroArmazenamento::Recusado(
                "o arquivo não é um backup global compatível do SIED".into(),
            ));
        }
        let provedores = ler_provedores(&externo)?;
        let permissoes = ler_permissoes(&externo)?;
        if provedores.iter().any(|p| {
            p.id.trim().is_empty()
                || p.id.len() > 80
                || p.nome.trim().is_empty()
                || p.nome.len() > 160
                || !matches!(p.modo.as_str(), "local" | "externo")
                || (p.modo == "externo" && p.habilitado)
        }) || permissoes.iter().any(|p| {
            p.usuario.trim().is_empty() || p.usuario.len() > 160 || !acao_permitida(&p.acao)
        }) {
            return Err(ErroArmazenamento::Recusado(
                "o backup contém manifesto ou permissões inválidos".into(),
            ));
        }
        let tx = self.conn.unchecked_transaction()?;
        for p in &provedores {
            tx.execute(
                "INSERT INTO provedor_estacao
                 (id, nome, tipo, modo, habilitado, caminho, versao, observacao, criado_em, atualizado_em)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET nome=excluded.nome, tipo=excluded.tipo,
                 modo=excluded.modo, habilitado=excluded.habilitado, caminho=excluded.caminho,
                 versao=excluded.versao, observacao=excluded.observacao, atualizado_em=excluded.atualizado_em",
                params![p.id, p.nome, p.tipo, p.modo, i64::from(p.habilitado), p.caminho, p.versao, p.observacao, p.criado_em, p.atualizado_em],
            )?;
        }
        for p in &permissoes {
            tx.execute(
                "INSERT INTO permissao_estacao (usuario, acao, permitido, criado_em, atualizado_em)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(usuario, acao) DO UPDATE SET permitido=excluded.permitido,
                 atualizado_em=excluded.atualizado_em",
                params![
                    p.usuario,
                    p.acao,
                    i64::from(p.permitido),
                    p.criado_em,
                    p.atualizado_em
                ],
            )?;
        }
        tx.commit()?;
        let resultado = ResultadoRestauracaoEstacao {
            origem: origem.display().to_string(),
            provedores_restaurados: provedores.len() as u64,
            permissoes_restauradas: permissoes.len() as u64,
        };
        auditoria_estacao::registrar(
            &self.conn,
            sessao,
            "backup_estacao_restaurado",
            json!({ "origem": resultado.origem, "provedores": resultado.provedores_restaurados, "permissoes": resultado.permissoes_restauradas }),
        )?;
        Ok(resultado)
    }

    fn exigir_permissao(&self, sessao: &Sessao, acao: &str) -> Resultado<()> {
        if !self.tem_permissao(&sessao.usuario, acao)? {
            return Err(ErroArmazenamento::Recusado(format!(
                "usuário {} não possui a permissão local {acao}",
                sessao.usuario
            )));
        }
        Ok(())
    }

    fn provedor(&self, id: &str) -> Resultado<ProvedorEstacao> {
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
            .optional()?
            .ok_or_else(|| ErroArmazenamento::NaoEncontrado(format!("provedor {id}")))
    }
}

fn acao_permitida(acao: &str) -> bool {
    matches!(
        acao,
        ACAO_EXECUTAR_PROVEDOR | ACAO_ENRIQUECER_IMEI | ACAO_CRIAR_BACKUP | ACAO_RESTAURAR_BACKUP
    )
}

fn limitar_saida(bytes: &[u8]) -> String {
    let texto = String::from_utf8_lossy(bytes);
    let mut valor: String = texto.chars().take(MAX_SAIDA_PROVEDOR).collect();
    if texto.chars().count() > MAX_SAIDA_PROVEDOR {
        valor.push('…');
    }
    valor
}

fn sha256_arquivo(caminho: &Path) -> Resultado<String> {
    let mut arquivo = std::fs::File::open(caminho)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = arquivo.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}

fn tabela_existe(conn: &Connection, nome: &str) -> Resultado<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1",
            [nome],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn ler_provedores(conn: &Connection) -> Resultado<Vec<ProvedorEstacao>> {
    let mut stmt = conn.prepare(
        "SELECT id, nome, tipo, modo, habilitado, caminho, versao, observacao,
                criado_em, atualizado_em FROM provedor_estacao ORDER BY id",
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
    Ok(linhas.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn ler_permissoes(conn: &Connection) -> Resultado<Vec<PermissaoEstacao>> {
    let mut stmt = conn.prepare(
        "SELECT usuario, acao, permitido, criado_em, atualizado_em FROM permissao_estacao",
    )?;
    let linhas = stmt.query_map([], |r| {
        Ok(PermissaoEstacao {
            usuario: r.get(0)?,
            acao: r.get(1)?,
            permitido: r.get::<_, i64>(2)? != 0,
            criado_em: r.get(3)?,
            atualizado_em: r.get(4)?,
        })
    })?;
    Ok(linhas.collect::<std::result::Result<Vec<_>, _>>()?)
}

fn localizar_imei(texto: &str, imei: &str) -> (bool, Option<String>, Option<String>) {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(texto) {
        let itens = json
            .as_array()
            .cloned()
            .or_else(|| json.get("items").and_then(|v| v.as_array()).cloned())
            .unwrap_or_default();
        for item in itens {
            let Some(valor) = item.get("imei").and_then(|v| v.as_str()) else {
                continue;
            };
            if valor.trim() == imei {
                return (
                    true,
                    item.get("manufacturer")
                        .or_else(|| item.get("fabricante"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                    item.get("model")
                        .or_else(|| item.get("modelo"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                );
            }
        }
        return (false, None, None);
    }
    let mut linhas = texto.lines();
    let cabecalho: Vec<&str> = linhas
        .next()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .collect();
    let idx_imei = cabecalho
        .iter()
        .position(|c| c.eq_ignore_ascii_case("imei"));
    let idx_fab = cabecalho
        .iter()
        .position(|c| matches!(*c, "manufacturer" | "fabricante"));
    let idx_modelo = cabecalho
        .iter()
        .position(|c| matches!(*c, "model" | "modelo"));
    for linha in linhas {
        let campos: Vec<&str> = linha.split(',').map(str::trim).collect();
        if idx_imei.and_then(|i| campos.get(i)).copied() == Some(imei) {
            return (
                true,
                idx_fab
                    .and_then(|i| campos.get(i))
                    .map(|v| (*v).to_string()),
                idx_modelo
                    .and_then(|i| campos.get(i))
                    .map(|v| (*v).to_string()),
            );
        }
    }
    (false, None, None)
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::fs;

    fn sessao() -> Sessao {
        Sessao {
            usuario: "analista-sintetico".into(),
            maquina: "estacao-sintetica".into(),
        }
    }

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

    #[test]
    fn permissoes_sao_explicitas_e_auditadas() {
        let pasta = tempfile::tempdir().unwrap();
        let catalogo = CatalogoGlobal::abrir(pasta.path()).unwrap();
        assert!(catalogo
            .executar_provedor("inexistente", &sessao())
            .is_err());
        let salvo = catalogo
            .salvar_permissao(
                NovaPermissaoEstacao {
                    usuario: "analista-sintetico".into(),
                    acao: ACAO_EXECUTAR_PROVEDOR.into(),
                    permitido: true,
                },
                &sessao(),
            )
            .unwrap();
        assert!(salvo.permitido);
        assert_eq!(catalogo.listar_auditoria_estacao(10).unwrap().len(), 1);
        assert_eq!(catalogo.verificar_auditoria_estacao().unwrap().registros, 1);
    }

    #[test]
    fn imei_local_le_apenas_o_arquivo_configurado() {
        let pasta = tempfile::tempdir().unwrap();
        let dados = pasta.path().join("imei.json");
        fs::write(
            &dados,
            r#"[{"imei":"490154203237518","manufacturer":"SINTÉTICO","model":"Modelo A"}]"#,
        )
        .unwrap();
        let catalogo = CatalogoGlobal::abrir(pasta.path()).unwrap();
        catalogo
            .salvar_provedor(NovoProvedorEstacao {
                id: "imei-local".into(),
                nome: "Tabela IMEI sintética".into(),
                tipo: "imei_enrichment".into(),
                modo: "local".into(),
                habilitado: true,
                caminho: Some(dados.display().to_string()),
                versao: Some("fixture-1".into()),
                observacao: None,
            })
            .unwrap();
        catalogo
            .salvar_permissao(
                NovaPermissaoEstacao {
                    usuario: "analista-sintetico".into(),
                    acao: ACAO_ENRIQUECER_IMEI.into(),
                    permitido: true,
                },
                &sessao(),
            )
            .unwrap();
        let resultado = catalogo
            .enriquecer_imei_local("490154203237518", &sessao())
            .unwrap();
        assert!(resultado.encontrado);
        assert_eq!(resultado.fabricante.as_deref(), Some("SINTÉTICO"));
        assert_eq!(resultado.modelo.as_deref(), Some("Modelo A"));
    }

    #[test]
    fn backup_e_restauracao_fazem_merge_sem_apagar_catalogo() {
        let pasta = tempfile::tempdir().unwrap();
        let catalogo = CatalogoGlobal::abrir(pasta.path()).unwrap();
        let s = sessao();
        catalogo
            .salvar_permissao(
                NovaPermissaoEstacao {
                    usuario: s.usuario.clone(),
                    acao: ACAO_CRIAR_BACKUP.into(),
                    permitido: true,
                },
                &s,
            )
            .unwrap();
        catalogo
            .salvar_provedor(NovoProvedorEstacao {
                id: "local-a".into(),
                nome: "Provider A".into(),
                tipo: "ocr".into(),
                modo: "local".into(),
                habilitado: false,
                caminho: None,
                versao: None,
                observacao: None,
            })
            .unwrap();
        let backup = pasta.path().join("backup.db");
        let criado = catalogo.criar_backup(&backup, &s).unwrap();
        assert_eq!(criado.sha256.len(), 64);

        let outra = tempfile::tempdir().unwrap();
        let destino = CatalogoGlobal::abrir(outra.path()).unwrap();
        destino
            .salvar_permissao(
                NovaPermissaoEstacao {
                    usuario: s.usuario.clone(),
                    acao: ACAO_RESTAURAR_BACKUP.into(),
                    permitido: true,
                },
                &s,
            )
            .unwrap();
        let restaurado = destino.restaurar_backup(&backup, &s).unwrap();
        assert_eq!(restaurado.provedores_restaurados, 1);
        assert!(destino
            .listar_provedores()
            .unwrap()
            .iter()
            .any(|p| p.id == "local-a"));
    }
}
