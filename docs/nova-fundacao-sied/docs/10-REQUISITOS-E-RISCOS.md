# Requisitos e riscos

## 1. Requisitos funcionais prioritários

| ID | Requisito | Prioridade | Fase |
|---|---|---:|---:|
| RF-001 | Autenticar usuário e verificar autorização/unidade | obrigatória | 1 |
| RF-002 | Exibir somente módulos disponíveis ao perfil | obrigatória | 1 |
| RF-003 | Informar estado online/offline sem mascarar falha | obrigatória | 1 |
| RF-004 | Criar, pesquisar, visualizar e editar procedimento autorizado | obrigatória | 2 |
| RF-005 | Controlar sigilo, equipe e histórico do procedimento | obrigatória | 2 |
| RF-006 | Cadastrar dispositivo apreendido e custódia mínima | obrigatória | 2 |
| RF-007 | Normalizar e validar IMEI sem afirmar existência | obrigatória | 2 |
| RF-008 | Importar e versionar catálogo IMEI autorizado | obrigatória | 3 |
| RF-009 | Consultar catálogo e explicar fonte/versão | obrigatória | 3 |
| RF-010 | Criar ou vincular caso forense local | obrigatória | 4 |
| RF-011 | Importar fonte suportada preservando original | obrigatória | 4 |
| RF-012 | Inventariar e calcular hashes | obrigatória | 4 |
| RF-013 | Normalizar registro com referência de origem | obrigatória | 4 |
| RF-014 | Pesquisar e visualizar artefatos suportados | obrigatória | 4 |
| RF-015 | Selecionar evidência e registrar ação | obrigatória | 4 |
| RF-016 | Gerar relatório e hash verificáveis | obrigatória | 4 |
| RF-017 | Publicar metadado autorizado de resultado | obrigatória | 5 |
| RF-018 | Repetir publicação sem duplicidade | obrigatória | 5 |
| RF-019 | Consultar trilhas administrativa e forense dentro do escopo | obrigatória | 5 |
| RF-020 | Criar e gerir agenda/oitiva vinculada | alta | 6 |
| RF-021 | Detectar conflitos de agenda | alta | 6 |
| RF-022 | Produzir lembretes sem expor dado indevido | alta | 6 |

## 2. Requisitos não funcionais

| ID | Requisito | Critério inicial |
|---|---|---|
| RNF-001 | Integridade | fonte não é escrita; hash verificável antes/depois |
| RNF-002 | Rastreabilidade | todo artefato/evidência aponta para origem |
| RNF-003 | Privacidade | conteúdo forense não sai da estação por padrão |
| RNF-004 | Autorização | proteção no servidor/banco, não apenas na UI |
| RNF-005 | Auditabilidade | ações críticas têm ator, alvo, tempo e resultado |
| RNF-006 | Reprodutibilidade | parser/dataset/versionamento registrado |
| RNF-007 | Resiliência | queda de rede não corrompe caso nem finge salvamento |
| RNF-008 | Desempenho | metas definidas por cenário e volume sintético antes de otimizar |
| RNF-009 | Acessibilidade | teclado, foco, contraste e rótulos no fluxo principal |
| RNF-010 | Manutenibilidade | módulos e contratos separados, testes e ADRs |
| RNF-011 | Instalabilidade | procedimento reproduzível em Windows suportado |
| RNF-012 | Recuperação | backup e restauração testados antes do piloto |
| RNF-013 | Segurança de supply chain | lockfiles, revisão, SBOM e assinatura de release |
| RNF-014 | Honestidade funcional | suporte só aparece quando testado |

## 3. Histórias da fatia vertical

### H-001 — Iniciar pelo procedimento

Como investigador autorizado, quero abrir um procedimento e visualizar os dispositivos relacionados para iniciar o trabalho sem repetir contexto.

### H-002 — Consultar o IMEI

Como usuário autorizado, quero consultar um IMEI e ver fonte, versão e limitações para não tratar inferência como fato.

### H-003 — Criar caso forense vinculado

Como analista, quero criar um caso local a partir do dispositivo sem copiar a extração para a nuvem.

### H-004 — Produzir evidência rastreável

Como analista, quero selecionar um registro e gerar relatório que permita conferir arquivo e origem.

### H-005 — Publicar resultado mínimo

Como responsável pelo procedimento, quero receber estado, referência e hash do relatório sem receber automaticamente todo o conteúdo forense.

## 4. Matriz de rastreabilidade inicial

| Resultado do produto | Requisitos | Teste principal |
|---|---|---|
| acesso autorizado | RF-001/002, RNF-004 | integração Auth + RLS |
| procedimento e dispositivo | RF-004–007 | API/RLS + componente |
| consulta explicável | RF-008/009, RNF-006 | importador + snapshot |
| preservação forense | RF-010–016, RNF-001/002/003 | E2E local |
| vínculo sem vazamento | RF-017–019 | E2E híbrido + inspeção do backend |
| agenda | RF-020–022 | regras de conflito/fuso/permissão |

## 5. Registro inicial de riscos

Escala: probabilidade e impacto de 1 a 5. Pontuação é produto e deve ser revista com evidência.

| ID | Risco | P | I | Pontos | Tratamento inicial |
|---|---|---:|---:|---:|---|
| RK-001 | Escopo grande impedir fatia utilizável | 4 | 5 | 20 | fases e gate vertical |
| RK-002 | Mistura de bancos vazar evidência | 3 | 5 | 15 | fronteira e publicação explícita |
| RK-003 | RLS incorreta expor procedimento | 3 | 5 | 15 | testes por papel/unidade/sigilo |
| RK-004 | Reescrita perder capacidades do SIED | 4 | 4 | 16 | migração por componente e testes originais |
| RK-005 | Dataset IMEI sem origem/licença | 4 | 4 | 16 | bloquear uso até proveniência |
| RK-006 | Credencial administrativa no desktop | 2 | 5 | 10 | proibir service role no cliente |
| RK-007 | Estação perdida expor caso local | 3 | 5 | 15 | ADR de criptografia e operação |
| RK-008 | Offline enfraquecer revogação | 3 | 4 | 12 | política de validade e bloqueio |
| RK-009 | UI unificada esconder escopos de busca | 3 | 4 | 12 | indicar local/remoto explicitamente |
| RK-010 | Migração de legado corromper dados | 3 | 5 | 15 | simulação, reconciliação e rollback |
| RK-011 | Atualização quebrar casos antigos | 3 | 5 | 15 | compatibility tests e backup |
| RK-012 | Relatórios alegarem além da fonte | 3 | 5 | 15 | origem, revisão e templates testados |
| RK-013 | Dependências comprometerem build | 2 | 5 | 10 | lock, scan, SBOM e assinatura |
| RK-014 | Projeto virar coleção de dashboards | 4 | 3 | 12 | toda tela ligada a tarefa/decisão |
| RK-015 | Módulos secundários atrasarem núcleo | 4 | 4 | 16 | backlog após fase 6 |

## 6. Gatilhos de reavaliação

Revisar arquitetura se ocorrer:

- exigência de armazenar evidência centralmente;
- operação em múltiplas unidades ou estados;
- obrigação de funcionar totalmente offline;
- necessidade de sincronizar um caso entre estações;
- dataset IMEI somente acessível por API externa;
- mudança de provedor/legislação/política institucional;
- volume ou desempenho fora dos cenários medidos;
- incidente ou achado de segurança de alto impacto.

## 7. Critério de priorização

Um item entra antes quando:

1. reduz risco de segurança/integridade;
2. desbloqueia a fatia vertical;
3. atende tarefa frequente comprovada;
4. possui fonte de dados e regra claras;
5. pode ser testado objetivamente.

Beleza visual é importante para uso e confiança, mas não transforma módulo sem dados e contratos em capacidade operacional.
