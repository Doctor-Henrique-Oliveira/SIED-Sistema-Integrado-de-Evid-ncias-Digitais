# Dados e integrações

## 1. Princípio de separação

O novo SIED terá duas autoridades de dados:

- **autoridade administrativa remota:** procedimento, agenda, usuário, unidade e referência operacional;
- **autoridade forense local:** fonte importada, arquivo, artefato, origem, seleção de evidência, relatório e auditoria do caso.

Uma entidade não é duplicada integralmente apenas para facilitar a interface.

## 2. Identificadores

- usar UUIDs opacos gerados por mecanismo criptograficamente adequado;
- não usar número de procedimento, CPF, IMEI ou nome como chave primária;
- preservar números oficiais como campos únicos dentro do escopo correto;
- registrar identificador remoto separadamente do identificador local;
- nunca derivar o ID de caso local de informação pessoal.

## 3. Modelo conceitual remoto inicial

### 3.1 Identidade e unidade

| Entidade | Campos conceituais mínimos |
|---|---|
| organization | id, nome, status, timestamps |
| unit | id, organization_id, nome, código, fuso, status |
| profile | user_id, nome, função institucional, status |
| membership | user_id, unit_id, papel, permissões excepcionais, vigência |

### 3.2 Procedimentos

| Entidade | Campos conceituais mínimos |
|---|---|
| procedure | id, unit_id, número, tipo, assunto, situação, sigilo, responsável, timestamps, version |
| procedure_person | id, procedure_id, person_id, papel no procedimento, observação controlada |
| person | id, nomes e identificadores estritamente necessários, classificação de dado |
| procedure_history | id, procedure_id, evento, ator, before/after minimizados, timestamp |

### 3.3 Dispositivos administrativos

| Entidade | Campos conceituais mínimos |
|---|---|
| seized_device | id, procedure_id, tipo, fabricante/modelo declarados, IMEI 1/2, serial, lacre, estado de custódia |
| device_movement | id, device_id, origem, destino, responsável, motivo, timestamp |
| forensic_case_link | id, procedure_id, device_id, local_case_id, fingerprint, estado, timestamps |

O registro administrativo de dispositivo não substitui o registro forense. O primeiro descreve custódia e vínculo; o segundo descreve o objeto no contexto do caso e da extração.

### 3.4 Agenda

| Entidade | Campos conceituais mínimos |
|---|---|
| agenda_event | id, unit_id, procedure_id opcional, tipo, início/fim, fuso, local, status, responsável |
| agenda_participant | event_id, person/profile, papel, presença/status |
| agenda_reminder | event_id, canal autorizado, antecedência, estado |
| agenda_history | event_id, ação, ator, timestamp, alteração mínima |

## 4. Modelo local preservado

O novo projeto deve migrar, não reinventar, os conceitos maduros do SIED:

- caso;
- dispositivo;
- extração;
- arquivo inventariado;
- execução e aviso de parser;
- artefato normalizado;
- referência de origem;
- conversa e participantes derivados;
- evidência selecionada;
- relatório;
- auditoria;
- dados derivados reconstruíveis.

Antes da migração, produzir mapa real de tabelas, versões de schema e invariantes do código antigo.

## 5. Estados do vínculo forense

Sugestão inicial:

```text
nao_iniciado
preparado
em_importacao
em_analise
aguardando_revisao
relatorio_emitido
encerrado
```

Esses estados são operacionais, não conclusões investigativas. Mudança de estado deve ser auditada. A nomenclatura final precisa ser validada com usuários.

## 6. Outbox de publicação

Para evitar salvar parcialmente no backend:

1. a ação de publicação cria um evento local imutável na outbox;
2. o cliente envia o evento com chave de idempotência;
3. o backend valida autorização e versão;
4. o backend registra o resultado e devolve recibo;
5. o cliente marca a entrega sem apagar o evento original.

Não implementar sincronização bidirecional genérica. Publicar eventos específicos é mais controlável e auditável.

## 7. Concorrência e conflitos

Dados remotos editáveis precisam de controle otimista por `version` ou timestamp confiável do servidor. Ao detectar conflito:

- não sobrescrever silenciosamente;
- mostrar os campos conflitantes;
- exigir escolha ou nova submissão;
- auditar o resultado.

O caso local é aberto por uma estação por vez, conforme o mecanismo de lock do SIED ou evolução validada dele.

## 8. Busca

Existirão escopos diferentes:

- **busca institucional:** procedimentos e metadados autorizados no backend;
- **busca do caso:** conteúdo normalizado e documentos no SQLite/FTS local;
- **busca global da estação:** somente se houver índice local seguro e política explícita.

A caixa de busca não deve enviar silenciosamente termos forenses ao serviço remoto. A interface precisa indicar o escopo antes da consulta.

## 9. Dados pessoais e sensíveis

Cada campo remoto deve ter:

- finalidade;
- base institucional/jurídica a ser definida pelo controlador;
- escopo de acesso;
- período de retenção;
- regra de correção;
- regra de exclusão ou bloqueio, quando aplicável;
- presença ou ausência em logs e backups.

O projeto técnico registra essas necessidades, mas não inventa política jurídica.

## 10. Ambientes

No mínimo:

| Ambiente | Dados permitidos | Finalidade |
|---|---|---|
| desenvolvimento local | somente sintéticos | desenvolvimento diário |
| homologação | sintéticos ou formalmente anonimizados/autorizados | teste integrado e aceite |
| produção | dados institucionais autorizados | operação real |

Nunca copiar produção inteira para desenvolvimento.

## 11. Seed e fixtures

- seed remoto deve conter unidade, perfis e procedimentos claramente sintéticos;
- fixtures forenses devem declarar `SINTÉTICO` no conteúdo visível;
- imagens e documentos precisam ser produzidos ou licenciados para teste;
- hashes esperados devem ser calculados no teste, não inventados em documentação;
- nenhum token, URL secreta ou identificador real entra no Git.

## 12. Backup e recuperação

Precisam existir planos diferentes:

- backup do PostgreSQL remoto;
- backup das configurações institucionais;
- backup/restauração do caso forense local;
- recuperação das chaves/segredos por processo administrativo;
- teste periódico de restauração.

“Backup configurado” não é suficiente. A restauração precisa ser testada e registrada.

## 13. Migração de dados antigos

Não migrar automaticamente no primeiro release. Preparar:

1. inventário de tabelas e volumes;
2. mapeamento origem-destino;
3. regras de transformação;
4. validações e reconciliação de contagem;
5. relatório de erros por registro;
6. execução simulada;
7. aceite;
8. migração controlada e rollback.
