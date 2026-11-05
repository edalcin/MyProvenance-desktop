# MyProvenance

Ferramenta web para documentar a proveniência (origem, transformação e análise) de conjuntos de dados de pesquisa, sem armazenar os dados em si — apenas os registros descritivos dos eventos, seguindo o modelo W3C PROV (Entidades, Atividades, Agentes).

## Language

**Entidade**:
Registro de metadados que descreve um artefato/versão de um conjunto de dados (ex.: nome, descrição, formato, localização). Nunca contém o dado de pesquisa em si — apenas informações sobre ele, inseridas manualmente pelo usuário. Cada Atividade de Criação ou Transformação gera uma nova Entidade (nova versão).
_Avoid_: Dataset (usar só para o dado real, fora do sistema), Arquivo

**Atividade**:
Registro manual, preenchido pelo usuário via formulário, de um processo que usa e/ou gera Entidades. Três tipos — Criação, Transformação, Análise — cada um com regra própria de cardinalidade de entrada/saída (ver abaixo).
_Avoid_: Evento

**Criação**:
Atividade que gera 1 ou mais Entidades novas; pode opcionalmente usar Entidades de entrada (0+). Sem entradas, é a origem de uma linhagem.

**Transformação**:
Atividade que usa 1 ou mais Entidades existentes e gera 1 ou mais Entidades novas.

**Análise**:
Atividade que usa 1 ou mais Entidades existentes e gera 0 ou mais Entidades novas. A saída (ex.: relatório, figura, resultado estatístico) é opcional e pode ser mais de um artefato.

**Agente**:
Registro cadastrado e reutilizável (por conta, quando há conta — ver abaixo) que identifica quem ou o que é responsável por uma Atividade: Pessoa, Instituição ou Software. Selecionado via autocomplete a partir de um cadastro, nunca redigitado — evita duplicidade e inconsistência de nomes entre Atividades.
_Avoid_: Autor

**Registro de Proveniência**:
Container de nível mais alto: um grafo de Entidades e Atividades, com título (obrigatório) e descrição (opcional) próprios. Anônimo (sem conta): vive só na memória do navegador, exportado/importado como JSON. Com conta: persistido no SQLite, escopado à conta. Sempre exportável como JSON e renderizado como diagrama Mermaid + relatório. Pode ter mais de uma Entidade-raiz (Criação) quando Transformações combinam dados de origens diferentes.
_Avoid_: Projeto (reservado ao projeto de pesquisa externo, maior que a ferramenta), Sessão

**Conta**:
Cadastro opcional de acesso (username + PIN numérico de 6 dígitos) que faz os Registros e Agentes de um usuário persistirem automaticamente entre visitas. Agentes ficam sempre isolados por Conta. Registros também são isolados por padrão, mas podem ser compartilhados para edição com outras Contas — ver **Papel de Acesso**. Sem conta (modo padrão, "anônimo"), nada é salvo no servidor — o usuário exporta/importa JSON manualmente para continuar depois.
_Avoid_: Usuário (usar "Conta" para o cadastro de acesso; "Agente" continua sendo quem realizou a Atividade, mesmo com Conta ativa), Login

**Papel de Acesso**:
Nível de permissão de uma Conta sobre um Registro de Proveniência compartilhado, hierarquia Dono > Administrador > Editor (docs/especificacao.md §2.6). Dono é sempre quem criou o Registro. Administrador tem paridade total com o Dono, exceto ser removido. Editor edita título/descrição e Atividades/Entidades, mas não finaliza, exclui, nem gerencia compartilhamento. Distinto do link público de leitura (compartilhamento sem Conta, sem edição).
_Avoid_: Permissão, Role, Nível de acesso

**Rascunho / Finalizado**:
Os dois estados de um Registro de Proveniência. Rascunho: edição livre de Entidades e Atividades. Finalizado: atingido só por ação explícita do usuário (botão "Finalizar"); a partir daí o histórico existente é imutável, só é possível adicionar novas Entidades/Atividades. Exportar o JSON não finaliza (ADR-0010) — serve de backup/arquivo portátil, inclusive para quem usa sem Conta.
_Avoid_: Publicado, Bloqueado, Travado

**Área administrativa**:
Painel global (`/admin`, docs/especificacao.md §9), protegido por uma senha única de instância (`ADM_PWD`) — não é o mesmo conceito que o papel **Administrador** de um Registro (esse é por Conta, escopado a um único Registro compartilhado; a Área administrativa enxerga/edita todas as Contas, Registros, Agentes e compartilhamentos do servidor, mais backup/restauração do SQLite inteiro).
_Avoid_: Admin (ambíguo com o papel de acesso — sempre escrever "Área administrativa" por extenso na documentação)
