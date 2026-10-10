# Checklist para 15 de outubro de 2026

## 1. Antes de criar contas

- [ ] Confirmar nome da organização e do repositório.
- [ ] Confirmar quem será proprietário e quem terá recuperação administrativa.
- [ ] Definir e-mail institucional ou dedicado.
- [ ] Guardar códigos de recuperação fora do computador de desenvolvimento.
- [ ] Definir se haverá uma ou mais unidades no piloto.
- [ ] Definir responsáveis por produto, segurança e banco.

## 2. GitHub

- [ ] Criar organização em vez de depender apenas de uma conta pessoal.
- [ ] Ativar MFA.
- [ ] Criar repositório privado.
- [ ] Definir visibilidade e membros mínimos.
- [ ] Proteger `main`.
- [ ] Exigir pull request e CI.
- [ ] Bloquear force push e exclusão de branch protegida.
- [ ] Ativar secret scanning e Dependabot, se disponíveis.
- [ ] Criar `CODEOWNERS` para migrations e áreas de segurança.
- [ ] Adicionar `SECURITY.md`, `CONTRIBUTING.md` e política de branches.
- [ ] Não subir dataset, dump, `.env`, certificado ou evidência.

## 3. Supabase

- [ ] Criar organização/projeto de desenvolvimento.
- [ ] Reservar projeto separado para produção; não reutilizar desenvolvimento.
- [ ] Escolher região conforme requisito institucional e jurídico.
- [ ] Configurar MFA dos administradores.
- [ ] Guardar segredos em cofre apropriado.
- [ ] Registrar quem possui acesso ao dashboard.
- [ ] Configurar Auth com política inicial.
- [ ] Criar migrations somente pelo repositório.
- [ ] Habilitar e testar RLS desde a primeira tabela.
- [ ] Preparar seed sintético.
- [ ] Não inserir dados reais na primeira semana.

## 4. Estação de desenvolvimento

- [ ] Confirmar Windows 10/11 atualizado.
- [ ] Instalar Git, Rust stable, Node LTS/pnpm e pré-requisitos do Tauri.
- [ ] Registrar versões em arquivo/toolchain.
- [ ] Configurar assinatura de commits se adotada.
- [ ] Separar diretório de código de diretório de casos/datasets.
- [ ] Garantir espaço em disco para builds Rust.
- [ ] Configurar backup do código sem incluir segredos/evidências.

## 5. Bootstrap do repositório

- [ ] Copiar este pacote documental para `docs/produto`.
- [ ] Criar README mínimo com comandos reais.
- [ ] Criar workspace Rust e aplicativo Tauri.
- [ ] Configurar TypeScript estrito, lint e formatação.
- [ ] Configurar testes Rust e Vitest.
- [ ] Configurar CI Windows e, se útil, Linux para crates puras.
- [ ] Criar `.env.example` fictício.
- [ ] Criar `.gitignore` antes do primeiro build.
- [ ] Confirmar que `git status` não inclui artefatos grandes.
- [ ] Gerar inventário de dependências.

## 6. Primeiras ADRs

- [ ] ADR-001 — arquitetura híbrida local/remota.
- [ ] ADR-002 — separação entre procedimento e caso forense.
- [ ] ADR-003 — identidade, papéis e sigilo.
- [ ] ADR-004 — modo offline e sessão.
- [ ] ADR-005 — armazenamento de tokens/segredos.
- [ ] ADR-006 — estratégia de criptografia local.
- [ ] ADR-007 — origem, licença e atualização do catálogo IMEI.
- [ ] ADR-008 — estratégia de migração do código antigo.
- [ ] ADR-009 — publicação de relatório e metadados.

## 7. Dataset IMEI

- [ ] Localizar o arquivo original fornecido anteriormente ou obter nova cópia autorizada.
- [ ] Não modificar o original.
- [ ] Calcular SHA-256.
- [ ] Registrar tamanho, formato, encoding e origem.
- [ ] Verificar licença/termo de uso.
- [ ] Identificar se é base TAC, IMEI completo ou outra estrutura.
- [ ] Listar colunas e significado conhecido.
- [ ] Contar linhas, duplicatas e chaves vazias.
- [ ] Criar amostra sintética equivalente para o Git.
- [ ] Definir local seguro fora do repositório para o arquivo real.

## 8. Repositórios antigos

- [ ] Congelar referência do SIED atual com hash/arquivo e data.
- [ ] Congelar commit do SIPI usado como origem.
- [ ] Exportar lista de branches/PRs relevantes.
- [ ] Confirmar propriedade e licença do código.
- [ ] Não apagar os repositórios antigos.
- [ ] Marcar como somente manutenção após o início da migração.
- [ ] Registrar issues conhecidas que não devem ser migradas.

## 9. Primeira semana sugerida

### 15/10 — Fundação

- repositório, CI, Tauri abrindo, documentação e ADR-001/002.

### 16/10 — Identidade e módulos

- autenticação sintética/de desenvolvimento, shell e central de módulos.

### 17/10 — Supabase mínimo

- perfis, unidades, memberships e testes de RLS.

### 18/10 — Procedimento mínimo

- migration, repositório e tela simples com dados sintéticos.

### 19/10 — Dispositivo e vínculo

- dispositivo apreendido, IMEI validado e contrato de caso local.

Datas são direção, não promessa. Segurança e correção prevalecem sobre velocidade.

## 10. Gate de encerramento do primeiro dia

Antes de parar em 15/10:

- [ ] clone novo funciona;
- [ ] CI executou ao menos uma vez;
- [ ] app abre;
- [ ] nenhum segredo está no histórico;
- [ ] docs estão no repositório;
- [ ] primeira decisão arquitetural está registrada;
- [ ] próximo passo é pequeno e verificável;
- [ ] não há dado real no projeto.
