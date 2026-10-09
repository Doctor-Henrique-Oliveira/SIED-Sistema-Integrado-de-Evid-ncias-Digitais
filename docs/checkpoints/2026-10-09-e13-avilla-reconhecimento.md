# E13 — reconhecimento local do Avilla Forensics

## Estado

E13 foi iniciada como reconhecimento controlado, ainda sem declarar um parser
funcional. O importador `avilla` permanece conservador (`NaoImplementado`) até
que exista uma árvore de extração real, autorizada e reproduzível.

## Evidência observada no ambiente

- `C:\Users\PCIVIL\OneDrive\Desktop\Avilla-Forensics-Free` contém a suíte
  instalada, ferramentas Android/iOS, `IPED-4.3.1_and_java_plugins`, bibliotecas
  e utilitários auxiliares.
- `C:\Users\PCIVIL\OneDrive\Desktop\AvillaForensics-main` contém código-fonte
  C# e material de referência do projeto Avilla.
- O atalho autorizado em `AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Forense Avancado`
  aponta para a instalação local.
- A fonte `FormIOS.cs` mostra que o módulo iOS chama `idevicebackup2.exe`,
  `idevicename.exe`, `idevice_id.exe` e `ideviceinfo.exe`, além do fluxo Apple
  Mobile Backup. Isso confirma ferramentas usadas pelo módulo, mas não define o
  formato final de uma extração.
- As pastas locais de aquisição encontradas em `C:\Avilla-Acquisitions`
  (`Case17072026084928` e `default\CopyAll`) estavam sem arquivos de extração.

## Decisão de integridade

Não foi copiado binário, código externo ou conteúdo de caso para o SIED. Também
não foi inventado nome de banco, manifesto, hash ou caminho de saída. Sem uma
extração autorizada não é possível criar fixture representativa, detecção
confiável ou parser E2E.

## Próximo insumo necessário

Uma pasta de extração Avilla autorizada, com a versão/opções usadas e os
manifestos/logs produzidos, ou amostra equivalente com autorização explícita.
Quando esse insumo existir, a implementação deve começar por inventário/hash,
detecção baseada na estrutura observada, fixture mínima e teste de origem intacta.
