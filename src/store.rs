//! Pasta de dados = 1 `.json` por Registro (decisão #5, ADR-0014). O app nunca altera arquivo inválido.
use crate::model::{self, Documento, Invalido};
use serde::Deserialize;
use std::{
    fmt, fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct Arquivo {
    pub caminho: PathBuf,
    /// `mtime` lido do disco; comparado antes de gravar.
    pub mtime: SystemTime,
    pub doc: Documento,
}

#[derive(Debug)]
pub enum ErroLeitura {
    Io(io::Error),
    Json(serde_json::Error),
    SemSchemaVersion,
    /// Arquivo da v1 (1–3): entra só pela ação "Importar da v1…" (decisão #9).
    V1(u64),
    SchemaDesconhecido(u64),
    Invalido(Invalido),
}

impl fmt::Display for ErroLeitura {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "JSON inválido: {e}"),
            Self::SemSchemaVersion => f.write_str("sem schemaVersion"),
            Self::V1(v) => write!(f, "arquivo da v1 (schemaVersion {v}); não é lido na pasta, só importado"),
            Self::SchemaDesconhecido(v) => write!(f, "schemaVersion {v} desconhecido"),
            Self::Invalido(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Debug)]
pub enum ErroGravar {
    /// Outro programa alterou o arquivo depois da leitura.
    Alterado,
    /// O arquivo sumiu da pasta depois da leitura.
    Removido,
    Invalido(Invalido),
    Io(io::Error),
}

impl fmt::Display for ErroGravar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Alterado => f.write_str("o arquivo mudou fora do app"),
            Self::Removido => f.write_str("o arquivo foi removido fora do app"),
            Self::Invalido(e) => write!(f, "registro inválido: {e}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Default)]
pub struct Carga {
    pub arquivos: Vec<Arquivo>,
    /// "arquivo: motivo" de cada arquivo ignorado.
    pub avisos: Vec<String>,
}

/// `<slug-do-título>-<últimos 8 hex do id>.json`, fixo na criação.
pub fn nome_arquivo(doc: &Documento) -> String {
    let hex = doc.registro.id.simple().to_string();
    format!("{}-{}.json", model::slug(&doc.registro.titulo), &hex[hex.len() - 8..])
}

pub fn nome(caminho: &Path) -> String {
    caminho.file_name().map_or_else(|| caminho.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// Lê só o nível de cima da pasta. Inválidos viram aviso; `.tmp` e outras extensões são ignorados em silêncio.
pub fn carregar(pasta: &Path) -> Carga {
    let mut carga = Carga::default();
    let mut entradas: Vec<PathBuf> = match fs::read_dir(pasta) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(e) => {
            carga.avisos.push(format!("{}: {e}", pasta.display()));
            return carga;
        }
    };
    entradas.sort();
    for caminho in entradas {
        if !caminho.is_file() || caminho.extension().is_none_or(|x| !x.eq_ignore_ascii_case("json")) {
            continue;
        }
        let a = match ler(&caminho) {
            Ok(a) => a,
            Err(e) => {
                carga.avisos.push(format!("{}: {e}", nome(&caminho)));
                continue;
            }
        };
        // Mesmo registro.id em dois arquivos: vence o maior atualizadoEm; empate = ordem do nome (já ordenado).
        match carga.arquivos.iter_mut().find(|x| x.doc.registro.id == a.doc.registro.id) {
            None => carga.arquivos.push(a),
            Some(x) => {
                let perdedor = if atualizado(&a) > atualizado(x) { std::mem::replace(x, a) } else { a };
                carga.avisos.push(format!("{}: mesmo registro.id de {}, que é mais novo", nome(&perdedor.caminho), nome(&x.caminho)));
            }
        }
    }
    carga
}

pub fn atualizado(a: &Arquivo) -> jiff::Timestamp {
    // validar() já garantiu ISO 8601 válido.
    a.doc.registro.atualizado_em.parse().unwrap_or(jiff::Timestamp::MIN)
}

pub fn ler(caminho: &Path) -> Result<Arquivo, ErroLeitura> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Versao {
        schema_version: Option<u64>,
    }
    let mtime = fs::metadata(caminho).and_then(|m| m.modified()).map_err(ErroLeitura::Io)?;
    let bytes = fs::read(caminho).map_err(ErroLeitura::Io)?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    match serde_json::from_slice::<Versao>(bytes).map_err(ErroLeitura::Json)?.schema_version {
        Some(4) => {}
        Some(v @ 1..=3) => return Err(ErroLeitura::V1(v)),
        Some(v) => return Err(ErroLeitura::SchemaDesconhecido(v)),
        None => return Err(ErroLeitura::SemSchemaVersion),
    }
    let doc: Documento = serde_json::from_slice(bytes).map_err(ErroLeitura::Json)?;
    model::validar(&doc).map_err(ErroLeitura::Invalido)?;
    Ok(Arquivo { caminho: caminho.to_owned(), mtime, doc })
}

/// Pretty-print 2 espaços, UTF-8 sem BOM, LF, newline no fim (ADR-0014).
pub fn serializar(doc: &Documento) -> String {
    let mut s = serde_json::to_string_pretty(doc).expect("Documento sempre serializa");
    s.push('\n');
    s
}

/// Grava `editado` por cima de `a`. Atualiza `atualizadoEm`, descarta Agentes sem uso (ADR-0014) e valida
/// antes de escrever. `forcar` pula a checagem de `mtime` (Sobrescrever / Gravar de novo).
pub fn gravar(a: &mut Arquivo, editado: &Documento, forcar: bool) -> Result<(), ErroGravar> {
    let mut doc = editado.clone();
    doc.registro.atualizado_em = model::agora();
    doc.agentes.retain(|g| doc.atividades.iter().any(|at| at.agente_id == g.id));
    model::validar(&doc).map_err(ErroGravar::Invalido)?;
    if !forcar {
        match fs::metadata(&a.caminho).and_then(|m| m.modified()) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(ErroGravar::Removido),
            Err(e) => return Err(ErroGravar::Io(e)),
            Ok(m) if m != a.mtime => return Err(ErroGravar::Alterado),
            Ok(_) => {}
        }
    }
    a.mtime = escrever(&a.caminho, &serializar(&doc), false).map_err(ErroGravar::Io)?;
    a.doc = doc;
    Ok(())
}

/// Cria o arquivo de um Registro novo; nunca sobrescreve um arquivo existente.
pub fn criar(pasta: &Path, doc: Documento) -> Result<Arquivo, ErroGravar> {
    model::validar(&doc).map_err(ErroGravar::Invalido)?;
    let caminho = pasta.join(nome_arquivo(&doc));
    let mtime = escrever(&caminho, &serializar(&doc), true).map_err(ErroGravar::Io)?;
    Ok(Arquivo { caminho, mtime, doc })
}

/// Escrita atômica: `.tmp` na mesma pasta → rename. Retorna o novo `mtime`.
pub fn escrever(caminho: &Path, conteudo: &str, novo: bool) -> io::Result<SystemTime> {
    let pasta = caminho.parent().ok_or_else(|| io::Error::other("caminho sem pasta"))?;
    let mut tmp = tempfile::Builder::new().suffix(".tmp").tempfile_in(pasta)?;
    tmp.write_all(conteudo.as_bytes())?;
    tmp.as_file().sync_all()?;
    if novo {
        tmp.persist_noclobber(caminho).map_err(|e| e.error)?;
    } else {
        tmp.persist(caminho).map_err(|e| e.error)?;
    }
    fs::metadata(caminho)?.modified()
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::{Value, json};

    fn gravar_json(pasta: &Path, nome: &str, v: &Value) {
        escrever(&pasta.join(nome), &(serde_json::to_string_pretty(v).unwrap() + "\n"), false).unwrap();
    }

    #[test]
    fn carregar_ignora_invalidos_e_resolve_id_duplicado() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        gravar_json(p, "aves.json", &model::exemplo());
        // Cópia antiga do mesmo registro.id (Dropbox/cópia manual), com nome que vem antes na ordem.
        let mut antiga = model::exemplo();
        antiga["registro"]["atualizadoEm"] = json!("2025-04-01T00:00:00Z");
        antiga["registro"]["titulo"] = json!("cópia antiga");
        gravar_json(p, "aves (antiga).json", &antiga);
        let mut typo = model::exemplo();
        typo["registro"]["id"] = json!("0199a8b0-1c2d-7e3f-8a4b-5c6d7e8f9c03");
        typo["registro"]["cor"] = json!("azul");
        gravar_json(p, "typo.json", &typo);
        let mut v1 = model::exemplo();
        v1["schemaVersion"] = json!(3);
        gravar_json(p, "v1.json", &v1);
        fs::write(p.join("x.json.tmp"), "{").unwrap();
        fs::write(p.join("com-bom.JSON"), [b"\xEF\xBB\xBF".as_slice(), b"{\"schemaVersion\": 4}"].concat()).unwrap();

        let c = carregar(p);
        assert_eq!(c.arquivos.len(), 1, "{:#?}", c.avisos);
        assert_eq!(c.arquivos[0].doc.registro.titulo, "Levantamento de aves — Mata Atlântica 2025");
        let avisos = c.avisos.join("\n");
        assert_eq!(c.avisos.len(), 4, "{avisos}");
        assert!(avisos.contains("aves (antiga).json: mesmo registro.id de aves.json"), "{avisos}");
        assert!(avisos.contains("typo.json: JSON inválido: unknown field `cor`"), "{avisos}");
        assert!(avisos.contains("v1.json: arquivo da v1 (schemaVersion 3)"), "{avisos}");
        // BOM aceito: o erro é do conteúdo, não do BOM.
        assert!(avisos.contains("com-bom.JSON: JSON inválido: missing field `registro`"), "{avisos}");
    }

    #[test]
    fn gravar_checa_mtime_e_reescreve_estavel() {
        let dir = tempfile::tempdir().unwrap();
        gravar_json(dir.path(), "aves.json", &model::exemplo());
        let mut a = ler(&dir.path().join("aves.json")).unwrap();
        let mut doc = a.doc.clone();
        doc.registro.titulo = "Novo título".into();

        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&a.caminho, "{}").unwrap();
        assert!(matches!(gravar(&mut a, &doc, false), Err(ErroGravar::Alterado)));
        gravar(&mut a, &doc, true).unwrap();
        gravar(&mut a, &doc, false).unwrap();
        assert_ne!(a.doc.registro.atualizado_em, "2025-10-01T18:30:00.000Z");

        let texto = fs::read_to_string(&a.caminho).unwrap();
        assert!(texto.ends_with("}\n") && !texto.contains('\r') && texto.starts_with("{\n  \"schemaVersion\": 4,"));
        assert_eq!(serializar(&ler(&a.caminho).unwrap().doc), texto, "ida e volta muda o arquivo");
        // Nome do arquivo fixo: título novo não renomeia.
        assert!(dir.path().join("aves.json").exists());

        fs::remove_file(&a.caminho).unwrap();
        assert!(matches!(gravar(&mut a, &doc, false), Err(ErroGravar::Removido)));
    }

    #[test]
    fn gravar_descarta_agente_sem_uso_e_recusa_invalido() {
        let dir = tempfile::tempdir().unwrap();
        gravar_json(dir.path(), "aves.json", &model::exemplo());
        let mut a = ler(&dir.path().join("aves.json")).unwrap();
        let mut doc = a.doc.clone();
        doc.atividades.pop();
        doc.entidades.pop();
        gravar(&mut a, &doc, false).unwrap();
        assert_eq!(a.doc.agentes.len(), 1, "agente 'R 4.4' ficou sem Atividade");

        let antes = fs::read_to_string(&a.caminho).unwrap();
        doc.registro.titulo = " ".into();
        assert!(matches!(gravar(&mut a, &doc, false), Err(ErroGravar::Invalido(_))));
        assert_eq!(fs::read_to_string(&a.caminho).unwrap(), antes);
    }

    #[test]
    fn criar_usa_slug_e_nao_sobrescreve() {
        let dir = tempfile::tempdir().unwrap();
        let doc = model::novo_documento("Ação nº 1 — Teste");
        let a = criar(dir.path(), doc.clone()).unwrap();
        let n = nome(&a.caminho);
        assert!(n.starts_with("acao-n-1-teste-") && n.len() == "acao-n-1-teste-".len() + 13, "{n}");
        assert!(n.contains(&doc.registro.id.simple().to_string()[24..]), "{n}");
        assert!(matches!(criar(dir.path(), doc), Err(ErroGravar::Io(_))));
        assert!(ler(&a.caminho).is_ok());
    }
}
