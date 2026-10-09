# Checkpoint — E12: desempenho, retomada e limites

## Entregue

- limites locais de arquivos, bytes e workers de hash, aplicados antes e durante o inventário;
- hashing paralelo controlado em janelas pequenas, com no máximo oito workers;
- cancelamento cooperativo em todos os workers, sem escrever na origem;
- `checkpoint-retomada.json` derivado, atualizado de forma segura por arquivo temporário;
- leitor validado de checkpoint (`ler_checkpoint`) que recusa schema/estado/contadores fora do contrato;
- avaliação de compatibilidade da fonte (`avaliar_retomada`), que exige conferência humana e recusa fonte divergente;
- estados `iniciada`, `inventariando`, `inventario_concluido`, `cancelada` e `interrompida`;
- bloqueio persistente de uma segunda importação da mesma origem/dispositivo/importador
  enquanto a primeira está `em_andamento`, evitando duplicação após reinício;
- limites e progresso preservados no log da importação;
- testes sintéticos para lote paralelo, cancelamento e volume de hash.

## Segurança da retomada

O checkpoint é somente um marcador derivado e nunca autoriza declarar sucesso. Uma
interrupção registra o estado parcial, mas a continuação automática da mesma transação
continua desabilitada até existir uma operação explícita que reabra o contexto e confira
hashes antes de evitar qualquer gravação duplicada.

O leitor público permite que uma camada de recuperação mostre o estado ao operador sem
precisar interpretar JSON diretamente. Estados parciais permanecem diagnósticos; o banco
do caso e sua auditoria continuam sendo a fonte de verdade.

## Validação

`cargo fmt --all` passou. `cargo metadata --no-deps` e a validação TypeScript/build da
interface permanecem independentes deste crate. A compilação/testes Rust locais continuam
bloqueados por `link.exe`/MSVC ausente no ambiente Windows atual; não foi instalado nenhum
toolchain nem alterado código para contornar o bloqueio.

## Próximos blocos da E12

Benchmark de grande volume em runner Windows, recuperação operacional do instalador e
retomada transacional conferida devem ser validados antes de marcar E12 como concluída.
