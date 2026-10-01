# Checkpoint: aviso de unidade de rede mapeada (01/10/2026)

Dívida técnica da auditoria (HANDOFF §25, item F5), na construção contínua. Sem migração.

## IMPLEMENTADO

- **Detecção pelo sistema operacional** (`app/src-tauri/src/disco.rs`, comando `pasta_em_rede`):
  - Windows: `GetDriveTypeW` na raiz da unidade; `DRIVE_REMOTE` (unidade mapeada) ou caminho UNC
    contam como rede.
  - Linux: tipo do sistema de arquivos do ponto de montagem mais próximo em
    `/proc/self/mountinfo` (NFS, SMB/CIFS, SSHFS, 9p...).
  - Pasta que ainda não existe: vale a pasta existente mais próxima acima dela.
  - Fora da thread da janela (unidade desconectada pode demorar); erro = "não é rede".
- **Novo caso**: o aviso que já aparecia para OneDrive/Dropbox/UNC aparece também para unidade
  mapeada (`localDaPasta` junta o caminho e a resposta do sistema).
- **Dashboard do caso aberto**: "Alertas e pendências" avisa quando a pasta do caso está num
  compartilhamento de rede ou pasta sincronizada, e orienta a fechar e copiar para disco local.
  Nada é bloqueado.

## TESTES

- 284 Rust (+3 medições ignoradas), 62 vitest: todos passam; fmt e clippy limpos. O código
  Windows foi conferido com `cargo check --target x86_64-pc-windows-gnu`; a CI roda no Windows.
- `disco.rs`: pasta temporária (e subpasta inexistente) é local; mountinfo SINTÉTICO com raiz
  ext4, `/mnt/casos` CIFS, submontagem ext4 dentro dele e ponto com espaço (`\040`): a montagem
  mais próxima decide e prefixo de texto não conta como prefixo de caminho; no Windows, UNC.
- Vitest: `localDaPasta` (unidade mapeada só pela resposta do sistema) e `PaginaNovoCaso` com
  pasta `Z:\SIED\Casos` avisando quando o sistema diz rede e não avisando quando diz local.

## ARQUIVOS

- `app/src-tauri/src/disco.rs` (novo), `comandos.rs`, `lib.rs`
- `app/src/api/cliente.ts`, `consultas.ts`, `util/formatar.ts` e teste
- `app/src/rotas/PaginaNovoCaso.tsx` e `PaginaNovoCaso.test.tsx` (novo), `PaginaCaso.tsx`

## DECISÕES

- Chamada direta a `GetDriveTypeW` (kernel32) em vez de uma dependência nova.
- Só aviso, nunca bloqueio (mesma decisão do aviso de nuvem): há delegacias que trabalham em
  rede por norma; a orientação fica visível.

## PROBLEMAS

Nenhum em aberto.

## COMO EXECUTAR / COMO TESTAR

```text
cargo test --workspace
cd app && npm test
```

No Windows: mapear uma pasta compartilhada como `Z:` e escolher `Z:\...` no Novo caso.

## LIMITAÇÕES

- Sem captura de tela no app real: o contêiner Linux não monta compartilhamento SMB. A tela usa o
  mesmo componente de aviso já capturado nas fases anteriores.

## PRÓXIMA FASE

Da dívida técnica resta F3 (assinatura/âncora externa), que depende de decisões do usuário.
Avilla segue aguardando a árvore real.
