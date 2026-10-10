# Roadmap e critérios de aceite

## 1. Estratégia de entrega

Construir uma fatia vertical antes de expandir módulos. Cada etapa termina em comportamento executável, testes e documentação atualizada.

## 2. Fase 0 — Fundação

**Objetivo:** repositório confiável e build mínimo.

Entregas:

- organização/repositório;
- proteção de branch e CI;
- Tauri + React + Rust compilando;
- estrutura modular;
- design tokens básicos;
- documentação e ADRs iniciais;
- ambientes Supabase separados;
- fixtures exclusivamente sintéticas;
- política de segredos.

Aceite:

- clone limpo compila seguindo README;
- CI passa;
- nenhum segredo ou dado real no Git;
- aplicativo abre a central de módulos;
- módulos ainda não implementados são marcados honestamente.

## 3. Fase 1 — Identidade, sessão e módulos

**Objetivo:** entrar com segurança e navegar entre domínios.

Entregas:

- login/logout;
- perfil, unidade e autorização;
- central de módulos;
- estado online/offline;
- bloqueio de módulo sem permissão;
- auditoria de autenticação;
- shell visual SIED.

Aceite:

- usuário autorizado vê apenas o permitido;
- usuário bloqueado não acessa API por chamada direta;
- indisponibilidade de rede é comunicada corretamente;
- sessão e tokens não aparecem em logs.

## 4. Fase 2 — Procedimento e dispositivo

**Objetivo:** criar o contexto operacional da análise.

Entregas:

- listar, pesquisar, criar e editar procedimento;
- sigilo e equipe;
- cadastro de dispositivo apreendido;
- IMEI/serial/lacre;
- histórico e auditoria;
- vínculo preparado para caso local.

Aceite:

- permissões e RLS testadas;
- concorrência não sobrescreve silenciosamente;
- histórico registra mudanças relevantes;
- dispositivo pertence ao procedimento correto.

## 5. Fase 3 — IMEI

**Objetivo:** consulta local, versionada e explicável.

Entregas:

- importador do dataset real autorizado;
- staging, validação e relatório de rejeições;
- ativação/rollback de snapshot;
- consulta e fonte do resultado;
- vínculo ao dispositivo;
- auditoria.

Aceite:

- critérios do documento do módulo IMEI atendidos;
- mesmo snapshot produz resultado reproduzível;
- ausência não é apresentada como falsidade;
- nenhum atributo é inventado.

## 6. Fase 4 — Núcleo forense integrado

**Objetivo:** executar o fluxo local dentro do novo shell.

Entregas:

- criar/abrir caso local;
- vincular procedimento/dispositivo;
- migrar banco e crates essenciais;
- importação suportada;
- conversas/busca/arquivos;
- evidência selecionada;
- relatório e hash;
- auditoria local.

Aceite:

- fonte permanece intacta;
- hashes conferem;
- referências de origem abrem o registro correto;
- caso funciona sem rede depois de autorizado conforme política;
- dados brutos não aparecem no Supabase.

## 7. Fase 5 — Publicação controlada

**Objetivo:** devolver resultado autorizado ao procedimento.

Entregas:

- outbox local;
- publicação idempotente;
- estado da análise;
- referência/hash do relatório;
- recibo e auditoria;
- tratamento de conflito e falha.

Aceite:

- repetir envio não duplica;
- falha de rede não perde publicação;
- backend rejeita usuário sem autorização;
- conteúdo publicado corresponde à confirmação mostrada.

## 8. Fase 6 — Agenda e oitivas

**Objetivo:** trazer a agenda SIPI para o produto integrado.

Entregas:

- calendário/lista;
- responsáveis e participantes;
- conflito;
- lembrete;
- vínculo com procedimento;
- histórico e permissões.

Aceite:

- fuso e intervalos testados;
- conflitos são detectados por regra documentada;
- lembretes não expõem conteúdo indevido;
- módulo respeita unidade e autorização.

## 9. Fases posteriores

- BO;
- veículos;
- objetos;
- localização operacional;
- relatórios administrativos;
- OCR, transcrição, busca híbrida e visão local;
- novos parsers;
- Avilla somente com estrutura real autorizada.

## 10. Definição de pronto comum

Uma entrega está pronta quando:

- comportamento está integrado e visível;
- autorização existe no servidor quando aplicável;
- migrations estão versionadas;
- estados de erro/vazio/carregamento existem;
- testes pertinentes passam;
- acessibilidade básica foi verificada;
- logs não vazam conteúdo;
- documentação reflete o código;
- limitações são declaradas;
- não existem mocks apresentados como suporte real;
- há caminho de recuperação/rollback proporcional ao risco.

## 11. Pirâmide de testes

- testes unitários de domínio;
- testes de repositório e migrations;
- testes de RLS por papel/unidade/sigilo;
- testes de componentes;
- integração Tauri;
- E2E da fatia vertical;
- testes de volume com dados sintéticos;
- testes de arquivos malformados;
- teste manual documentado em Windows;
- restauração de backup em ambiente de homologação.

## 12. Cenário E2E obrigatório

```text
Dado um usuário autorizado e um procedimento sintético
quando cadastra um dispositivo com IMEI válido
e consulta o catálogo sintético versionado
e cria um caso forense local vinculado
e importa uma extração sintética suportada
e seleciona uma mensagem como evidência
e gera um relatório
e publica a referência do relatório
então o procedimento mostra o vínculo e o hash
sem armazenar a mensagem ou a extração no backend remoto
e as duas auditorias permitem reconstruir as ações.
```

## 13. Métricas úteis

- taxa de sucesso da importação por formato;
- tempo de consulta local em volumes definidos;
- quantidade de conflitos/erros de sincronização;
- falhas de RLS detectadas em CI;
- restauração testada com sucesso;
- tempo para completar a fatia vertical;
- erros de usuário que indicam problema de interface.

Evitar métricas decorativas como “produtividade” sem definição, fonte e finalidade.
