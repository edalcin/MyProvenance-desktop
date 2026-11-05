# MyProvenance — Versão 2 (stand-alone, Rust puro)

> Documento de planejamento. Status: **consulta/decisão** — nenhum código escrito. Data: 2026-11-04.

## Objetivo

Transformar o MyProvenance em app **desktop stand-alone**, **monousuário**, para **Windows** (foco atual; macOS/Linux fora do escopo por ora). Mapa: [Wayfinder #1](https://github.com/edalcin/MyProvenance-desktop/issues/1).

## Estado atual (v1, referência)

- SvelteKit 3 + `adapter-node` + `better-sqlite3`; ~13k linhas TS/Svelte.
- `schema.sql`: 8 tabelas — `registros`, `agentes`, `entidades`, `atividades`, `atividade_entidades_usadas`, `usuarios`, `sessoes`, `registro_compartilhamentos`.
- Módulos multiusuário: `auth`, `admin-auth`, `rate-limit`, compartilhamento.
- Export JSON por registro já existe: `gerarJsonExportado()` em `src/lib/export.ts` → `RegistroExportado`; import em `src/lib/server/db/repositories/import.ts`.
- UI dependente de web: TipTap (`rich-text-editor.svelte`), Mermaid (`mermaid-diagram.svelte`).

## Alternativas avaliadas

| Opção | Tamanho | Mudança | Veredito |
|---|---|---|---|
| Tauri 2 (shell Rust + UI Svelte) | ~5–15 MB | adapter-static, lógica para client/commands | Viável; WebView do sistema (Linux fraco) |
| **Rust puro** | ~5–15 MB | Reescrita total | **Escolhido** |
| Electron | ~150 MB+ | Mínima | Descartado (tamanho/RAM) |
| Node SEA / Bun compile | ~50–100 MB | Mínima; trocar `better-sqlite3` por `node:sqlite` | Descartado |
| PWA client (SQLite WASM) | 0 install | Sem servidor | Descartado (dados presos ao browser) |

## Decisões tomadas

1. **Rust puro**, sem WebView.
2. **Sem SQLite**: armazenamento em **JSON**, **um arquivo por registro**.
3. **TipTap → editor Markdown + preview** (perda aceita).
4. **Mermaid renderizado → grafo nativo** na tela; Mermaid continua só como **texto no export** (perda aceita).
5. Removidos: `usuarios`, `sessoes`, `registro_compartilhamentos`, auth, admin, rate-limit, compartilhamento.
6. Toolkit de UI: **egui** (`eframe`).
7. Alvo: **só Windows x64** (`.msi`). macOS/Linux e assinatura fora do escopo por ora.
8. **Sem i18n**: UI só pt-BR (ADR-0007 mantida).
9. **v1 web congelada** após a v2; v2 importa JSON da v1 em sentido único.

## Armazenamento JSON

- Formato ([ADR-0014](adr/0014-v2-formato-json-schemaversion-4.md)): `schemaVersion: 4`, mesma linhagem e envelope do `RegistroExportado` da v1; arquivos da v1 (1–3) entram por migração 3→4. Nome: `<slug-do-título>-<últimos 8 hex do id>.json`, fixo na criação.
- **Escrita atômica obrigatória**: grava `.tmp` → `rename` (`tempfile::persist`).
- Validação na leitura (substitui FKs do SQLite): `serde` (`deny_unknown_fields`) + `garde`; integridade referencial, grafo acíclico, cardinalidade/Revisão — detalhe na ADR-0014.
- Local ([Pasta de dados da v2](https://github.com/edalcin/MyProvenance-desktop/issues/5)): **uma pasta ativa**, padrão `Documentos\MyProvenance`, trocável nas Configurações (troca não move arquivos). Config em `%APPDATA%\MyProvenance\config.json`. Só o nível de cima da pasta é lido.
- Listagem/busca: carregar todos os arquivos em memória na inicialização e no botão "Recarregar" (sem monitor de arquivos; ok até alguns milhares de registros).
- Conflito: `mtime` guardado na leitura e comparado antes de gravar; se mudou/sumiu fora do app, perguntar (Sobrescrever / Recarregar).
- Arquivo inválido: ignorado, aviso com nome + erro; nunca alterado. `.tmp` ignorados.

| Aspecto | 1 arquivo por registro | JSON único |
|---|---|---|
| Corrupção | Afeta 1 registro | Afeta tudo |
| Escrita | Pequena | Regrava tudo |
| Sync (Git/Dropbox) | Bom, diff legível | Conflitos |
| Compartilhar | Envia o arquivo | Precisa exportar |

Futuro opcional: PROV-JSON (W3C) como formato nativo — mais interoperável, mais mapeamento.

## Stack Rust

| Função | Crate |
|---|---|
| App/UI | `eframe` + `egui` 0.37, `default-features = false`, backend `glow` (~5,6 MB vs ~11 MB `wgpu`) |
| Markdown | `TextEdit` multilinha (editor), `egui_commonmark` 0.26 (preview) |
| Grafo | `egui::Painter` + `egui::Scene` + `rust-sugiyama` 0.4 + `petgraph` 0.8 (não `egui_graphs`: preso a egui 0.36, layout sem camadas de DAG) |
| Modelo/JSON | `serde`, `serde_json` |
| Validação | `garde` |
| IDs | `uuid` (feature `v7`; leitura aceita qualquer versão — a v1 tem UUIDv4) |
| Datas | `jiff` (ISO 8601, exibição no fuso do sistema) |
| Slug do nome do arquivo | `unicode-normalization` (NFD, igual ao `slugify` da v1) |
| Ícones | Boxicons vendorizado como fonte (egui 0.37 deixou emoji opt-in; a fonte padrão não tem `→`/`←`) |
| Arquivos | `directories`, `tempfile`, `rfd` |
| Migração v1 (HTML→Markdown) | `htmd` |
| Empacotamento | `cargo-packager` 0.11 (.msi, WiX baixado automaticamente) |

Release profile: `lto = "fat"`, `codegen-units = 1`, `strip = true`, mais `RUSTFLAGS=-C target-feature=+crt-static`. O protótipo deu 9,65 MB, sem dependência de `VCRUNTIME`. Detalhes: `docs/research/egui-viabilidade.md`.

UI ([Protótipo da Etapa 1](https://github.com/edalcin/MyProvenance-desktop/issues/8), código na tag `prototipo-etapa1`): **telas**. A tabela de Registros ocupa a tela toda; um clique abre a tela do Registro, que tem "Voltar". A descrição fica em abas Editar/Visualizar. A **gravação é explícita** (botão Gravar e Ctrl+S, com marcador "não gravado"); Voltar ou fechar com edição pendente pergunta Gravar/Descartar/Cancelar.

Toolkits considerados: egui (escolhido — simples, ecossistema), Iced (API instável), Slint (licença GPLv3/royalty-free/comercial), Dioxus/Tauri (WebView, não é Rust puro).

### Mapeamento v1 → v2

| v1 | v2 |
|---|---|
| Svelte + bits-ui + Tailwind | egui |
| TipTap | textarea Markdown + preview |
| mermaid (render) | grafo nativo |
| `gerarRelatorioMarkdown`, `gerarDiagramaMermaid`, `gerarJsonExportado` | port direto |
| zod | serde + garde |
| marked + sanitize-html | pulldown-cmark (`ammonia` só se renderizar HTML) |
| `DB_PATH` | `directories` |

## Arquitetura

```mermaid
flowchart LR
  UI[UI egui] --> App[Estado + regras]
  App --> Store[Store JSON<br/>serde + escrita atômica]
  App --> Exp[Export<br/>Markdown / Mermaid / JSON]
  Store --> FS[(Pasta de dados<br/>1 .json por registro)]
```

```
myprovenance/
  Cargo.toml
  src/
    main.rs        # eframe::run_native
    model.rs       # Registro, Agente, Entidade, Atividade
    store.rs       # listar/ler/gravar, escrita atômica, "versao"
    export.rs      # Markdown, Mermaid, JSON
    ui/
      lista.rs
      registro.rs  # formulário + editor Markdown
      grafo.rs
```

## Etapas

1. **Base do app**: o protótipo está feito (tag `prototipo-etapa1`). Falta o crate real na raiz: `model.rs` e `store.rs` partem da tag; telas de lista e de registro no layout escolhido; gravação explícita. Testar acentos ABNT2 no `TextEdit`.
2. **Port da lógica**: `cardinalidade.ts`, `relacao.ts`, `report.ts`, `mermaid.ts`, `export.ts`. Usar `*.test.ts` da v1 como especificação dos testes Rust.
3. **Grafo + export**.
4. **Acabamento**: tema claro/escuro, "Importar da v1…" ([decisão](https://github.com/edalcin/MyProvenance-desktop/issues/9): só ação explícita, `rfd` múltiplo + resumo; recusa id existente, migração inválida e arquivo v4). Sem i18n (ADR-0007 mantida).
5. **Distribuição**: GitHub Actions `windows-latest`, `.msi` no Release. Assinatura Windows (SmartScreen) e builds macOS/Linux fora do escopo por ora.

## Decisões pendentes

Rastreadas no mapa [Wayfinder #1](https://github.com/edalcin/MyProvenance-desktop/issues/1) (Agentes, repositório, Rascunho/Finalizado, pasta de dados, formato JSON v2, viabilidade egui, protótipo, import da v1).

Decidido: v1 web congelada após a v2 (só correções de segurança); v2 importa JSON da v1 em sentido único.
