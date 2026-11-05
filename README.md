# MyProvenance desktop (v2)

App desktop stand-alone (Windows x64, Rust/egui) para Registros de Proveniência em JSON. Sucessor da v1 web: https://github.com/edalcin/MyProvenance

Status: Etapa 1 (lista + Registro, gravação explícita). Plano em `docs/versao2.md`, estado em `docs/proximosPassos.md`.

## Dados

- **Pasta de dados**: padrão `Documentos\MyProvenance`, trocável em Configurações (trocar não move arquivos). Um `.json` por Registro (`schemaVersion: 4`, ADR-0014). Só o nível de cima da pasta é lido; arquivos inválidos são ignorados com aviso e nunca alterados.
- **Configuração**: `%APPDATA%\MyProvenance\config.json` (`pastaDados`, `tema`).

## Build

Requer Rust estável (edition 2024) e o toolchain MSVC.

```powershell
cargo run                 # debug
cargo test
cargo build --release     # target\release\myprovenance.exe, CRT estático (.cargo/config.toml)
```

Repo em drive de rede/mapeado (ex.: `S:`): o `cargo` pode falhar com "Permission denied" ou `LNK1104` porque o `.exe` gerado fica travado. Use um `CARGO_TARGET_DIR` no `C:`.

## Licenças de terceiros

- Ícones: [Boxicons](https://boxicons.com) 2.1.4, MIT (`assets/boxicons/LICENSE`).
