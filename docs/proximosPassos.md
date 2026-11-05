# Próximos Passos

## v2 desktop (Rust/egui, só Windows) — em andamento

Plano: `docs/versao2.md`. Mapa canônico: [Wayfinder: MyProvenance v2 desktop](https://github.com/edalcin/MyProvenance-desktop/issues/1) (inclui execução até o `.msi` no Release). Estado em 2026-11-05:

- [x] Mapa criado e migrado de `edalcin/MyProvenance` para este repo.
- [x] Viabilidade egui: viável; `docs/research/egui-viabilidade.md`.
- [x] Agentes: embutidos no Registro, cadastro derivado (ADR-0013).
- [x] Repositório: repo novo (este). v1 arquivada só após o `.msi`.
- [x] Rascunho/Finalizado: continua como regra do app; sem detecção de edição externa (nota na ADR-0003).
- [x] Pasta de dados: uma pasta ativa (padrão `Documentos\MyProvenance`, trocável), "Recarregar", `mtime` antes de gravar (#5).
- [x] Formato JSON v2: `schemaVersion: 4`, mesma linhagem da v1, leitura estrita (ADR-0014, #7).
- [x] Protótipo da Etapa 1 (#8): layout B (telas), gravação explícita com aviso; código só na tag `prototipo-etapa1`; dados reais da v1 passam no leitor v4; `.exe` de 9,65 MB.
- [x] Import do JSON da v1 (#9): só ação explícita "Importar da v1…"; recusa id existente (sem upsert), migração inválida e arquivo v4.
- [ ] Fronteira: Etapa 1, crate real (#10). Depois: Etapa 4, Importar da v1 (#11, bloqueado por #10).

**Dados de teste**: SQLite da v1 em `S:\appsdata\myprovenance\myprovenance.sqlite`, **só leitura**. Copiar o `.sqlite` e o `-wal` para fora do drive e ler a cópia. Conversor descartável para v4: `prototipo/etapa1/v1_para_v4.py` na tag `prototipo-etapa1`.

**Como continuar:** `/wayfinder https://github.com/edalcin/MyProvenance-desktop/issues/1`. Um ticket por sessão.

## Ao publicar o primeiro `.msi`

- [ ] Arquivar `edalcin/MyProvenance` (Settings → Archive). Antes disso, a v1 recebe só correções de segurança.

## Referência v1

Código e testes da v1 (spec do port): https://github.com/edalcin/MyProvenance/tree/main/src/lib
