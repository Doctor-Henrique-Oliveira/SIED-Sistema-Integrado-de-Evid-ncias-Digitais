# Checkpoint: SIED Premium Experience (01/10/2026)

## OBJETIVO

Criar uma experiência explícita de entrada e liberação do workspace — splash, welcome,
console forense e modo operacional — preservando o design system navy/preto + dourado e todas
as invariantes forenses. A mudança é de frontend e configuração da janela; banco, parsers,
hashes, auditoria, importação, relatórios e migrações não foram alterados.

## IMPLEMENTADO

- Máquina de estados visual: `splash`, `welcome`, `creating-case`, `opening-case`,
  `booting-case`, `workspace` e `error`.
- Splash SIED em CSS/SVG, curta, offline e compatível com `prefers-reduced-motion`.
- Welcome sem sidebar, centrada em **Novo caso** e **Abrir caso existente**, com catálogo local
  em segundo plano visual e distinção explícita entre caso e extração.
- Novo caso preserva todos os campos e validações existentes, agora numa tela contida.
- Console de abertura apresenta somente marcos reais: pasta reconhecida, banco aberto, caso
  identificado, resultado de `verificar_auditoria`, sessão local e workspace pronto.
- Falha permanece visível e oferece tentar novamente ou voltar; não fecha a janela.
- Workspace só renderiza sidebar e topbar quando existe caso aberto.
- Dashboard ganhou hero hierárquico, sem oito colunas microscópicas.
- Conversas preserva as três colunas e dá mais área à lista e ao inspector técnico.

## JANELA

- Entrada: 1020 × 680, centralizada, sem resize e sem maximização.
- Workspace: resize/maximização liberados e tamanho calculado até 92% × 90% do monitor atual,
  sem fullscreen e sem cobrir deliberadamente a área inteira.
- Permissões adicionadas são somente as operações específicas de janela. Não foi adicionada
  permissão de filesystem, shell, processo, HTTP ou rede; a CSP não mudou.

## COMPONENTES E TOKENS

- `componentes/Experiencia.tsx`: `SplashScreen`, `ForensicBootConsole` e `ErroExperiencia`.
- `util/janela.ts`: alternância segura entre janela contida e workspace.
- `estilos.css`: assinatura de splash, welcome, console, estados e reduced motion sobre os
  tokens já incorporados pelo PR #14. Nenhuma dependência ou asset remoto.

## TESTES

- Splash termina e respeita reduced motion.
- Welcome mostra as duas ações e não mostra sidebar.
- Console usa a verificação real, mantém a ordem e libera o workspace.
- Erro é recuperável.
- Sem caso não há workspace; com caso o console precede sidebar/workspace.
- Suítes existentes de rotas, Conversas, importação, evidências e volume continuam cobrindo as
  funcionalidades preservadas.

Comandos:

```text
cd app
npm run typecheck
npm test
npm run build
```

Resultado local antes da CI: 14 arquivos de teste, 69 testes aprovados; build Vite aprovado.
O build Tauri local não liga porque esta estação não possui `link.exe`/MSVC. A validação do
instalador deve usar o job Windows/NSIS oficial da CI.

## DECISÕES

- O serviço não reabre automaticamente o último caso ao iniciar. Portanto: splash → welcome.
- A criação ou abertura bem-sucedida muda para console → workspace.
- O console não afirma hash, inventário ou percentual durante a abertura do caso; esses eventos
  pertencem ao console de importação já existente e só aparecem quando emitidos pelo backend.
- `verificar_auditoria` é chamado de fato antes de liberar o workspace.

## LIMITAÇÕES E PRÓXIMOS REFINAMENTOS

- A lista do catálogo local mostra até seis casos para não competir com as ações principais.
- O aviso de chunk JavaScript acima de 500 kB permanece não bloqueante e preexistente; divisão
  de bundle não faz parte desta missão.
- Revisar capturas em 1366 × 768 e 1920 × 1080 e repetir três ciclos com o caso sintético usando
  o instalador NSIS produzido pela CI antes de qualquer merge.

