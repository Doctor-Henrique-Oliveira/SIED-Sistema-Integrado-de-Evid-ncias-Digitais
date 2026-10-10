# Migração e reaproveitamento

## 1. Estratégia

O repositório será novo, mas o software não será reescrito por vaidade. Cada parte dos projetos atuais deve ser classificada por contrato, teste, dependência e risco.

Categorias:

- **migrar quase intacto:** implementação e testes já aderem à arquitetura alvo;
- **adaptar:** regra é válida, mas depende de roteamento, UI ou backend antigo;
- **usar como referência:** conceito é útil, código não deve ser copiado;
- **reescrever:** fronteira ou risco torna a adaptação mais cara;
- **descartar:** duplicação, mock, experimento ou funcionalidade sem valor comprovado.

## 2. Matriz inicial

| Área | Origem | Decisão inicial | Observação |
|---|---|---|---|
| crates de domínio forense | SIED | migrar | preservar APIs e testes, após inventário |
| armazenamento SQLite/migrations | SIED | migrar com compatibilidade | não reescrever schema sem necessidade |
| ingestão/importadores/parsers | SIED | migrar | manter origem somente leitura |
| documentos/mídia/relatórios | SIED | migrar | validar dependências e build |
| comandos Tauri | SIED | adaptar | reorganizar por módulo/capability |
| interface forense | SIED | adaptar | encaixar no novo shell sem perder funções |
| identidade visual/abertura | SIED | adaptar | reduzir poluição e preservar marca |
| regras de procedimento | SIPI | adaptar | desacoplar de rotas específicas |
| regras de agenda | SIPI | adaptar | preservar conflito/lembrete após revisão |
| migrations Supabase | SIPI | revisar e selecionar | não copiar 75 cegamente |
| RLS e authz | SIPI | usar como base | redesenhar papéis para produto integrado |
| veículos/objetos/BO/localização | SIPI | postergar | módulos posteriores |
| dashboard antigo | ambos | usar como referência | nova home é central de módulos |
| consulta IMEI atual | SIED | reaproveitar validação | catálogo e importador são novos |
| dados de teste | ambos | selecionar | somente sintéticos/licenciados |

## 3. Processo por componente

Para cada componente migrado:

1. identificar comportamento e dependências;
2. localizar testes existentes;
3. registrar invariantes;
4. copiar em commit isolado, preservando autoria quando possível;
5. adaptar imports e fronteiras sem alterar comportamento;
6. executar testes originais;
7. adicionar teste de integração no novo shell;
8. documentar divergências;
9. somente depois refatorar.

Não misturar migração, refatoração extensa e recurso novo no mesmo commit.

## 4. Histórico Git e autoria

Como o repositório será novo, escolher conscientemente:

- importar histórico dos repositórios antigos por subtree/filter-repo; ou
- copiar componentes com referência ao commit de origem no commit e no inventário.

Recomendação inicial: repositório limpo com commits de migração claramente referenciados. Se houver exigência de preservar histórico integral, decidir antes da primeira migração.

## 5. Licenças e ativos

Antes de copiar:

- confirmar propriedade/autorização dos dois códigos;
- verificar licenças de ícones, fontes, imagens e fixtures;
- manter avisos de dependências;
- não copiar credenciais, dados reais, dumps ou arquivos gerados;
- não incorporar logos de plataformas de forma incompatível com suas marcas;
- registrar origem de qualquer dataset IMEI.

## 6. Migração do SIED

Ordem recomendada:

1. workspace e build mínimo;
2. crates puras com testes;
3. banco/migrations do caso;
4. bridge Tauri mínima;
5. abrir/criar caso;
6. dispositivo;
7. importação e inventário;
8. parsers e visualizações;
9. busca e evidências;
10. relatórios e auditoria;
11. recursos documentais/multimodais em etapas posteriores.

O velho roadmap de IA não deve bloquear a integração administrativa.

## 7. Migração do SIPI

Ordem recomendada:

1. extrair vocabulário e regras de negócio;
2. desenhar tenancy, papéis e sigilo;
3. criar migrations novas e mínimas;
4. implementar repositórios/serviços tipados;
5. migrar procedimentos;
6. migrar agenda;
7. migrar auditoria;
8. só então avaliar dados legados;
9. módulos secundários entram depois do fluxo principal.

TanStack Start e Cloudflare não precisam ser carregados para dentro do desktop apenas porque existem no SIPI. Reutilizar React e regras, não necessariamente o runtime web inteiro.

## 8. Dados legados

O sistema novo deve iniciar com dados sintéticos. Dados reais dos sistemas antigos entram somente por migrador auditável.

O migrador precisa produzir:

- versão;
- origem e hash do export;
- contagens por tabela;
- registros aceitos, rejeitados e transformados;
- erros acionáveis;
- reconciliação final;
- modo simulação;
- relatório assinado/armazenado conforme política.

## 9. Compatibilidade de casos antigos

O novo SIED deve decidir entre:

- abrir caso antigo diretamente com migrations compatíveis; ou
- importar uma cópia do caso para formato novo.

Recomendação: manter abertura/migração compatível e criar backup antes de qualquer upgrade. Nunca atualizar o único exemplar sem cópia e verificação.

## 10. Critérios para descartar código

Descartar quando:

- é mock apresentado como funcionalidade;
- duplica fonte ou índice sem necessidade;
- depende de formato não documentado;
- não possui origem/licença válida;
- viola a fronteira local/remota;
- exige segredo no cliente;
- não tem usuário ou objetivo identificável;
- a regra de negócio foi substituída por decisão registrada.

## 11. Resultado esperado da migração

Ao final, o novo repositório deve conseguir apontar de onde veio cada capacidade importante, quais testes a protegem e quais diferenças foram deliberadas. “Visualmente parecido” não constitui migração concluída.
