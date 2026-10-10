# Arquitetura alvo

## 1. Decisão recomendada

Adotar uma arquitetura híbrida em um aplicativo desktop Tauri:

- **shell e navegação únicos**;
- **módulos administrativos conectados** ao backend Supabase;
- **núcleo forense local** em Rust, SQLite e pasta de caso;
- **contrato de integração estreito**, sem replicação automática de evidência bruta.

## 2. Visão lógica

```text
┌───────────────────────────────────────────────────────────────┐
│                      SIED Desktop (Tauri)                     │
│                                                               │
│  ┌──────────────┐   ┌──────────────────────────────────────┐  │
│  │ Shell comum  │   │ Interface React                     │  │
│  │ autenticação │   │ módulos, navegação, design system   │  │
│  └──────┬───────┘   └──────────────┬───────────────────────┘  │
│         │                           │                          │
│  ┌──────▼────────────────┐   ┌──────▼──────────────────────┐  │
│  │ Serviços conectados   │   │ Serviços forenses locais   │  │
│  │ procedimentos/agenda  │   │ casos/importação/busca     │  │
│  │ autorização/auditoria │   │ evidências/relatórios      │  │
│  └──────────┬────────────┘   └──────────┬──────────────────┘  │
└─────────────┼───────────────────────────┼─────────────────────┘
              │ TLS                       │ sistema de arquivos
     ┌────────▼─────────┐        ┌────────▼────────────────────┐
     │ Supabase         │        │ Caso local                  │
     │ Auth/Postgres    │        │ SQLite + fontes + hashes    │
     │ RLS/auditoria    │        │ derivados + relatórios     │
     └──────────────────┘        └─────────────────────────────┘
```

## 3. Por que não usar apenas Supabase

Extrações podem conter grandes volumes, bancos internos de aplicativos, mídias, documentos e informações altamente sensíveis. Enviá-las ao backend central aumenta superfície de ataque, custo, tempo, dependência de rede e impacto de incidente.

O Supabase é adequado para colaboração administrativa, não deve virar depósito automático da extração.

## 4. Por que não tornar tudo local

Procedimentos e agenda precisam de colaboração, permissões centralizadas, atualização por vários usuários e histórico institucional. Converter todo o SIPI para bancos locais isolados eliminaria parte central do seu valor e criaria um problema de sincronização muito maior.

## 5. Camadas

### 5.1 Shell

Responsabilidades:

- abertura e carregamento;
- autenticação e troca de sessão;
- seleção de unidade quando aplicável;
- central de módulos;
- estado de conectividade;
- atualização da aplicação;
- configuração não sensível;
- observabilidade técnica sem conteúdo de evidência.

O shell não executa regras internas dos módulos.

### 5.2 Interface compartilhada

Responsabilidades:

- tokens visuais e acessibilidade;
- componentes básicos;
- tratamento consistente de carregamento, vazio, erro e permissão;
- navegação e breadcrumbs;
- avisos úteis e acionáveis;
- distinção visual entre dado remoto, local e não sincronizado.

Não criar uma biblioteca abstrata enorme antes de existirem dois consumidores reais.

### 5.3 Domínio administrativo

Responsabilidades:

- procedimentos;
- pessoas e vínculos administrativos;
- agenda;
- papéis, unidade e sigilo;
- referências a dispositivos e casos forenses;
- auditoria das operações remotas.

### 5.4 Domínio forense

Responsabilidades:

- caso local e dispositivos;
- importação e inventário;
- hash;
- normalização e referências de origem;
- busca, leitura e revisão;
- seleção de evidências;
- relatório e auditoria local.

O núcleo deve permanecer operável sem conexão depois de aberto e autorizado conforme a política definida.

### 5.5 Catálogo IMEI

Responsabilidades:

- importar um snapshot autorizado;
- validar estrutura e integridade;
- consultar por IMEI/TAC;
- explicar a fonte do resultado;
- manter versões e permitir rollback;
- não inferir titularidade.

## 6. Estrutura recomendada do novo repositório

Começar simples, preservando separação de domínios:

```text
sied/
├── .github/
│   ├── workflows/
│   └── CODEOWNERS
├── docs/
│   ├── adr/
│   ├── produto/
│   ├── seguranca/
│   └── operacao/
├── app/
│   ├── src/
│   │   ├── app/
│   │   ├── modules/
│   │   │   ├── procedimentos/
│   │   │   ├── agenda/
│   │   │   ├── imei/
│   │   │   └── forense/
│   │   └── shared/
│   └── src-tauri/
├── crates/
│   ├── sied-dominio/
│   ├── sied-armazenamento/
│   ├── sied-ingestao/
│   ├── sied-documentos/
│   ├── sied-midia/
│   ├── sied-relatorio/
│   └── sied-integracao/
├── supabase/
│   ├── migrations/
│   ├── seed/
│   └── tests/
├── testes/
│   ├── fixtures-sinteticas/
│   └── e2e/
├── Cargo.toml
├── package.json
├── README.md
└── SECURITY.md
```

Não criar múltiplos aplicativos ou microserviços no início. Um repositório e um executável reduzem custo operacional. A separação deve ser de módulos e contratos.

## 7. Contrato entre nuvem e caso local

O vínculo mínimo recomendado contém:

```text
ForensicCaseLink
- id remoto
- procedure_id
- device_id remoto opcional
- local_case_id
- workstation_id pseudônimo
- local_case_fingerprint
- status operacional
- created_by / created_at
- last_published_at
- report_reference opcional
- report_sha256 opcional
```

O backend remoto não recebe caminho local absoluto como dado de negócio. Caminhos revelam nomes de usuário e topologia da estação e não funcionam em outro computador.

## 8. Publicação controlada

Publicar é uma ação explícita e diferente de sincronizar interface.

Exemplo de conteúdo publicável:

- que existe um caso forense vinculado;
- dispositivo relacionado;
- estado da análise;
- hash do relatório;
- data de emissão;
- autor responsável;
- identificador de custódia ou protocolo;
- cópia do relatório, somente se houver política e ação explícitas.

Conteúdo não publicável automaticamente:

- bancos de aplicativos;
- mensagens normalizadas;
- mídias;
- miniaturas;
- termos pesquisados;
- notas privadas do analista;
- credenciais ou tokens;
- caminhos locais.

## 9. Funcionamento offline

| Situação | Comportamento esperado |
|---|---|
| Sem rede antes do login | Exibir limitação; permitir somente modo local se a política e uma credencial local válida o autorizarem |
| Rede cai em módulo administrativo | Preservar formulário localmente sem afirmar que foi salvo; oferecer nova tentativa controlada |
| Rede cai durante análise forense | Análise continua; publicação fica pendente |
| Sessão expira | Bloquear chamadas remotas; não corromper nem fechar à força o caso local |
| Backend indisponível | Mostrar estado objetivo, sem substituir por dados inventados |

O modo offline autenticado é uma decisão de segurança ainda aberta; não deve ser improvisado com token eterno.

## 10. Compatibilidade e versionamento

- migrations remotas são incrementais e revisáveis;
- schema do caso local mantém migrações próprias;
- contratos compartilhados têm versão explícita;
- o aplicativo verifica compatibilidade mínima do backend;
- catálogo IMEI tem versão independente;
- relatórios registram versões relevantes de parser, schema e catálogo usados.

## 11. Observabilidade

Separar três trilhas:

1. **log técnico local:** diagnóstico sem conteúdo de evidência;
2. **auditoria local do caso:** ações forenses e integridade;
3. **auditoria institucional remota:** autenticação, autorização e alterações administrativas.

Um identificador de correlação pode unir eventos sem copiar conteúdo sensível.

## 12. Decisões arquiteturais que exigem ADR

- modo offline autenticado;
- criptografia do banco e dos anexos locais;
- armazenamento seguro de tokens;
- publicação de relatórios;
- política de backup e restauração;
- atualização automática ou administrada;
- instalação por usuário ou por máquina;
- múltiplas unidades/tenancy;
- operação em estações compartilhadas;
- origem e licença do catálogo IMEI.
