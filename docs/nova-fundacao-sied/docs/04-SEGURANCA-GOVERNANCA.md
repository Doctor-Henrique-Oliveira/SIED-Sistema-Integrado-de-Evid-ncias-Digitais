# Segurança, privacidade e governança

## 1. Objetivos de segurança

1. Somente pessoas autorizadas acessam o sistema e os dados permitidos.
2. Evidência original não é alterada pelo SIED.
3. Alterações administrativas relevantes são atribuíveis e auditáveis.
4. Conteúdo forense não é enviado a terceiros por padrão.
5. Segredos não aparecem no código, logs, relatórios ou banco local em texto aberto sem decisão explícita.
6. Comprometimento de uma estação não concede automaticamente acesso administrativo total ao backend.
7. Comprometimento do backend administrativo não entrega automaticamente todas as extrações locais.

## 2. Fronteiras de confiança

- usuário e estação Windows;
- processo webview da interface;
- processo Rust/Tauri;
- pasta e banco do caso;
- sistema operacional e armazenamento de credenciais;
- rede;
- Supabase Auth/API/Postgres/Storage;
- catálogo IMEI;
- ferramentas externas de extração;
- mecanismo de atualização do aplicativo.

Cada travessia precisa de validação de entrada, autorização e tratamento de erro.

## 3. Ameaças prioritárias

| Ameaça | Exemplo | Controles necessários |
|---|---|---|
| Exposição de credencial | token em log ou `.env` versionado | armazenamento seguro, mascaramento, secret scanning |
| Bypass de autorização | interface esconde botão, API aceita chamada | RLS e autorização no servidor |
| Vazamento de evidência | upload automático de anexos | bloqueio por arquitetura e ação explícita |
| Alteração de fonte | parser escreve na extração | acesso somente leitura, inventário e hash |
| Confusão de origem | mensagem sem referência verificável | referência de origem obrigatória |
| Escalada por admin técnico | admin lê caso sigiloso sem autorização | papéis separados e menor privilégio |
| Injeção/arquivo malicioso | nome, JSON, banco ou documento hostil | limites, parsers defensivos, sem execução |
| Supply chain | dependência/atualização comprometida | lockfiles, revisão, SBOM, assinatura |
| Perda de estação | disco ou backup exposto | criptografia e política de chave |
| Auditoria manipulada | exclusão ou alteração de eventos | append-only, RLS, retenção e exportação assinada quando necessário |

## 4. Autenticação

- utilizar Supabase Auth somente para módulos conectados e identidade institucional;
- exigir verificação de e-mail institucional quando disponível;
- exigir MFA para perfis privilegiados e considerar MFA para todos;
- sessões com expiração e revogação;
- registrar login, falha, logout, revogação e mudança relevante de contexto;
- nunca incluir `service_role` no aplicativo;
- a chave pública/anon não substitui RLS;
- não manter senha no aplicativo.

## 5. Autorização

Autorização deve ser aplicada em três níveis:

1. **navegação:** evita confusão, mas não é controle suficiente;
2. **serviço/API:** valida ação e contexto;
3. **banco/RLS:** última barreira contra leitura ou alteração indevida.

Regras precisam considerar:

- unidade;
- status da conta;
- papel;
- função institucional;
- sigilo do procedimento;
- atribuição à equipe;
- ação específica;
- eventual delegação temporária;
- separação entre metadado administrativo e evidência.

## 6. Segredos

| Segredo | Local recomendado |
|---|---|
| service role e chaves de deploy | CI/servidor, nunca no app |
| token de sessão | armazenamento seguro do sistema operacional ou mecanismo Tauri revisado |
| credencial de assinatura | infraestrutura de release isolada |
| chaves de banco local | cofre do sistema/credencial do usuário, após ADR |
| variáveis de desenvolvimento | `.env.local` ignorado |

Criar `.env.example` apenas com nomes e valores fictícios.

## 7. Criptografia

- TLS para todo tráfego remoto;
- criptografia do provedor para dados remotos, sem assumir que isso resolve autorização;
- decidir criptografia em repouso para banco e anexos locais antes do piloto real;
- não criar criptografia caseira;
- documentar rotação, recuperação e perda de chave;
- hashes de integridade não substituem criptografia.

## 8. Auditoria

Evento mínimo:

```text
event_id
occurred_at UTC
actor_id ou ator local
unit_id quando aplicável
action
target_type
target_id
result
correlation_id
workstation_id pseudônimo
metadata minimizada
```

Não registrar corpo de mensagem, conteúdo de documento, senha, token ou termos de pesquisa em log técnico centralizado.

Eventos prioritários:

- autenticação e autorização;
- criação, edição e mudança de sigilo;
- vínculo ou desvínculo de caso forense;
- importação iniciada/concluída/falha/cancelada;
- seleção e remoção de evidência;
- geração e publicação de relatório;
- importação/ativação de catálogo IMEI;
- exportação;
- mudança de papel/permissão;
- ação administrativa destrutiva.

## 9. Privacidade e minimização

- coletar somente o necessário para a finalidade definida;
- evitar caminhos locais e nomes de usuário no backend;
- separar dados de contato de credenciais;
- limitar prévias e notificações em tela bloqueada;
- ocultar conteúdo sensível em telemetria;
- permitir classificação de sigilo;
- definir retenção com responsável institucional;
- não usar dados operacionais para treinar modelos sem autorização específica.

## 10. Arquivos e parsers hostis

- impor limites de tamanho, profundidade, quantidade e tempo;
- bloquear travessia de diretórios e links inseguros;
- não executar macros, scripts ou binários encontrados;
- tratar nomes e metadados como dados não confiáveis;
- preferir leitura streaming;
- manter cancelamento cooperativo;
- criar fixtures malformadas e testes de regressão;
- registrar “não interpretado” sem fingir sucesso.

## 11. Desenvolvimento seguro

- branch principal protegida;
- revisão antes de merge;
- CI com lint, typecheck, testes Rust/TS, build e verificação de migrations;
- análise de dependências e segredos;
- atualizações de dependências em PRs pequenos;
- builds reproduzíveis na medida possível;
- SBOM por release;
- binários e instaladores assinados;
- changelog e versão;
- canal de reporte em `SECURITY.md`.

## 12. Supabase

Antes de produção:

- RLS habilitada em todas as tabelas expostas;
- políticas positivas e testadas;
- funções `security definer` mínimas, com `search_path` fixado;
- migrations revisadas e nunca editadas depois de aplicadas;
- Storage com buckets privados e políticas explícitas;
- backups e restauração testados;
- ambientes separados;
- nenhum dado real no projeto de desenvolvimento;
- logs e retenção revisados.

## 13. Tauri

- allowlist/capabilities mínimas;
- comandos com validação de todos os parâmetros;
- caminhos canônicos e confinados ao caso autorizado;
- nenhuma API genérica de leitura/gravação arbitrária;
- CSP restritiva;
- navegação externa controlada;
- atualização assinada;
- comandos destrutivos com confirmação e auditoria;
- frontend não é fronteira confiável.

## 14. Resposta a incidente

O plano operacional precisa responder:

- quem pode revogar contas e sessões;
- como isolar estação comprometida;
- como preservar logs;
- como verificar integridade de casos;
- como trocar segredos;
- como interromper atualização defeituosa;
- como comunicar o incidente;
- como restaurar sem apagar vestígios necessários.

## 15. Gate para dados reais

Nenhum dado real deve entrar antes de existir:

- responsável institucional designado;
- matriz de acesso aprovada;
- ambiente de produção separado;
- RLS testada;
- backup/restauração testados;
- armazenamento local e política de chave definidos;
- trilha de auditoria verificada;
- termos de uso e procedimento operacional;
- estação e usuário piloto autorizados;
- plano de incidente.
