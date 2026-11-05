# Próximos Passos

## v2 desktop (Rust/egui, só Windows) — em andamento

Plano: `docs/versao2.md`. Mapa canônico: [Wayfinder: MyProvenance v2 desktop](https://github.com/edalcin/MyProvenance-desktop/issues/1) (inclui execução até o `.msi` no Release). Estado em 2026-11-05:

- [x] Mapa criado e migrado de `edalcin/MyProvenance` para este repo.
- [x] Viabilidade egui: viável; `docs/research/egui-viabilidade.md`.
- [x] Agentes: embutidos no Registro, cadastro derivado (ADR-0013).
- [x] Repositório: repo novo (este). v1 arquivada só após o `.msi`.
- [x] Rascunho/Finalizado: continua como regra do app; sem detecção de edição externa (nota na ADR-0003).
- [ ] Fronteira: Pasta de dados (#5), Formato JSON v2 (#7).
- [ ] Bloqueados: Protótipo Etapa 1 (#8) ← #7.

**Como continuar:** `/wayfinder https://github.com/edalcin/MyProvenance-desktop/issues/1`. Um ticket por sessão.

## Ao publicar o primeiro `.msi`

- [ ] Arquivar `edalcin/MyProvenance` (Settings → Archive). Antes disso, a v1 recebe só correções de segurança.

## Referência v1

Código e testes da v1 (spec do port): https://github.com/edalcin/MyProvenance/tree/main/src/lib
