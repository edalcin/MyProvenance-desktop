# v2: Agentes embutidos em cada Registro, cadastro derivado

Na v2 (desktop, 1 arquivo JSON por Registro de Proveniência, sem SQLite), cada `registro.json` leva cópias completas dos Agentes que usa, com `id` UUID estável — mesmo formato do `RegistroExportado` da v1. Não existe `agentes.json`: o cadastro usado no autocomplete é derivado em memória, lendo todos os Registros da pasta e agrupando por `id`. Um Agente sai do cadastro quando nenhum Registro o usa.

Editar um Agente presente em outros Registros pergunta "Atualizar todos?" (Sim / Só este); Registros Finalizados nunca são alterados. Cópias divergentes do mesmo `id` aparecem uma vez no autocomplete, na versão do Registro com maior `atualizadoEm`.

Motivo: manter "1 arquivo = 1 Registro completo" — compartilhar é enviar um arquivo, sync via Git/Dropbox não tem arquivo global conflitante, e o import da v1 é direto.

Alternativas rejeitadas: **`agentes.json` compartilhado** com `agenteId` nos Registros — edição propaga sozinha, mas um Registro enviado isolado chega sem Agentes e o arquivo global vira ponto de conflito de sync. **Cadastro + cópia embutida** — duas fontes de verdade, divergência garantida.

Decidido em [Agentes na v2: embutidos no registro ou agentes.json compartilhado?](https://github.com/edalcin/MyProvenance-desktop/issues/2).
