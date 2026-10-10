# Inventário técnico dos projetos de origem

## 1. Data e escopo

Leitura realizada em 10/10/2026 sobre:

- cópia local do SIED em `C:\Users\Henrique\Documents\SIED-Sistema-Integrado-de-Evid-ncias-Digitais-main`;
- branch `main` do repositório SIPI/Delegacia-10V, commit `4e5090dc95aac8b5df44d6c4209887d9ba24b877`;
- documentação, manifests, rotas, migrations e buscas direcionadas no código.

Não foi executada auditoria integral linha a linha nem validação de produção. Os números são fotografia do código observado e não certificação de segurança.

## 2. SIED atual

### 2.1 Stack verificada

| Camada | Tecnologia |
|---|---|
| Desktop | Tauri 2 |
| Interface | React 19, TypeScript, Vite, Tailwind 4 |
| Backend local | Rust |
| Banco do caso | SQLite com FTS5 |
| Relatórios | Typst |
| Estado/consultas | TanStack Query e TanStack Virtual |

### 2.2 Organização verificada

Workspace Rust separado em crates de domínio, armazenamento, ingestão, documentos, mídia e relatório, mais o aplicativo Tauri. Essa separação tem valor e não deve ser achatada em um único backend genérico.

### 2.3 Capacidades existentes relevantes

- criação e abertura de casos locais;
- cadastro de dispositivos, incluindo IMEI 1, IMEI 2 e validação Luhn;
- importação com inventário, SHA-256 e log;
- importadores genérico, IPED, ADB e ALEAPP/iLEAPP documentados;
- parsers de SMS/MMS, contatos, chamadas e WhatsApp Android documentados;
- arquivos e referências de origem normalizados;
- busca e FTS;
- conversas, mídias, contatos, localizações e linha do tempo;
- seleção de evidências;
- relatório PDF e arquivo de hash;
- auditoria local;
- processamento documental local em evolução;
- preservação da pasta de origem e testes de integridade.

### 2.4 Evidência de maturidade

Na fotografia analisada foram encontrados aproximadamente:

- 116 arquivos Rust;
- 347 anotações de testes Rust;
- 24 arquivos de testes de interface;
- dezenas de comandos Tauri;
- documentação de arquitetura, continuidade, handoff, checkpoints e roadmap.

Esses números ajudam a dimensionar o ativo, mas não substituem cobertura, execução dos testes ou revisão de qualidade.

### 2.5 IMEI no SIED atual

O código atual:

- armazena IMEI 1 e IMEI 2 no dispositivo;
- normaliza para dígitos;
- exige 15 dígitos e valida Luhn;
- evita IMEI 1 e IMEI 2 idênticos;
- encontra candidatos rotulados em documentos, também com validação conservadora;
- deixa explícito que a validação não confirma TAC, fabricante, modelo, existência ou titularidade.

Não foi identificada, nesta leitura, uma base operacional completa de enriquecimento IMEI/TAC incorporada ao núcleo atual.

### 2.6 Invariantes úteis a herdar

- origem somente leitura;
- hash e inventário antes da interpretação;
- registros normalizados com referência à origem;
- auditoria local sem conteúdo sensível desnecessário;
- dados derivados reconstruíveis;
- suporte de formato baseado em amostra e teste, não em aparência;
- Avilla somente após amostras reais autorizadas e documentadas.

## 3. SIPI / Delegacia-10V

### 3.1 Stack verificada

| Camada | Tecnologia |
|---|---|
| Aplicação | TanStack Start, React 19 e TanStack Router |
| Estilo | Tailwind 4 e componentes Radix |
| Backend/dados | Supabase |
| Hospedagem | Cloudflare/Wrangler |
| Testes | Vitest |

### 3.2 Capacidades existentes relevantes

- procedimentos/inquéritos e pessoas relacionadas;
- representações;
- boletim de ocorrência;
- agenda e oitivas, responsáveis, conflitos e lembretes;
- veículos apreendidos, custódia e movimentações;
- objetos apreendidos, fotos e movimentações;
- localização operacional, pessoas, endereços, diligências e rotas;
- alertas;
- auditoria;
- administração de usuários;
- papéis, status de autorização e acesso a sigilo;
- preferências e filtros salvos.

### 3.3 Evidência de maturidade

Na fotografia da `main` analisada:

- 75 migrations Supabase;
- 65 arquivos de rotas;
- 33 arquivos de teste TypeScript/TSX;
- RLS, políticas, funções e gatilhos de auditoria documentados em migrations;
- tela de módulos já operacional para diversas áreas.

### 3.4 Permissões existentes

O código diferencia status de autorização, função institucional, papéis internos e permissão para casos sigilosos. Há regras específicas para visualizar, criar, editar, movimentar, liberar ou excluir entidades.

Essa lógica serve como referência, mas não deve ser copiada sem revisão. O novo produto precisa separar administração da plataforma, permissão administrativa e autorização sobre evidência forense.

### 3.5 IMEI no SIPI

Não foi encontrada funcionalidade de consulta de IMEI ou catálogo TAC. Existem números de série em objetos apreendidos, mas isso não equivale a um módulo IMEI.

## 4. Diferenças estruturais

| Tema | SIED | SIPI | Consequência |
|---|---|---|---|
| Forma | Desktop | Web | O novo shell precisa acomodar módulo conectado e núcleo local |
| Persistência | SQLite/pastas locais | PostgreSQL/Supabase | Não fundir tabelas diretamente |
| Uso | Caso forense individual | Colaboração institucional | Identidade e autorização têm escopos diferentes |
| Evidência bruta | Local | Não é foco | Não enviar extrações ao Supabase por padrão |
| Disponibilidade | Offline | Depende de rede | Interface deve declarar estado online/offline |
| Auditoria | Local ao caso | Centralizada | Eventos continuam em domínios próprios, com correlação |
| Atualização | Instalador desktop | Deploy web | Definir estratégia única de release e compatibilidade |

## 5. Conclusão do inventário

A união deve ocorrer no produto, na navegação, na identidade e nos contratos. Ela não deve ocorrer por cópia indiscriminada de arquivos nem por tentativa de usar um único banco para todos os dados.

O SIED fornece o núcleo forense maduro. O SIPI fornece regras e módulos administrativos maduros. O novo projeto deve preservar ambos os ativos e criar uma fronteira explícita entre eles.
