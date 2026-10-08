# Instruções permanentes de continuidade do SIED

Estas instruções valem para todo o repositório.

## Comando de continuidade

Quando o responsável escrever **`CONTINUE O PROJETO`**, trate a mensagem como autorização para:

1. ler integralmente `CONTINUE-PROJETO.md` e `docs/PLANO-EXECUCAO-CONTINUA.md`;
2. conferir o estado real do código, testes, branch e alterações locais;
3. localizar a primeira etapa `PENDENTE` cujos pré-requisitos estejam concluídos;
4. implementar essa etapa de ponta a ponta no máximo permitido pelo ambiente;
5. corrigir falhas introduzidas pelo próprio trabalho, criar testes e integrar backend, persistência e interface necessários à mesma etapa;
6. atualizar o plano somente com fatos comprovados por código e validação;
7. criar ou continuar uma branch dedicada, fazer commits claros e enviar a branch ao GitHub.

Não pedir confirmação para decisões técnicas normais dentro da etapa. Só interromper por conclusão funcional ou bloqueio técnico real que impeça todo avanço seguro.

## Regras de execução

- Não pular etapas apenas porque uma etapa posterior parece mais interessante.
- Não considerar uma tela, contrato, mock, TODO ou documentação como funcionalidade concluída.
- Priorizar resultado real, local/offline, integrado e visível no aplicativo.
- Reutilizar os pipelines existentes; não criar produtos ou índices paralelos desnecessários.
- Preservar evidência original, hashes, rastreabilidade, auditoria e separação entre fato, derivado e revisão humana.
- Nunca inventar resultado investigativo, progresso, confiança, hash ou suporte de formato.
- Não enviar evidência a serviços externos sem configuração e ação explícitas.
- Usar somente dados sintéticos ou conjuntos formalmente autorizados em testes.
- Não alterar nem versionar `.claude/`.
- CI pendente, ferramenta opcional ausente e validação externa não impedem trabalho local independente; registrar a limitação e continuar.
- Não fazer merge automático. O comando autoriza implementação, commit e push da branch da etapa, mas o merge continua dependendo de ordem explícita.
- Não iniciar a etapa seguinte no mesmo ciclo depois de concluir a etapa atual, salvo se o responsável pedir expressamente execução de múltiplas etapas.

## Avilla Forensics

O importador Avilla Forensics é obrigatoriamente a **última etapa** do plano. Não antecipá-lo e não presumir sua estrutura. Ele só pode ser implementado com árvore e amostras de laboratório reais, autorizadas e documentadas.
