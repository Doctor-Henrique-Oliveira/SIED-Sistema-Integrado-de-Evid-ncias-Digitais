# Prévia visual sintética do leitor documental

Com o servidor Vite do diretório `app` em execução (`npm run dev`), abra:

`http://127.0.0.1:1420/src/testes/previa-documentos.html`

Esta entrada de teste importa o `LeitorDocumento` real, mas substitui **somente nessa página** `api.lerDocumento` por cenários exclusivamente sintéticos. Não é o aplicativo Tauri, não confirma o backend e não abre um caso real. O aviso de prévia permanece visível.

Roteiro curto:

1. Selecione `NOTA-SINTETICA.txt` e pressione **Ler documento**.
2. Expanda **Proveniência do texto** e confira a distinção explícita de motor de prévia.
3. Alterne CSV, Markdown/HTML inerte e arquivo vazio.
4. Alterne os casos A/B: o texto anterior deve ser ocultado.
5. Inspecione ausência de hash, PDF sem suporte e respostas sintéticas de divergência, codificação recusada e falha IPC.

Os hashes e tamanhos são calculados dos conteúdos UTF-8 reais desta prévia usando WebCrypto/TextEncoder. Nenhum percentual ou processo forense é simulado. O fluxo normal de produção e seu build não importam esta entrada.
