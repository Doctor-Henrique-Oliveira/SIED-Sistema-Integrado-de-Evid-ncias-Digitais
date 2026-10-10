# Registro de decisões

Este documento distingue decisão, recomendação e pendência. Ele deve evoluir para ADRs no novo repositório.

## 1. Decisões confirmadas pelo responsável

| ID | Decisão | Data | Consequência |
|---|---|---|---|
| D-001 | Preservar a marca e a abertura visual do SIED | 10/10/2026 | SIED é a identidade do produto integrado |
| D-002 | A aplicação deve abrir inicialmente em módulos | 10/10/2026 | Home não abre diretamente um caso forense |
| D-003 | Módulos principais incluem IMEI, procedimentos, agenda e análise de extração | 10/10/2026 | Define o núcleo da primeira visão do produto |
| D-004 | O projeto novo começa em 15/10/2026 | 10/10/2026 | Preparação documental antecede código |
| D-005 | Contas, banco e GitHub serão novos | 10/10/2026 | Migração é explícita; não herdar configuração acidental |
| D-006 | Documentação deve ser ampla e cuidadosa | 10/10/2026 | Decisões e limitações serão registradas desde o início |

## 2. Recomendações arquiteturais aguardando ratificação

| ID | Recomendação | Motivo | Se rejeitada |
|---|---|---|---|
| R-001 | Aplicativo Tauri único | preserva desktop e núcleo Rust | definir web + aplicativo separado |
| R-002 | Supabase para administrativo e SQLite local para forense | colaboração sem enviar evidência bruta | escolher tudo local ou backend próprio |
| R-003 | Um repositório modular | reduz operação e mantém contratos próximos | justificar monorepo/múltiplos serviços |
| R-004 | Publicação por outbox/eventos específicos | idempotência e auditoria | desenhar sincronização alternativa |
| R-005 | Catálogo IMEI local e versionado | consulta offline e reprodutível | definir serviço externo autorizado |
| R-006 | Migrar comportamento antes de refatorar | reduz regressão | aceitar risco de reescrita ampla |
| R-007 | Somente dados sintéticos até gates de segurança | reduz exposição | autorização formal e controles equivalentes |

## 3. Decisões abertas críticas

### A-001 — Nome expandido e descrição

Manter “Sistema Integrado de Evidências Digitais” ou alterar a expansão da sigla para refletir gestão operacional? Recomendação: manter a expansão no primeiro ciclo e usar uma descrição complementar.

### A-002 — Organização e tenancy

O sistema atenderá somente uma delegacia/unidade ou várias? Isso altera chaves, RLS, administração e relatórios.

### A-003 — Usuários e papéis

Quais funções existirão no piloto? Quem pode ver sigiloso, cadastrar, editar, publicar relatório e administrar usuários?

### A-004 — Offline autenticado

O usuário poderá abrir caso local sem rede? Por quanto tempo? Como revogação remota será respeitada? Qual risco de estação compartilhada?

### A-005 — Criptografia local

Banco e anexos serão criptografados? Como chave será obtida, recuperada e rotacionada? Sem essa decisão não há piloto com dados reais.

### A-006 — Catálogo IMEI

Qual é a fonte, licença, estrutura e frequência de atualização? O arquivo contém TACs ou IMEIs completos? Há dado pessoal/restrito?

### A-007 — Relatórios

O backend guardará somente hash/referência ou também uma cópia do PDF? Quem pode abrir, substituir ou revogar?

### A-008 — Legado

Quais dados reais do SIPI antigo precisam migrar? Casos SIED antigos precisam abrir diretamente no novo aplicativo?

### A-009 — Hospedagem e região

Qual região e contrato do provedor atendem os requisitos institucionais? Quem é controlador e operador dos dados?

### A-010 — Backup e retenção

Quem executa, testa e autoriza restauração? Por quanto tempo procedimentos, logs, catálogos e relatórios ficam armazenados?

### A-011 — Distribuição

Instalador será por usuário ou por máquina? Atualização automática ou administrada? Quem assina os binários?

### A-012 — Escopo da primeira versão

O primeiro marco inclui Agenda ou termina na fatia procedimento → dispositivo → caso → relatório? Recomendação: terminar primeiro a fatia vertical e migrar Agenda em seguida.

## 4. Modelo de ADR

```markdown
# ADR-NNN — Título

Status: proposta | aceita | substituída | rejeitada
Data: AAAA-MM-DD
Responsáveis:

## Contexto
Qual problema exige decisão e quais restrições existem?

## Opções consideradas
Opção A, B e C, com custos e riscos.

## Decisão
O que foi escolhido, sem ambiguidade.

## Consequências
Ganhos, custos, riscos e trabalho posterior.

## Evidência de validação
Testes, protótipo, medição ou fonte que sustenta a decisão.
```

## 5. Política de mudança

- decisão aceita pode mudar;
- mudança cria novo ADR e marca o anterior como substituído;
- documentação nunca deve reescrever o passado para parecer que a decisão sempre foi outra;
- implementação divergente exige atualização ou correção;
- dúvida relevante vira pendência explícita, não suposição silenciosa.
