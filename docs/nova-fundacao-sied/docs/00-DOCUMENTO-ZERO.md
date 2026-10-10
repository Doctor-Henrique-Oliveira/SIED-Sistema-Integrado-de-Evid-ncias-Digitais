# Documento Zero — SIED integrado

## 1. Identidade do produto

**Nome de trabalho:** SIED
**Expansão atual preservada:** Sistema Integrado de Evidências Digitais
**Descrição de produto recomendada:** Plataforma integrada para gestão de procedimentos, dispositivos e análise de evidências digitais.

A sigla e a abertura visual do SIED permanecem. A ampliação de finalidade é comunicada pela descrição do produto, sem trocar a marca antes de uma decisão explícita.

## 2. Problema que o produto resolve

Hoje, o fluxo de trabalho está fragmentado:

- o procedimento e os prazos vivem em um sistema administrativo;
- o aparelho apreendido e seus identificadores podem estar em planilhas, textos ou outro cadastro;
- a extração e sua análise vivem em uma ferramenta local separada;
- evidências selecionadas e relatórios precisam ser relacionados manualmente ao procedimento;
- o investigador alterna entre aplicações e repete dados.

O SIED integrado deve oferecer continuidade operacional sem misturar indevidamente dados administrativos com conteúdo forense bruto.

## 3. Proposta de valor

O sistema acompanha o ciclo:

```text
Procedimento
  -> pessoa ou fato investigado
  -> objeto/dispositivo apreendido
  -> identificação e consulta de IMEI
  -> caso forense local
  -> extração
  -> análise e revisão
  -> evidência selecionada
  -> relatório verificável
  -> referência devolvida ao procedimento
```

O valor não está apenas em “abrir conversas”. Está em reduzir perda de contexto, registrar a origem de cada conclusão e aproximar a gestão do caso do trabalho técnico sem enfraquecer a cadeia de custódia.

## 4. Princípios não negociáveis

1. **A evidência original é somente leitura.** O sistema trabalha sobre inventário, cópias controladas e dados derivados.
2. **Toda afirmação deve ter origem.** Mensagens, arquivos, hashes e campos interpretados precisam apontar para o registro e arquivo de origem.
3. **Fato, derivação automática e revisão humana são diferentes.** A interface e o banco não podem confundi-los.
4. **Processamento local por padrão.** Evidência não sai da estação sem configuração, ação e auditoria explícitas.
5. **Sem compatibilidade fictícia.** Uma plataforma ou formato só aparece como operacional quando existe leitor testado.
6. **Sem métricas inventadas.** Painéis exibem somente dados calculados a partir de registros reais.
7. **Menor privilégio.** Acesso a módulos, procedimentos sigilosos e ações administrativas depende de autorização verificável.
8. **Auditoria não é decoração.** Operações relevantes precisam ser registradas com ator, tempo, alvo, resultado e contexto mínimo.
9. **Falha segura.** Indisponibilidade de rede não autoriza contornar políticas nem apagar trabalho local.
10. **Teste com dados sintéticos ou formalmente autorizados.** Nunca usar evidência real como fixture comum do repositório.

## 5. Público e papéis

| Papel conceitual | Necessidade principal |
|---|---|
| Analista forense | Importar, interpretar, pesquisar, revisar, selecionar e relatar evidências |
| Investigador | Consultar procedimento, relacionamentos, dispositivos e resultados publicados |
| Escrivão/apoio | Manter dados processuais, agenda e documentos administrativos |
| Delegado/gestor | Controlar acesso, sigilo, prazos, conclusões e supervisão |
| Administrador | Gerir usuários, unidade, configuração e auditoria técnica, sem acesso automático a todo conteúdo forense |

Papéis definitivos dependem da política da unidade. O código não deve presumir que “administrador técnico” equivale a “autorizado a ler toda evidência”.

## 6. Módulos iniciais

### 6.1 Procedimentos Policiais

Gestão de inquéritos e demais classes adotadas pela unidade, envolvidos, situação, gravidade, equipe, prazos, histórico e vínculos com bens/dispositivos.

### 6.2 Agenda e Oitivas

Compromissos, oitivas, responsáveis, participantes, conflitos, lembretes e histórico. A agenda deve respeitar unidade, fuso horário e permissões.

### 6.3 Consulta de IMEI

Valida o identificador, consulta uma base cuja origem e versão estejam registradas e retorna somente atributos sustentados por essa base. Não identifica proprietário, linha ativa ou situação criminal sem fonte específica autorizada.

### 6.4 Análise de Evidências Digitais

Preserva as capacidades do SIED atual: casos locais, dispositivos, importação, inventário e hashing, normalização, visualização de conversas e arquivos, busca, revisão, evidências selecionadas, relatório e auditoria.

## 7. Módulos posteriores candidatos

- Boletim de Ocorrência;
- veículos apreendidos;
- objetos apreendidos;
- contatos e localização operacional;
- representações;
- estatísticas e produtividade;
- descoberta multimodal local;
- integrações institucionais formalmente autorizadas.

Eles não devem disputar a primeira entrega. Entram após o fluxo vertical principal estar funcionando.

## 8. Experiência inicial

Após autenticação e carregamento, o usuário vê uma central de módulos. Cada cartão informa:

- nome e função do módulo;
- disponibilidade real;
- se depende de conexão;
- alertas objetivos que exigem ação;
- permissão insuficiente sem expor dados do módulo;
- última sincronização quando aplicável.

A tela inicial não deve ser um painel lotado de estatísticas. Deve permitir que o usuário entenda onde está e comece uma tarefa.

## 9. Definição de sucesso da primeira versão integrada

Um usuário autorizado consegue:

1. entrar no SIED;
2. localizar ou criar um procedimento;
3. registrar um dispositivo e seu IMEI;
4. consultar o IMEI em uma base identificada e versionada;
5. criar ou vincular um caso forense local;
6. importar uma extração suportada preservando a fonte;
7. selecionar ao menos uma evidência com origem verificável;
8. gerar relatório local com hashes;
9. publicar no procedimento apenas a referência e os metadados autorizados;
10. consultar a trilha de auditoria das ações.

Se esse fluxo não funcionar de ponta a ponta, a integração ainda não está pronta, mesmo que todas as telas existam.

## 10. Não objetivos da primeira versão

- substituir ferramentas de aquisição física ou lógica;
- prometer detecção automática de armas, drogas ou pessoas sem motores locais avaliados;
- funcionar como interceptação, monitoramento ou consulta clandestina;
- replicar toda evidência na nuvem;
- implementar todas as redes sociais simultaneamente;
- oferecer IA sem explicabilidade, revisão e medição;
- migrar automaticamente toda a base antiga antes de validação.

## 11. Hipóteses a validar

- Procedimentos e agenda devem continuar multiusuário e conectados.
- O núcleo forense deve continuar local/offline.
- A unidade aceita autenticação online para abrir módulos administrativos.
- Há fonte legítima e atualizável para a base de IMEI.
- Existe política institucional para retenção, backup, acesso e descarte.
- Os usuários dispõem de Windows 10/11 compatível com Tauri 2.

Essas hipóteses são registradas, não tratadas como fatos.
