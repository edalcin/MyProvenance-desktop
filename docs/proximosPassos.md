# Próximos Passos

## v2 desktop (Rust/egui, só Windows) — em andamento

Plano: `docs/versao2.md`. Mapa canônico: [Wayfinder: MyProvenance v2 desktop](https://github.com/edalcin/MyProvenance-desktop/issues/1) (inclui execução até o `.msi` no Release). Estado em 2026-11-07:

- [x] Mapa criado e migrado de `edalcin/MyProvenance` para este repo.
- [x] Viabilidade egui: viável; `docs/research/egui-viabilidade.md`.
- [x] Agentes: embutidos no Registro, cadastro derivado (ADR-0013).
- [x] Repositório: repo novo (este). v1 arquivada só após o `.msi`.
- [x] Rascunho/Finalizado: continua como regra do app; sem detecção de edição externa (nota na ADR-0003).
- [x] Pasta de dados: uma pasta ativa (padrão `Documentos\MyProvenance`, trocável), "Recarregar", `mtime` antes de gravar (#5).
- [x] Formato JSON v2: `schemaVersion: 4`, mesma linhagem da v1, leitura estrita (ADR-0014, #7).
- [x] Protótipo da Etapa 1 (#8): layout B (telas), gravação explícita com aviso; código só na tag `prototipo-etapa1`; dados reais da v1 passam no leitor v4; `.exe` de 9,65 MB.
- [x] Import do JSON da v1 (#9): só ação explícita "Importar da v1…"; recusa id existente (sem upsert), migração inválida e arquivo v4.
- [x] Etapa 1, crate real (#10): lista (tabela virtualizada + busca), tela do Registro, gravação explícita, conflito de `mtime`, Novo/Finalizar/Excluir, Configurações (pasta), tema. ABNT2 testado na descrição. `.exe` release de 9,19 MB.
- [x] Etapa 2, port da lógica da v1 (#12): `validar` recusa ciclo e violação de cardinalidade/Revisão; `export.rs` gera relatório Markdown e Mermaid (ainda sem botão: Etapa 3). Os 3 Registros reais da v1 continuam válidos.
- [x] Etapa 4, Importar da v1 (#11), código no `main` (commit `ae57278`): botão "Importar da v1…" na Lista (`rfd` múltiplo, resumo); `importar::migrar` = whitelist do zod da v1 + `htmd` + campos novos do v4 → `validar` → arquivo novo. Os 3 Registros reais (export v3 gerado da cópia da base) importam; ciclo, arquivo v4 e id existente são recusados; origem intacta. 27 testes ok. `.exe` release de 9,80 MiB (+`htmd`/`html5ever`).
- [ ] **Pendente da sessão de 2026-11-07 (fazer primeiro):** fechar no tracker o ticket [Etapa 4: Importar da v1](https://github.com/edalcin/MyProvenance-desktop/issues/11), que está atribuído e ainda aberto:
  1. Comentário de resolução: o que foi feito (linha acima) + recusas testadas + nota de que o leitor tolerante descarta todo campo fora do zod da v1 e remove Agentes sem Atividade (ADR-0014).
  2. `gh issue close 11`.
  3. No corpo do mapa (#1), em "Decisions so far", adicionar a linha `[Etapa 4: Importar da v1](…/issues/11): <gist>`. Nenhuma névoa nova; nada sai de "Not yet specified".
- [ ] Fronteira depois disso: Formulários de Atividade, Entidade e Agente (#13, protótipo HITL). Etapa 3 (grafo + export) ainda está na névoa do mapa.

**Dados de teste**: SQLite da v1 em `S:\appsdata\myprovenance\myprovenance.sqlite`, **só leitura**. Copiar o `.sqlite` e o `-wal` para fora do drive e ler a cópia. Para ter Registros reais na v2: gerar o export v3 da cópia (como o `gerarJsonExportado` da v1) e usar "Importar da v1…". Build no `S:`: usar `CARGO_TARGET_DIR=C:/Users/EDalcin/AppData/Local/cargo-target/myprovenance` (o `.exe` fica travado no drive mapeado). `cargo clippy` não está instalado.

**Smoke de UI**: accesskit expõe os widgets do egui ao UI Automation; `uv run --no-project --with pywinauto --with pillow python <script>` dirige o app e o diálogo do `rfd` (campo "Nome do arquivo" = `auto_id 1148`). Debug build: `Application.start(exe, wait_for_idle=False)`. O `set_focus` pode travar se outra janela tem o foco. Antes do smoke, apontar `pastaDados` em `%APPDATA%\MyProvenance\config.json` para uma pasta em OMPtemp e restaurar o config depois.

**Como continuar:** `/wayfinder https://github.com/edalcin/MyProvenance-desktop/issues/1`. Um ticket por sessão.

## Ao publicar o primeiro `.msi`

- [ ] Arquivar `edalcin/MyProvenance` (Settings → Archive). Antes disso, a v1 recebe só correções de segurança.

## Referência v1

Código e testes da v1 (spec do port): https://github.com/edalcin/MyProvenance/tree/main/src/lib
