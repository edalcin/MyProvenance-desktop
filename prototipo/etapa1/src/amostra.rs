//! Dados de exemplo para a pasta PROTÓTIPO. Cobre: válido rico, Finalizado, inválido, v1 e id duplicado.
use crate::store;
use serde_json::{Value, json};
use std::path::Path;

const AVES: &str = "0199a8b0-1c2d-7e3f-8a4b-5c6d7e8f9a01";
const RESTINGA: &str = "0199a8b0-1c2d-7e3f-8a4b-5c6d7e8f9b02";

fn aves() -> Value {
    json!({
      "schemaVersion": 4,
      "registro": {
        "id": AVES,
        "titulo": "Levantamento de aves — Mata Atlântica 2025",
        "descricao": "## Objetivo\n\nDocumentar a **origem** e o _tratamento_ das planilhas de campo do levantamento de aves.\n\n- Campanhas: março e setembro\n- Método: pontos de escuta (10 min)\n- Ver [protocolo](https://example.org/protocolo)\n\n| Campanha | Pontos | Registros |\n|---|---|---|\n| Março | 40 | 1.212 |\n| Setembro | 38 | 1.087 |\n\n> Ação: revisar a taxonomia com o CBRO 2024.",
        "status": "rascunho",
        "direcaoDiagrama": "TD",
        "criadoEm": "2025-03-02T12:00:00.000Z",
        "atualizadoEm": "2025-10-01T18:30:00.000Z",
        "finalizadoEm": null
      },
      "agentes": [
        {"id": "0199a8b0-0000-7000-8000-00000000a001", "nome": "Ana Souza", "tipo": "pessoa", "afiliacao": "JBRJ", "identificadorExterno": "https://orcid.org/0000-0002-1825-0097"},
        {"id": "0199a8b0-0000-7000-8000-00000000a002", "nome": "R 4.4", "tipo": "software", "afiliacao": null, "identificadorExterno": null}
      ],
      "entidades": [
        {"id": "0199a8b0-0000-7000-8000-00000000e001", "nome": "Planilha de campo (bruta)", "descricao": "Fichas digitadas", "formato": "XLSX", "localizacao": "Drive/aves/campo.xlsx", "licenca": "CC BY 4.0", "geradaPorAtividadeId": "0199a8b0-0000-7000-8000-00000000c001", "tipoRelacaoOrigem": null, "revisaoDeId": null},
        {"id": "0199a8b0-0000-7000-8000-00000000e002", "nome": "Planilha limpa", "descricao": null, "formato": "CSV", "localizacao": "Drive/aves/limpa.csv", "licenca": "CC BY 4.0", "geradaPorAtividadeId": "0199a8b0-0000-7000-8000-00000000c002", "tipoRelacaoOrigem": "revisao", "revisaoDeId": "0199a8b0-0000-7000-8000-00000000e001"},
        {"id": "0199a8b0-0000-7000-8000-00000000e003", "nome": "Relatório de riqueza", "descricao": "Curvas de rarefação", "formato": "PDF", "localizacao": null, "licenca": null, "geradaPorAtividadeId": "0199a8b0-0000-7000-8000-00000000c003", "tipoRelacaoOrigem": "derivacao", "revisaoDeId": null}
      ],
      "atividades": [
        {"id": "0199a8b0-0000-7000-8000-00000000c001", "tipo": "criacao", "agenteId": "0199a8b0-0000-7000-8000-00000000a001", "dataHora": "2025-03-10T09:00:00-03:00", "descricao": "Coleta em campo", "entidadesUsadas": [], "local": "PARNA Tijuca", "instrumento": "Gravador Zoom H5", "processo": null, "parametros": null, "ambienteExecucao": null},
        {"id": "0199a8b0-0000-7000-8000-00000000c002", "tipo": "transformacao", "agenteId": "0199a8b0-0000-7000-8000-00000000a002", "dataHora": "2025-04-02T14:00:00-03:00", "descricao": "Limpeza e padronização de nomes", "entidadesUsadas": ["0199a8b0-0000-7000-8000-00000000e001"], "local": null, "instrumento": null, "processo": "script limpeza.R", "parametros": [{"chave": "taxonomia", "valor": "CBRO 2024"}], "ambienteExecucao": {"sistemaOperacional": "Windows 11", "pacotes": [{"nome": "dplyr", "versao": "1.1.4"}]}},
        {"id": "0199a8b0-0000-7000-8000-00000000c003", "tipo": "analise", "agenteId": "0199a8b0-0000-7000-8000-00000000a002", "dataHora": "2025-05-20T10:00:00-03:00", "descricao": "Riqueza e rarefação", "entidadesUsadas": ["0199a8b0-0000-7000-8000-00000000e002"], "local": null, "instrumento": null, "processo": "iNEXT", "parametros": null, "ambienteExecucao": null}
      ]
    })
}

fn restinga(atualizado: &str) -> Value {
    json!({
      "schemaVersion": 4,
      "registro": {
        "id": RESTINGA,
        "titulo": "Inventário florístico — Restinga de Maricá",
        "descricao": "Registro **finalizado**. Só aceita novas Entidades/Atividades.",
        "status": "finalizado",
        "direcaoDiagrama": "LR",
        "criadoEm": "2024-08-01T12:00:00.000Z",
        "atualizadoEm": atualizado,
        "finalizadoEm": "2024-09-15T12:00:00.000Z"
      },
      "agentes": [],
      "entidades": [],
      "atividades": []
    })
}

pub fn semear(pasta: &Path) {
    let gravar = |nome: &str, v: &Value| store::escrever(&pasta.join(nome), &(serde_json::to_string_pretty(v).unwrap() + "\n"), false).unwrap();
    gravar("levantamento-de-aves-mata-atlantica-2025-7e8f9a01.json", &aves());
    gravar("inventario-floristico-restinga-de-marica-7e8f9b02.json", &restinga("2024-09-15T12:00:00.000Z"));
    // Cópia antiga do mesmo registro.id (Dropbox/cópia manual) → aviso, perde por atualizadoEm.
    gravar("inventario-floristico-restinga-de-marica-7e8f9b02 (cópia).json", &restinga("2024-08-20T12:00:00.000Z"));
    // Campo desconhecido → inválido (deny_unknown_fields).
    let mut v = aves();
    v["registro"]["id"] = json!("0199a8b0-1c2d-7e3f-8a4b-5c6d7e8f9c03");
    v["registro"]["titulo"] = json!("Registro com typo");
    v["registro"]["cor"] = json!("azul");
    gravar("registro-com-typo-7e8f9c03.json", &v);
    // Arquivo da v1 (schemaVersion 3) → aviso até a decisão do import (#9).
    let mut v = aves();
    v["schemaVersion"] = json!(3);
    v["registro"]["id"] = json!("0199a8b0-1c2d-7e3f-8a4b-5c6d7e8f9d04");
    v["registro"]["titulo"] = json!("Exportado da v1");
    gravar("exportado-da-v1.json", &v);
    // Lixo de escrita interrompida → ignorado em silêncio.
    std::fs::write(pasta.join("x.json.tmp"), "{").unwrap();
}
