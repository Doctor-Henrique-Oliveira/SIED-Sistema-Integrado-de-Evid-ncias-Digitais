# Nova fundação do SIED

**Estado:** documento de preparação
**Data de corte:** 10/10/2026
**Início pretendido da implementação:** 15/10/2026
**Responsável pelo produto:** Henrique Dias de Oliveira

Este diretório é o pacote de fundação para reconstruir o SIED em um repositório limpo, incorporando as capacidades úteis do SIPI sem destruir a arquitetura forense já existente.

O pacote não é código de produção e não cria banco, conta ou infraestrutura. Ele fixa o que precisa estar decidido antes da primeira linha do novo projeto e separa:

- fatos verificados nos projetos atuais;
- decisões recomendadas para a nova versão;
- hipóteses que ainda exigem confirmação;
- itens que não podem ser alegados como prontos.

## Ordem de leitura

1. [Documento Zero](docs/00-DOCUMENTO-ZERO.md)
2. [Inventário técnico](docs/01-INVENTARIO-TECNICO.md)
3. [Arquitetura alvo](docs/02-ARQUITETURA-ALVO.md)
4. [Dados e integrações](docs/03-DADOS-E-INTEGRACOES.md)
5. [Segurança, privacidade e auditoria](docs/04-SEGURANCA-GOVERNANCA.md)
6. [Consulta de IMEI](docs/05-MODULO-IMEI.md)
7. [Migração e reaproveitamento](docs/06-MIGRACAO-E-REAPROVEITAMENTO.md)
8. [Roadmap e critérios de aceite](docs/07-ROADMAP-E-ACEITE.md)
9. [Preparação para 15 de outubro](docs/08-CHECKLIST-DIA-15.md)
10. [Registro de decisões](docs/09-REGISTRO-DE-DECISOES.md)
11. [Requisitos e riscos](docs/10-REQUISITOS-E-RISCOS.md)

## Decisão central

O novo SIED será um produto único com módulos distintos. O SIPI deixa de existir como marca e aplicação paralela; suas capacidades administrativas úteis passam a compor módulos do SIED. O núcleo forense continua local e preserva evidência original, referências de origem, hashes, auditoria e reprodução.

O sistema não será uma simples soma de duas interfaces nem uma cópia cega de dois bancos.

## Módulos de primeira ordem

| Módulo | Finalidade | Persistência principal |
|---|---|---|
| Procedimentos Policiais | Gestão compartilhada do procedimento e seus envolvidos | Nuvem institucional |
| Agenda e Oitivas | Compromissos, responsáveis, conflitos e lembretes | Nuvem institucional |
| Consulta de IMEI | Validação e enriquecimento controlado por base versionada | Catálogo local/versionado; metadados controlados |
| Análise de Evidências Digitais | Importação, normalização, busca, revisão, seleção e relatório | Caso forense local |

## Regra de ouro

Dados administrativos podem apontar para um caso forense. Eles não recebem automaticamente mensagens, mídias, bancos extraídos ou cópias da evidência bruta. O vínculo é feito por identificadores, estado, hashes e artefatos deliberadamente publicados.

## Fora de escopo deste pacote

- criar a nova conta ou organização GitHub;
- criar projeto Supabase;
- escolher ou armazenar credenciais;
- importar dados reais;
- prometer compatibilidade com formatos não testados;
- migrar código antes da revisão de licença, segurança e dependências;
- decidir sozinho políticas institucionais de retenção e acesso.
