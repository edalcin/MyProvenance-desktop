# v2: formato JSON por Registro = `schemaVersion: 4`, mesma linhagem da v1

Na v2, cada Registro de Proveniência é um arquivo JSON que **é o store**, e não um export. O formato continua a linhagem do `RegistroExportado` da v1: mesmo campo `schemaVersion`, valor `4`, e mesmo envelope `{schemaVersion, registro, agentes, entidades, atividades}`. Os arquivos da v1 (`schemaVersion` 1–3) passam por uma migração 3→4, e não por um detector separado.

Mudanças em relação ao `schemaVersion: 3`:

- `registro` recebe `atualizadoEm` (obrigatório; o app grava em toda escrita; a ADR-0013 o usa para resolver divergência de Agentes) e `direcaoDiagrama` (`'LR'|'TD'`, padrão `'TD'`). Esse campo agora vai junto com o arquivo, porque não existe outro store por Registro.
- `registro.descricao` passa de HTML (TipTap) para **Markdown**.
- Saem `registroId` (entidades/atividades) e `atividade.entidadesGeradas`. O arquivo já liga os itens ao Registro, e o app deriva `entidadesGeradas` em memória a partir de `entidade.geradaPorAtividadeId`, que é a fonte única, como no SQLite da v1.
- Agentes que nenhuma Atividade usa: o app aceita na leitura e não grava no save.

Leitura estrita: um campo desconhecido no `schemaVersion: 4` torna o arquivo inválido, e o app o ignora com um aviso, sem alterar o arquivo. A mesma regra vale para estes erros: estrutura e limites (os mesmos do zod da v1), ids repetidos, referência quebrada, `status` incoerente com `finalizadoEm`, ciclo no grafo, ou violação de cardinalidade/Revisão. Fica uma definição só de "Registro válido". O leitor da v1 é tolerante (descarta `deOutraConta`/`donoUsername`).

Migração 3→4: `descricao` HTML→Markdown com a crate `htmd` (usada só aqui); `atualizadoEm = finalizadoEm ?? criadoEm`, para que uma cópia antiga importada nunca vença uma edição feita na v2; `direcaoDiagrama = 'TD'`.

Arquivo: `<slug-do-título>-<últimos 8 hex do id>.json`. O nome é fixo na criação e nunca muda quando o título muda. A identidade é sempre `registro.id`. Os 8 últimos hex são aleatórios; os primeiros são timestamp do UUIDv7 e se repetem dentro do mesmo minuto. Se dois arquivos têm o mesmo `registro.id`, o app carrega o arquivo com o maior `atualizadoEm` (empate: ordem do nome) e avisa sobre os outros, sem alterá-los.

Ids: o leitor aceita UUID de qualquer versão, porque a base real da v1 tem UUIDv4 (visto no [Protótipo da Etapa 1](https://github.com/edalcin/MyProvenance-desktop/issues/8)). A v2 gera só UUIDv7.

Serialização: pretty-print com 2 espaços, UTF-8 sem BOM, LF e newline no fim, `null` explícito, ordem dos campos = ordem do struct, arrays na ordem de inserção. O leitor aceita BOM.

Alternativas rejeitadas: **campo novo `versao: 1`**, que criaria duas linhagens e um segundo detector para a v1. **Manter `registroId`/`entidadesGeradas`**, que deixaria dados redundantes e regras de coerência a mais. **Ignorar ou preservar campos desconhecidos**, porque ignorar perde dados em silêncio e preservar esconde typos. **Renomear o arquivo quando o título muda**, porque Git/Dropbox veem o rename como delete + create. **Aplicar as regras de domínio só nos formulários**, porque o app ficaria com duas noções de "válido".

Decidido em [Formato JSON v2: diferenças em relação ao RegistroExportado](https://github.com/edalcin/MyProvenance-desktop/issues/7).
