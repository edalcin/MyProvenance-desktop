//! "Importar da v1…" (decisão #9): leitor tolerante v1 (`schemaVersion` 1–3) → migração 3→4 (ADR-0014) →
//! leitura estrita v4 → arquivo novo na pasta ativa. O arquivo de origem nunca é alterado.
use crate::{
    model::{self, Documento, Invalido},
    store::{self, Arquivo, ErroGravar},
};
use serde_json::{Map, Value, json};
use std::{fmt, fs, io, path::Path};

#[derive(Debug)]
pub enum Recusa {
    Io(io::Error),
    Json(serde_json::Error),
    SemSchemaVersion,
    /// Arquivo v4: Importar só traz da v1.
    JaV2,
    SchemaDesconhecido(u64),
    Invalido(Invalido),
    /// Sem upsert (ADR-0004 superada na v2): substituir = excluir na v2 e importar de novo.
    IdExistente,
    Gravar(ErroGravar),
}

impl fmt::Display for Recusa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "JSON inválido: {e}"),
            Self::SemSchemaVersion => f.write_str("sem schemaVersion; não é um JSON exportado da v1"),
            Self::JaV2 => f.write_str("arquivo já é da v2; copie para a pasta de dados"),
            Self::SchemaDesconhecido(v) => write!(f, "schemaVersion {v} desconhecido"),
            Self::Invalido(e) => write!(f, "inválido depois da migração: {e}"),
            Self::IdExistente => {
                f.write_str("o Registro já existe na pasta (mesmo registro.id); para substituir, exclua-o e importe de novo")
            }
            Self::Gravar(e) => write!(f, "{e}"),
        }
    }
}

// Campos do zod da v1 (`registroExportadoSchema`); o resto é descartado, como no `z.object` (strip).
const REGISTRO: &[&str] = &["id", "titulo", "descricao", "status", "criadoEm", "finalizadoEm"];
const AGENTE: &[&str] = &["id", "nome", "tipo", "afiliacao", "identificadorExterno"];
const ENTIDADE: &[&str] =
    &["id", "nome", "descricao", "formato", "localizacao", "licenca", "geradaPorAtividadeId", "tipoRelacaoOrigem", "revisaoDeId"];
const ATIVIDADE: &[&str] = &[
    "id", "tipo", "agenteId", "dataHora", "descricao", "entidadesUsadas", "local", "instrumento", "processo", "parametros",
    "ambienteExecucao",
];

fn manter<'a>(v: Option<&'a mut Value>, campos: &[&str]) -> Option<&'a mut Map<String, Value>> {
    let o = v?.as_object_mut()?;
    o.retain(|k, _| campos.contains(&k.as_str()));
    Some(o)
}

fn cada<'a>(v: Option<&'a mut Value>) -> impl Iterator<Item = &'a mut Value> {
    v.and_then(Value::as_array_mut).into_iter().flatten()
}

/// JSON exportado pela v1 → Documento v4 válido. Tipos errados ficam para o serde/`validar` recusarem.
pub fn migrar(bytes: &[u8]) -> Result<Documento, Recusa> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut v: Value = serde_json::from_slice(bytes).map_err(Recusa::Json)?;
    match v.get("schemaVersion").and_then(Value::as_u64) {
        Some(1..=3) => {}
        Some(4) => return Err(Recusa::JaV2),
        Some(n) => return Err(Recusa::SchemaDesconhecido(n)),
        None => return Err(Recusa::SemSchemaVersion),
    }
    let raiz = manter(Some(&mut v), &["schemaVersion", "registro", "agentes", "entidades", "atividades"]).expect("objeto");
    raiz.insert("schemaVersion".into(), json!(model::SCHEMA_VERSION));
    if let Some(r) = manter(raiz.get_mut("registro"), REGISTRO) {
        if let Some(html) = r.get("descricao").and_then(Value::as_str) {
            let md = htmd::convert(html).map_err(Recusa::Io)?;
            let md = md.trim();
            r.insert("descricao".into(), if md.is_empty() { Value::Null } else { json!(md) });
        }
        // Cópia antiga nunca vence uma edição feita na v2 (ADR-0013).
        let atualizado = r.get("finalizadoEm").filter(|x| !x.is_null()).or(r.get("criadoEm")).cloned();
        r.insert("atualizadoEm".into(), atualizado.unwrap_or(Value::Null));
        r.insert("direcaoDiagrama".into(), json!("TD"));
    }
    for g in cada(raiz.get_mut("agentes")) {
        manter(Some(g), AGENTE);
    }
    for e in cada(raiz.get_mut("entidades")) {
        manter(Some(e), ENTIDADE);
    }
    for a in cada(raiz.get_mut("atividades")) {
        let Some(a) = manter(Some(a), ATIVIDADE) else { continue };
        for p in cada(a.get_mut("parametros")) {
            manter(Some(p), &["chave", "valor"]);
        }
        if let Some(amb) = manter(a.get_mut("ambienteExecucao"), &["sistemaOperacional", "pacotes"]) {
            for p in cada(amb.get_mut("pacotes")) {
                manter(Some(p), &["nome", "versao"]);
            }
        }
    }
    let mut doc: Documento = serde_json::from_value(v).map_err(Recusa::Json)?;
    doc.agentes.retain(|g| doc.atividades.iter().any(|at| at.agente_id == g.id));
    model::validar(&doc).map_err(Recusa::Invalido)?;
    Ok(doc)
}

/// Migra `origem` e cria o arquivo `<slug>-<8 hex>.json` em `pasta`. Recusa `registro.id` já presente em `existentes`.
pub fn importar(pasta: &Path, origem: &Path, existentes: &[Arquivo]) -> Result<Arquivo, Recusa> {
    let doc = migrar(&fs::read(origem).map_err(Recusa::Io)?)?;
    if existentes.iter().any(|a| a.doc.registro.id == doc.registro.id) {
        return Err(Recusa::IdExistente);
    }
    store::criar(pasta, doc).map_err(Recusa::Gravar)
}

#[cfg(test)]
mod testes {
    use super::*;

    /// O exemplo como a v1 o exporta (`gerarJsonExportado`, schemaVersion 3).
    fn v1() -> Value {
        let mut v = model::exemplo();
        v["schemaVersion"] = json!(3);
        let r = v["registro"].as_object_mut().unwrap();
        r.remove("atualizadoEm");
        r.remove("direcaoDiagrama");
        r.insert("descricao".into(), json!("<h2>Objetivo</h2><p>Documentar a <strong>origem</strong> &amp; o uso.</p><p></p>"));
        let id = r["id"].clone();
        v["agentes"][0]["deOutraConta"] = json!(true);
        v["agentes"][0]["donoUsername"] = json!("ana");
        for e in v["entidades"].as_array_mut().unwrap() {
            e["registroId"] = id.clone();
        }
        for a in v["atividades"].as_array_mut().unwrap() {
            a["registroId"] = id.clone();
            a["entidadesGeradas"] = json!([]);
        }
        v
    }

    fn bytes(v: &Value) -> Vec<u8> {
        serde_json::to_vec(v).unwrap()
    }

    #[test]
    fn migra_v3_descartando_campos_da_v1() {
        let d = migrar(&bytes(&v1())).unwrap();
        assert_eq!(d.schema_version, 4);
        assert_eq!(d.registro.descricao.as_deref(), Some("## Objetivo\n\nDocumentar a **origem** & o uso."));
        assert_eq!(d.registro.atualizado_em, d.registro.criado_em);
        assert_eq!(d.registro.direcao_diagrama, model::Direcao::TD);
        // Ida e volta pelo leitor estrito: nenhum campo da v1 sobra.
        let texto = store::serializar(&d);
        assert!(!texto.contains("registroId") && !texto.contains("entidadesGeradas") && !texto.contains("deOutraConta"));
        let _: Documento = serde_json::from_str(&texto).unwrap();
    }

    #[test]
    fn finalizado_usa_finalizado_em_e_descricao_vazia_vira_null() {
        let mut v = v1();
        v["registro"]["status"] = json!("finalizado");
        v["registro"]["finalizadoEm"] = json!("2025-11-01T10:00:00.000Z");
        v["registro"]["descricao"] = json!("<p></p>");
        let d = migrar(&bytes(&v)).unwrap();
        assert_eq!(d.registro.atualizado_em, "2025-11-01T10:00:00.000Z");
        assert_eq!(d.registro.descricao, None);
    }

    #[test]
    fn recusas() {
        let recusa = |v: Value| migrar(&bytes(&v)).unwrap_err();
        assert!(matches!(recusa(model::exemplo()), Recusa::JaV2));
        assert!(matches!(recusa(json!({"registro": {}})), Recusa::SemSchemaVersion));
        assert!(matches!(recusa(json!({"schemaVersion": 9})), Recusa::SchemaDesconhecido(9)));
        let mut ciclo = v1();
        ciclo["atividades"][0]["entidadesUsadas"] = json!(["0199a8b0-0000-7000-8000-00000000e002"]);
        assert!(matches!(recusa(ciclo), Recusa::Invalido(_)));
        let mut sem_titulo = v1();
        sem_titulo["registro"].as_object_mut().unwrap().remove("titulo");
        assert!(matches!(recusa(sem_titulo), Recusa::Json(_)));
    }

    #[test]
    fn importar_cria_arquivo_novo_e_recusa_id_existente() {
        let dir = tempfile::tempdir().unwrap();
        let origem = dir.path().join("export-v1.json");
        fs::write(&origem, bytes(&v1())).unwrap();
        let pasta = dir.path().join("dados");
        fs::create_dir(&pasta).unwrap();

        let a = importar(&pasta, &origem, &[]).unwrap();
        assert!(store::nome(&a.caminho).starts_with("levantamento-de-aves-mata-atlantica-2025-"));
        assert_eq!(fs::read(&origem).unwrap(), bytes(&v1()), "origem alterada");
        assert!(store::ler(&a.caminho).is_ok());
        assert!(matches!(importar(&pasta, &origem, &[a]), Err(Recusa::IdExistente)));
    }
}
