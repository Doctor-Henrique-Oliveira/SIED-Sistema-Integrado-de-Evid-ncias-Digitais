# Comando permanente para continuar o SIED

Use esta mensagem em uma nova sessão:

> **CONTINUE O PROJETO**

Ao receber esse comando, o agente deve seguir `AGENTS.md` e executar a primeira etapa realmente pendente de [`docs/PLANO-EXECUCAO-CONTINUA.md`](docs/PLANO-EXECUCAO-CONTINUA.md).

## Variações úteis

- `CONTINUE O PROJETO — SOMENTE INFORME A PRÓXIMA ETAPA`: consulta sem alterar código.
- `CONTINUE O PROJETO — RETOME A ETAPA ATUAL`: continua uma etapa já iniciada sem escolher outra.
- `CONTINUE O PROJETO — VALIDE A ETAPA ATUAL`: executa validações e corrige apenas falhas que impedem a conclusão da etapa.

## O que o comando não autoriza

- merge automático;
- uso de dados reais sem autorização específica;
- envio de evidências para serviços externos;
- alteração silenciosa de evidência, hashes ou invariantes forenses;
- antecipação do importador Avilla.

O estado oficial, a ordem e os critérios de conclusão ficam no plano. O código continua sendo a fonte final da verdade: uma marca documental incorreta deve ser corrigida, nunca usada para esconder uma implementação incompleta.
