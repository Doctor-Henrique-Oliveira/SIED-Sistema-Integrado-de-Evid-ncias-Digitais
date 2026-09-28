# SIED — Visão principal e permanente

> Texto fornecido pelo responsável pelo projeto em 28/09/2026, preservado integralmente.
> É a referência para decisões de produto e arquitetura. Ver `ARQUITETURA.md`.

---

Quero que você trate as instruções abaixo como a visão principal e permanente do projeto SIED.

# SIED — Sistema Integrado de Evidências Digitais

Quero desenvolver uma plataforma forense digital extremamente avançada, moderna e orientada ao trabalho investigativo, com foco inicial na análise de dados provenientes de dispositivos móveis.

O objetivo NÃO é simplesmente criar outro visualizador de arquivos ou um gerador de PDF.

Quero construir uma estação completa de análise de evidências digitais que consiga receber grandes extrações forenses, interpretar seus artefatos, normalizar os dados, apresentá-los de forma visual e intuitiva e permitir que o investigador transforme milhares ou milhões de registros técnicos em informações compreensíveis e em um relatório investigativo profissional.

A aplicação deverá ser pensada para uso institucional e para análise de extrações legalmente obtidas.

## 1. FILOSOFIA PRINCIPAL

Ferramentas como Avilla Forensics, IPED, ADB, ALEAPP, iLEAPP, Cellebrite e outras poderão fornecer os dados.

O SIED deverá atuar principalmente na etapa:

IMPORTAÇÃO → PROCESSAMENTO → NORMALIZAÇÃO → INDEXAÇÃO → ANÁLISE → CORRELAÇÃO → SELEÇÃO DE EVIDÊNCIAS → RELATÓRIO

O SIED não deverá depender de um único formato de aquisição.

Quero uma arquitetura baseada em importadores/adapters.

Exemplo: Avilla Importer, IPED Importer, ADB Importer, Generic Folder Importer, ALEAPP Importer, iLEAPP Importer, Futuros importadores.

Todos deverão transformar os dados encontrados em um modelo interno comum do SIED.

## 2. MODELO UNIVERSAL DE ARTEFATOS

Independentemente da ferramenta de origem, o SIED deverá normalizar os artefatos.

Exemplo conceitual:

Artifact
- id
- case_id
- device_id
- type
- subtype
- application
- source_tool
- source_file
- original_path
- database
- table
- record_id
- sender
- recipients
- account
- timestamp
- timestamp_original
- timezone
- content
- media
- mime_type
- filename
- file_size
- latitude
- longitude
- metadata
- hash
- deleted/status information when available
- parser used
- confidence/parse information
- raw/original reference

O conteúdo original nunca deverá ser modificado.

O SIED pode criar banco local, índices, miniaturas, previews e caches, mas tudo deverá ficar separado da evidência original.

## 3. EXPERIÊNCIA DE USO

Quero uma interface extremamente moderna.

Não quero que o investigador precise olhar tabelas SQLite ou registros técnicos para compreender uma conversa.

O princípio é:

DADOS FORENSES POR BAIXO.
EXPERIÊNCIA HUMANA POR CIMA.

Por exemplo, se o parser encontrar mensagens do WhatsApp, quero reconstruir a conversa visualmente como se o investigador estivesse utilizando um aplicativo de mensagens.

Deverá existir: lista de conversas; foto/nome do contato quando disponível; mensagens enviadas e recebidas em lados diferentes; horários; datas; separadores por dia; respostas; mensagens encaminhadas; mídias; imagens; vídeos; áudios; documentos; localizações; contatos compartilhados; participantes de grupos; informações de chamadas quando disponíveis; status técnicos disponíveis no artefato.

Ao clicar em uma mensagem deverá existir: VISÃO NORMAL e VISÃO FORENSE / DETALHES TÉCNICOS.

A visão técnica poderá mostrar: arquivo de origem; caminho; database; tabela; record ID; timestamps; remetente; destinatário; identificador da mensagem; hash; metadados; parser; ferramenta de origem; informações adicionais existentes no artefato.

## 4. ÁREAS PRINCIPAIS

Quero futuramente possuir pelo menos: Dashboard / Visão Geral; Casos; Dispositivos; Extrações; Conversas; Contatos; Chamadas; SMS; Aplicativos; Imagens; Vídeos; Áudios; Documentos; Arquivos; Localizações; Linha do Tempo; Busca Global; Evidências Selecionadas; Relatórios; Auditoria; Configurações.

## 5. CASOS

Toda análise deverá estar vinculada a um caso/procedimento.

Exemplo: Procedimento; Número do procedimento; Número do BO; Investigado; Vítima; Delegado; Responsável pela análise; Unidade; Observações.

Um caso poderá possuir vários dispositivos e várias extrações.

## 6. DISPOSITIVOS

Registrar, quando disponível: fabricante; modelo; nome comercial; Android/iOS; versão do sistema; IMEI; IMEI 2; serial; número; operadora; contas; capacidade de armazenamento; identificadores técnicos; datas relevantes; ferramenta utilizada para aquisição.

## 7. IMPORTAÇÃO

Quero uma experiência simples:

NOVO CASO → ADICIONAR DISPOSITIVO → IMPORTAR EXTRAÇÃO → selecionar origem

Exemplo: Avilla Forensics; IPED; ADB / Android; ALEAPP; iLEAPP; Pasta genérica.

O sistema deverá: detectar a estrutura; inventariar arquivos; calcular hashes quando apropriado; registrar origem; identificar bancos SQLite; identificar mídias; identificar documentos; executar parsers compatíveis; indexar conteúdo; gerar previews; registrar erros de parsing; mostrar progresso.

Grandes extrações não podem travar a interface.

Processamento deverá ocorrer por tarefas/jobs em background.

## 8. BUSCA

Quero uma busca extremamente poderosa.

Uma única pesquisa poderá procurar em: mensagens; contatos; nomes; números; e-mails; documentos; nomes de arquivos; metadados; localizações; OCR futuramente; transcrições futuramente; URLs; aplicativos.

Filtros: data inicial/final; aplicativo; contato; tipo de artefato; dispositivo; extração; somente mídia; somente localização; somente evidências selecionadas; outros filtros relevantes.

Quero busca rápida mesmo em bases muito grandes.

## 9. LINHA DO TEMPO

Quero uma timeline única capaz de correlacionar eventos.

Exemplo:

- 14:31 mensagem WhatsApp
- 14:32 ligação
- 14:35 fotografia
- 14:37 localização GPS
- 14:42 pesquisa no navegador
- 14:45 vídeo

Isso deverá permitir ao investigador compreender cronologicamente o que aconteceu.

## 10. LOCALIZAÇÕES

Quando existirem coordenadas, quero uma área própria para análise geográfica.

Mostrar: mapa; pontos; datas; origem do dado; agrupamento; trajetos quando tecnicamente justificáveis; filtro por período; artefatos relacionados.

Toda localização deverá manter referência para sua fonte original.

## 11. GALERIA DE MÍDIA

Quero uma galeria avançada para: imagens; vídeos; áudios.

Filtros por: data; pasta; aplicativo; tipo; extensão; metadados; localização; evidência selecionada.

Ao abrir uma imagem quero visualizar EXIF e demais metadados existentes.

## 12. EVIDÊNCIAS SELECIONADAS

Este deverá ser um dos principais recursos do SIED.

Qualquer artefato poderá possuir: ADICIONAR ÀS EVIDÊNCIAS.

Exemplos: mensagem; trecho de conversa; imagem; vídeo; áudio; documento; localização; ligação; contato; arquivo.

As evidências selecionadas deverão ir para uma área própria.

Nela o investigador poderá: organizar; reordenar; categorizar; adicionar título; adicionar observação; adicionar descrição contextual; remover; visualizar fonte original.

Cada evidência deverá manter rastreabilidade completa até o artefato original.

## 13. GERADOR DE RELATÓRIO

Quero um dos melhores geradores de relatório possíveis.

O usuário deverá poder selecionar quais seções entram no documento.

Exemplo: capa; identificação do procedimento; responsável; informações do dispositivo; informações da aquisição; hashes; metodologia; evidências selecionadas; conversas; imagens; mapas; linha do tempo; observações; conclusão do analista.

O relatório deverá apresentar as evidências visualmente.

Uma conversa selecionada não deve virar uma tabela feia.

Ela deve ser apresentada de forma legível, semelhante à reconstrução visual do chat, acompanhada das informações forenses necessárias.

Imagens poderão aparecer com legenda.

Localizações poderão possuir mapa/imagem e coordenadas.

Cada evidência deverá possuir uma referência que permita descobrir sua origem.

Gerar PDF profissional e padronizado.

Futuramente considerar HTML e outros formatos.

## 14. INTEGRIDADE E RASTREABILIDADE

Essa parte é obrigatória.

O sistema nunca deverá alterar silenciosamente os dados originais.

Registrar: hashes; origem da importação; data/hora; usuário; parser utilizado; versão do parser; ações relevantes realizadas no SIED; erros; processamentos; exportações; geração de relatórios.

Quero auditoria.

O banco e os arquivos de trabalho do SIED deverão ser separados da evidência original.

## 15. OFFLINE / PRIVACIDADE

O sistema deverá ser LOCAL-FIRST.

Deverá funcionar completamente offline.

Por padrão: nenhuma evidência; nenhuma mensagem; nenhuma fotografia; nenhum arquivo; nenhum dado do procedimento deverá ser enviado para servidores externos.

No futuro poderá existir uma edição servidor exclusivamente para rede interna.

## 16. ARQUITETURA

Quero desktop + tecnologias web.

Minha preferência inicial: Tauri, React, TypeScript.

Quero uma interface moderna e rápida.

Para armazenamento/indexação local, avalie tecnologias apropriadas, como SQLite e mecanismos de full-text search.

O sistema deve conseguir lidar com centenas de milhares ou milhões de artefatos sem carregar tudo na memória.

Utilize: paginação; virtualização de listas; lazy loading; jobs; workers; cache; processamento incremental; indexação.

A escolha do backend/parsers pode variar de acordo com o problema.

Podemos utilizar Rust, Python ou outras tecnologias quando tecnicamente justificável.

Mas evite criar uma arquitetura desnecessariamente complexa.

## 17. SEGURANÇA

Preparar arquitetura para: login local; controle de usuários; perfis; permissões; auditoria; bloqueio de acesso a casos; proteção das informações; logs.

Nunca armazenar credenciais diretamente no código.

Nunca versionar .env, tokens, senhas ou dados reais de investigações no GitHub.

## 18. DESENVOLVIMENTO

Não tente implementar tudo ao mesmo tempo.

Quero desenvolvimento incremental.

Primeiro devemos estabelecer: arquitetura; estrutura do projeto; modelo de dados; sistema de casos; dispositivos; importação; primeiro importer; indexação; visualização básica.

Nossa primeira fonte de dados deverá ser: AVILLA FORENSICS.

Depois: IPED.

Depois poderemos expandir.

O primeiro grande objetivo funcional deverá ser: importar uma extração; identificar artefatos; indexar; pesquisar; visualizar conversas; selecionar evidências; gerar relatório.

## 19. QUALIDADE DO PROJETO

Quero que este seja tratado como software profissional e não como protótipo descartável.

Código organizado. Componentes reutilizáveis. Separação entre UI, domínio, parsers e armazenamento. Tipagem forte. Tratamento de erros. Logs adequados. Migrações de banco. Testes para componentes críticos. Documentação.

Não faça grandes alterações arquiteturais sem explicar a razão.

Não remova funcionalidades existentes sem necessidade.

## 20. PRINCÍPIO FUNDAMENTAL

Sempre pense na seguinte pergunta:

"Como podemos transformar uma grande quantidade de dados técnicos de uma extração forense em uma experiência extremamente simples e útil para o investigador, sem perder a rastreabilidade técnica da evidência?"

Quero unir dois mundos: FACILIDADE DE USO + RIGOR FORENSE.

A interface principal deve ser compreensível até para alguém que não conheça SQLite, bancos de dados ou estruturas internas do Android.

Mas um profissional técnico deverá conseguir abrir os detalhes e verificar exatamente de onde cada informação veio.

## 21. VISÃO DE LONGO PRAZO

Não quero limitar o SIED a ser apenas "um gerador de relatório".

A visão é criar uma plataforma completa de análise forense digital.

No futuro poderemos estudar: correlação automática de artefatos; relações entre pessoas; gráficos de vínculos; timeline avançada; análise geográfica; transcrição local de áudios; OCR local; detecção de duplicados; comparação entre dispositivos; cruzamento entre casos; análise de bancos SQLite; sistema de plugins/parsers; integração com outras ferramentas forenses; automação de tarefas repetitivas.

Esses recursos futuros não devem impedir a criação de uma primeira versão sólida.

---

Antes de começar a escrever grandes quantidades de código:

1. Analise esta visão.
2. Analise o repositório atual.
3. Proponha a arquitetura do SIED.
4. Defina a estrutura das pastas.
5. Proponha o modelo inicial do banco de dados.
6. Defina como funcionará o sistema de importadores/parsers.
7. Defina como preservaremos a referência entre um artefato normalizado e sua fonte original.
8. Divida o desenvolvimento em fases.
9. Informe o que será implementado na primeira fase.

Não implemente dezenas de módulos vazios apenas para aparentar que o sistema está completo.

Prefiro uma base extremamente sólida e funcional que possa crescer durante anos.

Depois que a arquitetura estiver definida, começaremos a implementação.
