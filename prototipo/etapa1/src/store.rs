//! Pasta de dados = 1 `.json` por Registro (decisões #5 e ADR-0014).
use crate::model::{self, Documento};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct Arquivo {
    pub caminho: PathBuf,
    /// `mtime` lido do disco; comparado antes de gravar.
    pub mtime: SystemTime,
    pub doc: Documento,
    /// Edição em memória ainda não gravada.
    pub sujo: bool,
}

pub enum Conflito {
    Alterado,
    Removido,
}

pub fn nome_arquivo(doc: &Documento) -> String {
    let hex: String = doc.registro.id.chars().filter(|c| *c != '-').collect();
    format!("{}-{}.json", model::slug(&doc.registro.titulo), &hex[hex.len() - 8..])
}

/// Lê só o nível de cima. Inválidos e `schemaVersion` ≠ 4 viram aviso; o arquivo nunca é alterado.
pub fn carregar(pasta: &Path) -> (Vec<Arquivo>, Vec<String>) {
    let mut avisos = Vec::new();
    let mut lidos: Vec<Arquivo> = Vec::new();
    let mut entradas: Vec<PathBuf> = match fs::read_dir(pasta) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(e) => return (lidos, vec![format!("{}: {e}", pasta.display())]),
    };
    entradas.sort();
    for caminho in entradas {
        if !caminho.is_file() || caminho.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let nome = caminho.file_name().unwrap().to_string_lossy().into_owned();
        match ler(&caminho) {
            Ok(a) => lidos.push(a),
            Err(e) => avisos.push(format!("{nome}: {e}")),
        }
    }
    // id duplicado: vence maior atualizadoEm; empate = ordem do nome (já ordenado).
    let mut arquivos: Vec<Arquivo> = Vec::new();
    for a in lidos {
        match arquivos.iter_mut().find(|x| x.doc.registro.id == a.doc.registro.id) {
            None => arquivos.push(a),
            Some(x) => {
                let perde = if ts(&a) > ts(x) { std::mem::replace(x, a) } else { a };
                avisos.push(format!(
                    "{}: mesmo registro.id de {} (mais novo) — ignorado",
                    perde.caminho.file_name().unwrap().to_string_lossy(),
                    x.caminho.file_name().unwrap().to_string_lossy()
                ));
            }
        }
    }
    (arquivos, avisos)
}

fn ts(a: &Arquivo) -> jiff::Timestamp {
    a.doc.registro.atualizado_em.parse().unwrap_or(jiff::Timestamp::MIN)
}

pub fn ler(caminho: &Path) -> Result<Arquivo, String> {
    let mtime = fs::metadata(caminho).and_then(|m| m.modified()).map_err(|e| e.to_string())?;
    let bytes = fs::read(caminho).map_err(|e| e.to_string())?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    let valor: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| format!("JSON inválido: {e}"))?;
    match valor.get("schemaVersion").and_then(|v| v.as_u64()) {
        Some(4) => {}
        Some(v @ 1..=3) => return Err(format!("schemaVersion {v} (v1) — import da v1 ainda não decidido (#9)")),
        Some(v) => return Err(format!("schemaVersion {v} desconhecido")),
        None => return Err("sem schemaVersion".into()),
    }
    let doc: Documento = serde_json::from_value(valor).map_err(|e| e.to_string())?;
    model::validar(&doc)?;
    Ok(Arquivo { caminho: caminho.to_owned(), mtime, doc, sujo: false })
}

pub fn serializar(doc: &Documento) -> String {
    let mut s = serde_json::to_string_pretty(doc).expect("serialize");
    s.push('\n');
    s
}

/// Escrita atômica (`.tmp` na mesma pasta → rename). `forcar` pula a checagem de `mtime`.
pub fn gravar(a: &mut Arquivo, forcar: bool) -> Result<(), Conflito> {
    if !forcar {
        match fs::metadata(&a.caminho).and_then(|m| m.modified()) {
            Err(_) => return Err(Conflito::Removido),
            Ok(m) if m != a.mtime => return Err(Conflito::Alterado),
            Ok(_) => {}
        }
    }
    a.doc.registro.atualizado_em = model::agora();
    escrever(&a.caminho, &serializar(&a.doc), false).expect("gravar");
    a.mtime = fs::metadata(&a.caminho).and_then(|m| m.modified()).expect("mtime");
    a.sujo = false;
    Ok(())
}

pub fn escrever(caminho: &Path, conteudo: &str, novo: bool) -> std::io::Result<()> {
    let mut tmp = tempfile::Builder::new().suffix(".tmp").tempfile_in(caminho.parent().unwrap())?;
    tmp.write_all(conteudo.as_bytes())?;
    tmp.as_file().sync_all()?;
    if novo {
        tmp.persist_noclobber(caminho)?;
    } else {
        tmp.persist(caminho)?;
    }
    Ok(())
}

pub fn criar(pasta: &Path, doc: Documento) -> Arquivo {
    let caminho = pasta.join(nome_arquivo(&doc));
    escrever(&caminho, &serializar(&doc), true).expect("criar");
    let mtime = fs::metadata(&caminho).and_then(|m| m.modified()).expect("mtime");
    Arquivo { caminho, mtime, doc, sujo: false }
}

pub fn novo_documento(titulo: &str) -> Documento {
    let agora = model::agora();
    Documento {
        schema_version: model::SCHEMA_VERSION,
        registro: model::Registro {
            id: uuid::Uuid::now_v7().to_string(),
            titulo: titulo.into(),
            descricao: None,
            status: model::Status::Rascunho,
            direcao_diagrama: model::Direcao::TD,
            criado_em: agora.clone(),
            atualizado_em: agora,
            finalizado_em: None,
        },
        agentes: vec![],
        entidades: vec![],
        atividades: vec![],
    }
}

#[cfg(test)]
#[test]
fn pasta_de_amostra() {
    let dir = tempfile::tempdir().unwrap();
    crate::amostra::semear(dir.path());
    let (mut arqs, avisos) = carregar(dir.path());
    // aves + restinga válidos; cópia antiga, typo e v1 viram aviso; .tmp ignorado em silêncio.
    assert_eq!(arqs.len(), 2, "{avisos:#?}");
    assert_eq!(avisos.len(), 3, "{avisos:#?}");
    assert!(avisos.iter().any(|a| a.contains("(cópia)") && a.contains("mesmo registro.id")));
    assert!(avisos.iter().any(|a| a.contains("unknown field `cor`")));
    assert!(avisos.iter().any(|a| a.contains("schemaVersion 3")));

    // Ida e volta estável.
    let s = serializar(&arqs[0].doc);
    assert_eq!(serializar(&serde_json::from_str(&s).unwrap()), s);
    assert!(s.ends_with("}\n") && !s.contains('\r'));

    // mtime: alterado fora do app → conflito; forçar grava; removido → conflito.
    std::thread::sleep(std::time::Duration::from_millis(20));
    fs::write(&arqs[0].caminho, s.as_bytes()).unwrap();
    assert!(matches!(gravar(&mut arqs[0], false), Err(Conflito::Alterado)));
    assert!(gravar(&mut arqs[0], true).is_ok());
    assert!(gravar(&mut arqs[0], false).is_ok());
    fs::remove_file(&arqs[0].caminho).unwrap();
    assert!(matches!(gravar(&mut arqs[0], false), Err(Conflito::Removido)));

    // Nome fixo na criação: slug do título + últimos 8 hex do id.
    let novo = criar(dir.path(), novo_documento("Ação nº 1 — Teste"));
    let nome = novo.caminho.file_name().unwrap().to_string_lossy().into_owned();
    assert!(nome.starts_with("acao-n-1-teste-") && nome.ends_with(".json") && nome.len() == "acao-n-1-teste-".len() + 13, "{nome}");
    assert!(ler(&novo.caminho).is_ok());
}
