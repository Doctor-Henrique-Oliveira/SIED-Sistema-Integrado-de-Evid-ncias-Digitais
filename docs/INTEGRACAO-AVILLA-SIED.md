# Integração Avilla Forensics ↔ SIED

## Objetivo

Registrar a análise funcional do Avilla Forensics disponível na estação de
treinamento e definir como seus resultados poderão entrar no SIED sem misturar
aquisição, evidência original e análise derivada.

Este documento registra capacidades observadas no aplicativo e nas telas de
cadeia de custódia, criação de caso e workspace. Não é uma declaração de
compatibilidade universal nem substitui validação em aparelho de laboratório.

## O que foi observado

O Avilla Forensics FREE funciona como uma estação de aquisição lógica/partial e
como um orquestrador de ferramentas auxiliares. A instalação observada expõe:

- abertura de caso, caso existente, modo sem caso e caso em disco virtual;
- formulário de cadeia de custódia com origem, data/hora, lacre, armazenamento,
  responsável e observações;
- cadastro de pasta de aquisição e metadados básicos do caso;
- coletas ADB de propriedades, IMEI/serial quando disponíveis, apps, usuários,
  contatos, SMS, localização, Wi-Fi, Bluetooth, processos e logs;
- aquisição de armazenamento compartilhado e conversão de backups ADB;
- módulos de WhatsApp, Signal, iOS, transcrição OPUS, OCR, mídia, localização e
  geração de relatórios;
- integração com IPED, ALEAPP/iLEAPP, SQLite, JADX, scrcpy, ExifTool e outros
  utilitários de análise;
- cálculo de hashes e geração de logs de integridade da própria ferramenta.

As telas observadas mostram uma barra de módulos extensa e atalhos para
ferramentas externas. Isso confirma que o Avilla é principalmente uma estação
de aquisição e preparação; o SIED deve permanecer como o sistema central de
caso, inventário, auditoria, busca e correlação.

## Mapeamento conceitual para o SIED

| Avilla | Destino conceitual no SIED |
| --- | --- |
| Número do caso | Identidade do caso |
| Autorização legal | Metadado de cadeia de custódia |
| Origem/data/hora/lacre | Eventos de custódia auditáveis |
| Responsável pela guarda | Participante/operador do caso |
| Pasta de aquisição | Raiz de importação/extração |
| Modelo, serial, IMEI e Android | Dispositivo cadastrado |
| TXT de coleta | Artefato derivado com origem preservada |
| Banco e mídia de aplicativo | Artefatos estruturados e anexos |
| Transcrição/OCR | Derivados pesquisáveis, nunca evidência original |
| Hashes e log `.avilla` | Evidência de integridade vinculada à auditoria |
| HTML/IPED/ALEAPP | Relatório ou resultado externo derivado |

## Contrato de integração recomendado

Uma futura importação Avilla no SIED deve:

1. receber uma pasta de aquisição como fonte somente leitura;
2. registrar o hash dos arquivos antes de qualquer parsing;
3. preservar a árvore original e os nomes dos arquivos;
4. importar manifestos e logs de integridade como referências auditáveis;
5. reconhecer TXT de coleta, bancos, mídias, transcrições, OCR e relatórios;
6. associar cada derivado ao arquivo de origem e ao dispositivo correspondente;
7. apresentar a ferramenta, versão, parser e limitações em cada resultado;
8. permitir reprocessamento sem sobrescrever a aquisição original;
9. manter a auditoria própria do SIED, mesmo quando houver log `.avilla`;
10. marcar resultados como derivados/candidatos quando não forem fatos brutos.

## Limites e segurança

- A coleta do Avilla é parcial/complementar; não equivale automaticamente a
  Full File System (FFS).
- Compatibilidade depende do fabricante, Android, patch, perfil, aplicativo,
  backup, chave e estado de autorização ADB.
- Resultados de vídeos, README ou telas do fornecedor são documentação, não
  validação pericial do dispositivo concreto.
- Ações como downgrade, instalação de APK, `adb push`, reboot, PIN, recovery,
  bootloader e fastboot não devem ser acionadas automaticamente pelo SIED.
- Nenhuma importação deve alterar o original, recalcular ou substituir hashes,
  ou transformar uma inferência de ferramenta em conclusão pericial.
- Aquisições reais e datasets de treinamento devem permanecer separados.

## Próximos blocos de engenharia

1. Adaptador de importação de pasta Avilla para o importador genérico do SIED.
2. Parser de manifesto, hashes e logs `.avilla` com vínculo de auditoria.
3. Parser dos TXT de coleta (dispositivo, contas, apps, contatos, localização,
   eventos e logs), mantendo o texto original junto do campo normalizado.
4. Reconhecimento de bancos/mídias/transcrições/OCR e vínculo ao artefato-fonte.
5. Tela de importação mostrando origem, ferramenta, versão, hashes e limitações.
6. Testes com dataset sintético/autorizado antes de qualquer aparelho real.

Até esses blocos existirem, o Avilla deve ser tratado como uma fonte externa de
aquisição a ser importada conscientemente, não como um pipeline interno do SIED.
