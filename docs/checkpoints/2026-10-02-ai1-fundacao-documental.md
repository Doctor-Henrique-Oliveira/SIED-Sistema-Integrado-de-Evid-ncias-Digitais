# Checkpoint — AI-1: fundação documental local

Data: 02/10/2026. Estado: **primeiro incremento implementado; Rust não validado; AI-1 não concluída**.

Autorização: o responsável solicitou explicitamente iniciar a AI-1. Branch dedicada
`feat/mdfml-ai1-documentos-locais`, a partir da `main`
`f3123f548852ec6962c34f1c01e97e8ce09cef5c`. A AI-0 permanece separada no PR #24,
sem cherry-pick, merge ou alegação de que sua validação foi concluída por este trabalho.
Planejamento: [ROADMAP-MDFML.md](../ROADMAP-MDFML.md), especialmente §§6 e 18.

## 1. Entrega efetiva

Biblioteca Rust interna `sied-documentos`, usando dependências já existentes no workspace.
Primeiro motor: `texto_utf8_nativo`, versão `0.1.0`. Não é modelo de IA nem motor OCR.

- Leitura por `FonteSomenteLeitura`, sem resolver raízes ou acessar disco por outra via.
- Extensões admitidas: `txt`, `csv`, `tsv`, `json`, `xml`, `md`, `log` em modo automático.
- SHA-256 completo e tamanho do inventário obrigatórios; comparação com todos os bytes lidos
  antes da interpretação. Ausência de hash não vira sucesso ou integridade presumida.
- UTF-8 estrito: preserva BOM, finais de linha, composição Unicode e conteúdo exato.
  Não normaliza, não usa conversão com perdas e não publica resultado parcial.
- Assinaturas PDF/ZIP, controles binários incompatíveis e UTF-8 inválido são recusados.
- Texto pronto carrega caso, extração, arquivo, caminho, hash da origem, motor/versão,
  horário real e hash do conteúdo derivado. Texto nativo não inventa página, região ou confiança.
- Estados explícitos: pronto, sem suporte, origem divergente/indisponível, hash ausente,
  limite excedido, cancelado e falhou. Falhas não retornam conteúdo parcial nem erro bruto da fonte.
- Cancelamento cooperativo monotônico; prazo cooperativo padrão/máximo de 5 segundos.
- Limite de 2 MiB de bytes retidos/texto, com um byte adicional não retido para detectar excesso.

**Não entregue:** PDF, Office, OCR, imagens incorporadas, entidades, pesquisa em derivados,
armazenamento/cache, jobs persistidos, auditoria documental, novo comando Tauri ou interface.
Solicitar OCR ou formato não suportado produz recusa explícita, não processamento simulado.

## 2. Limites do contrato

O chamador deve construir o pedido a partir do inventário autoritativo do caso aberto e
vincular caso, extração, arquivo e fonte. A biblioteca não consulta banco nem certifica
associação entre identificadores fornecidos. Não aceitar um pedido arbitrário do frontend
como se fosse inventário. Ainda não existe esse adaptador desktop.

Confinamento/localidade dependem da implementação confiável de `FonteSomenteLeitura`.
A biblioteca não é sandbox para chamador ou fonte arbitrários. A fonte de pasta existente
possui controles lexicais e recusa do symlink final; esta entrega não acrescenta uma garantia
por handle, sem corrida, para todos os ancestrais.

Cancelamento e prazo são verificados antes/depois de E/S e interpretação/publicação.
Não há interrupção forçada de uma abertura/leitura bloqueada. O teto de texto não é teto de RSS,
payload JSON, prefetch ou leitura física da fonte. Textos acima de 2 MiB ficam explicitamente
limitados neste incremento, não truncados silenciosamente.

`sha256_texto` considera apenas os bytes do texto. Como não há transformação, coincide com
o SHA-256 da origem neste motor; não inclui horário/IDs e não altera `hash_registro` histórico.
`SemSuporte` não significa integridade conferida: várias recusas acontecem antes da leitura.

## 3. Testes e medição preparados

Arquivo: `crates/sied-documentos/tests/documentos.rs`. **13 testes normais e uma medição ignorada**.
Só dados sintéticos e temporários; reutiliza `testes/rust/caso_sintetico.rs` e o fluxo normal de
importação para os testes de inventário, sem banco/caso de produção.

Cobertura preparada:

- Texto exato de dez documentos sintéticos, incluindo vazio, UTF-8/BOM/CRLF/Unicode,
  extensão maiúscula e conteúdo adversário que permanece texto inerte.
- Identidade e hash provenientes do inventário persistido, motor/versão e horário real.
- Origem sem mudanças nos hashes e datas de modificação dos arquivos; inventário, extração,
  auditoria e hashes dos registros preservados nos testes de integração.
- Divergência de hash/tamanho provocada no pedido, nunca adulterando a evidência.
- Recusa de OCR/formatos/contêineres, hash inválido/ausente, ID inválido e tamanho declarado
  excessivo sem abertura da fonte.
- Origem com tamanho subdeclarado, teto exato de 2 MiB, cancelamento prévio/durante leitura,
  prazo zero, erros de abertura/leitura e codificações/binários recusados sem texto parcial.

A medição faz 20 leituras reais de um texto sintético de 2 MiB importado normalmente.
Emite durações por análise e tamanhos JSON; total inclui verificações/serialização.
Cache do sistema operacional não controlado; RSS não medido; SLO não estabelecido.
Não é benchmark de OCR, PDF, pesquisa ou volume completo de caso.
O teste de prazo cobre prazo zero; não prova contenção temporal de E/S bloqueada.

## 4. Validação executada e pendente

| Verificação local | Resultado |
|---|---|
| `npm.cmd test -- --maxWorkers=2`, em `app` | **70 testes / 14 arquivos passaram**. |
| `npm.cmd run build`, em `app` | **Passou**, TypeScript + Vite. Aviso não bloqueante de chunk acima de 500 kB. |
| `cargo fmt --all --check` | **Passou**. Não é compilação ou execução dos testes Rust. |
| `cargo update --offline --workspace` | **Passou**; lockfile gerado pelo Cargo, só acrescenta o pacote local. Sem troca de versões externas. |
| `cargo metadata --offline --locked --no-deps --format-version 1` | **Passou**, reconhece o novo pacote. Não é type-check Rust. |
| `git diff --check` | **Passou**. |

Dependência real para validação Rust local: toolchain nativo MSVC de C/linkedição e
Windows SDK/UCRT. `cl.exe`/`link.exe` não estão disponíveis no PATH e o caminho padrão
do SDK Windows 10 não existe neste ambiente. Nenhuma instalação ou mudança de segurança
corporativa foi feita; não foi repetida compilação Tauri nem tentativa de reparar o ambiente.

Rust/Clippy/medição e build nativo completo **continuam pendentes**, não aprovados por
inspeção estática ou pelo build da interface. Foram adicionados passos obrigatórios na CI Linux:

```text
cargo test --locked -p sied-documentos
cargo clippy --locked -p sied-documentos --all-targets -- -D warnings
cargo test --locked --release -p sied-documentos --test documentos medicao_20_leituras_reais_de_texto_sintetico_de_2mib -- --ignored --exact --nocapture
```

Gates completos existentes de workspace, Windows, frontend, geração/conferência de TypeScript
e instaladores foram preservados. Não se aguardou nem fez polling de jobs externos.
O build/testes locais acima pertencem à base da `main` desta branch, não validam a AI-0 do PR #24.

Sem fronteira frontend nova, não há novos contratos TypeScript a gerar neste incremento.
Quando existir integração, os contratos deverão continuar gerados pelo Rust, nunca escritos
manualmente. O gate atual de `git diff --exit-code -- app/src/api/tipos` não foi enfraquecido.

## 5. Revisão arquitetural e preservação

O modelo de ameaças AI-1 foi produzido com a skill de threat modeling e uma revisão independente
de contexto fresco. Influenciou a conferência de hash antes da interpretação, limites e a
documentação das obrigações do chamador. Não representa auditoria concluída ou vulnerabilidade confirmada.

Artefato suplementar persistente do Codex Security, fora do checkout:
`artifacts/01_context/threat_model_ai1_texto.md`, SHA-256
`ab3443264ae328f5a79df6c1afbfdffa212cf0538d7e1214c00135ada96c25bb`.
Não substitui o modelo da AI-0 nem atualiza um modelo compartilhado de outras fases.

Nenhuma alteração em parsers/Avilla, migrações, fatos/bancos existentes, hashes históricos,
auditoria, evidências, relatórios, permissões/CSP, frontend, Design System ou shell.
Nenhum modelo, dependência externa nova, processo de OCR, rede, caso real ou `.claude/` utilizado.

## 6. Próximas entregas da AI-1, ainda não executadas

1. Executar e revisar a validação Rust/Clippy/medição deste incremento; manter PR draft sem merge.
2. Estudar motores locais de PDF/OCR: licença, origem/integridade, instalação offline, limites,
   isolamento de falhas, corpus sintético e critérios mensuráveis. Não presumir nomes/modelos aprovados.
3. Desenhar/validar adaptador case-owned, persistência versionada de derivados e auditoria, sem
   alterar fatos históricos; definir APIs e contratos TypeScript gerados quando forem necessários.
4. Implementar PDF/OCR progressivamente, preservando página/região observadas e ausência de
   confiança quando o motor não a fornece. Medir recursos, falhas e fidelidade.
5. Integrar consulta dos derivados e entidades candidatas revisáveis com proveniência e controle
   humano, sem promoção automática a evidência ou alteração de relatório.

AI-2 e demais fases **não iniciadas**. AI-1 inteira **não concluída**. Sem merge automático.
