//! Formato JSON v2 = `schemaVersion: 4` (ADR-0014). Ordem dos campos = ordem de serialização.
//! "Registro válido" tem uma definição só: `validar`.
use garde::Validate;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 4;

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Documento {
    #[garde(skip)]
    pub schema_version: u32,
    #[garde(dive)]
    pub registro: Registro,
    #[garde(dive)]
    pub agentes: Vec<Agente>,
    #[garde(dive)]
    pub entidades: Vec<Entidade>,
    #[garde(dive)]
    pub atividades: Vec<Atividade>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Rascunho,
    Finalizado,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direcao {
    LR,
    TD,
}

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Registro {
    #[garde(skip)]
    pub id: Uuid,
    #[garde(custom(texto(true, 300)))]
    pub titulo: String,
    /// Markdown (v1: HTML do TipTap).
    #[garde(custom(opcional(20000)))]
    pub descricao: Option<String>,
    #[garde(skip)]
    pub status: Status,
    #[garde(skip)]
    pub direcao_diagrama: Direcao,
    #[garde(custom(iso_8601))]
    pub criado_em: String,
    #[garde(custom(iso_8601))]
    pub atualizado_em: String,
    #[garde(custom(iso_8601_opcional))]
    pub finalizado_em: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TipoAgente {
    Pessoa,
    Instituicao,
    Software,
}

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Agente {
    #[garde(skip)]
    pub id: Uuid,
    #[garde(custom(texto(true, 200)))]
    pub nome: String,
    #[garde(skip)]
    pub tipo: TipoAgente,
    #[garde(custom(opcional(300)))]
    pub afiliacao: Option<String>,
    #[garde(custom(opcional(200)))]
    pub identificador_externo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TipoRelacaoOrigem {
    Derivacao,
    Revisao,
}

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entidade {
    #[garde(skip)]
    pub id: Uuid,
    #[garde(custom(texto(true, 300)))]
    pub nome: String,
    #[garde(custom(opcional(5000)))]
    pub descricao: Option<String>,
    #[garde(custom(opcional(100)))]
    pub formato: Option<String>,
    #[garde(custom(opcional(2000)))]
    pub localizacao: Option<String>,
    #[garde(custom(opcional(300)))]
    pub licenca: Option<String>,
    #[garde(skip)]
    pub gerada_por_atividade_id: Uuid,
    #[garde(skip)]
    pub tipo_relacao_origem: Option<TipoRelacaoOrigem>,
    #[garde(skip)]
    pub revisao_de_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TipoAtividade {
    Criacao,
    Transformacao,
    Analise,
}

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Parametro {
    #[garde(custom(texto(true, 200)))]
    pub chave: String,
    #[garde(custom(texto(false, 2000)))]
    pub valor: String,
}

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Pacote {
    #[garde(custom(texto(true, 200)))]
    pub nome: String,
    #[garde(custom(texto(false, 100)))]
    pub versao: String,
}

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AmbienteExecucao {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(custom(opcional(200)))]
    pub sistema_operacional: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[garde(dive)]
    pub pacotes: Option<Vec<Pacote>>,
}

#[derive(Serialize, Deserialize, Validate, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Atividade {
    #[garde(skip)]
    pub id: Uuid,
    #[garde(skip)]
    pub tipo: TipoAtividade,
    #[garde(skip)]
    pub agente_id: Uuid,
    #[garde(custom(iso_8601))]
    pub data_hora: String,
    #[garde(custom(opcional(5000)))]
    pub descricao: Option<String>,
    #[garde(skip)]
    pub entidades_usadas: Vec<Uuid>,
    #[garde(custom(opcional(500)))]
    pub local: Option<String>,
    #[garde(custom(opcional(300)))]
    pub instrumento: Option<String>,
    #[garde(custom(opcional(20000)))]
    pub processo: Option<String>,
    #[garde(dive)]
    pub parametros: Option<Vec<Parametro>>,
    #[garde(dive)]
    pub ambiente_execucao: Option<AmbienteExecucao>,
}

/// Limites iguais ao zod da v1 (`src/lib/schemas.ts`): `.trim()` e depois comprimento em UTF-16.
fn texto(obrigatorio: bool, max: usize) -> impl FnOnce(&String, &()) -> garde::Result {
    move |v, _| limite(v, obrigatorio, max)
}

fn opcional(max: usize) -> impl FnOnce(&Option<String>, &()) -> garde::Result {
    move |v, _| v.as_deref().map_or(Ok(()), |s| limite(s, false, max))
}

fn limite(s: &str, obrigatorio: bool, max: usize) -> garde::Result {
    let n = s.trim().encode_utf16().count();
    if obrigatorio && n == 0 {
        return Err(garde::Error::new("obrigatório"));
    }
    if n > max {
        return Err(garde::Error::new(format!("mais de {max} caracteres")));
    }
    Ok(())
}

/// ISO 8601 com fuso (`z.iso.datetime({ offset: true })` da v1). Guardado como texto para manter o fuso original.
fn iso_8601(v: &String, _: &()) -> garde::Result {
    v.parse::<jiff::Timestamp>().map(drop).map_err(|_| garde::Error::new("data/hora ISO 8601 inválida"))
}

fn iso_8601_opcional(v: &Option<String>, c: &()) -> garde::Result {
    v.as_ref().map_or(Ok(()), |s| iso_8601(s, c))
}

#[derive(Debug)]
pub enum Invalido {
    Limites(garde::Report),
    IdRepetido(Uuid),
    StatusIncoerente,
    AgenteInexistente { atividade: Uuid, agente: Uuid },
    EntidadeUsadaInexistente { atividade: Uuid, entidade: Uuid },
    GeradoraInexistente { entidade: Uuid, atividade: Uuid },
    RevisaoInexistente { entidade: Uuid, revisao: Uuid },
    /// Regras de `cardinalidade.ts` da v1; a mensagem é a da v1.
    Cardinalidade { atividade: Uuid, regra: &'static str },
    Revisao { entidade: Uuid, regra: &'static str },
    Ciclo { atividade: Uuid },
}

impl fmt::Display for Invalido {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Limites(r) => {
                let erros: Vec<_> = r.iter().map(|(caminho, e)| format!("{caminho}: {}", e.message())).collect();
                f.write_str(&erros.join("; "))
            }
            Self::IdRepetido(id) => write!(f, "id repetido: {id}"),
            Self::StatusIncoerente => f.write_str("status incoerente com finalizadoEm"),
            Self::AgenteInexistente { atividade, agente } => write!(f, "atividade {atividade}: agente inexistente {agente}"),
            Self::EntidadeUsadaInexistente { atividade, entidade } => {
                write!(f, "atividade {atividade}: entidade usada inexistente {entidade}")
            }
            Self::GeradoraInexistente { entidade, atividade } => {
                write!(f, "entidade {entidade}: atividade geradora inexistente {atividade}")
            }
            Self::RevisaoInexistente { entidade, revisao } => write!(f, "entidade {entidade}: revisaoDeId inexistente {revisao}"),
            Self::Cardinalidade { atividade, regra } => write!(f, "atividade {atividade}: {regra}"),
            Self::Revisao { entidade, regra } => write!(f, "entidade {entidade}: {regra}"),
            Self::Ciclo { atividade } => write!(f, "atividade {atividade}: ciclo no grafo (usa Entidade gerada por ela mesma ou por uma sucessora)"),
        }
    }
}

pub fn validar(d: &Documento) -> Result<(), Invalido> {
    d.validate().map_err(Invalido::Limites)?;
    let r = &d.registro;
    if (r.status == Status::Finalizado) != r.finalizado_em.is_some() {
        return Err(Invalido::StatusIncoerente);
    }
    let mut ids = HashSet::new();
    let todos = [r.id].into_iter().chain(d.agentes.iter().map(|a| a.id)).chain(d.entidades.iter().map(|e| e.id));
    for id in todos.chain(d.atividades.iter().map(|a| a.id)) {
        if !ids.insert(id) {
            return Err(Invalido::IdRepetido(id));
        }
    }
    let agentes: HashSet<Uuid> = d.agentes.iter().map(|g| g.id).collect();
    let entidades: HashSet<Uuid> = d.entidades.iter().map(|e| e.id).collect();
    let atividades: HashMap<Uuid, &Atividade> = d.atividades.iter().map(|a| (a.id, a)).collect();
    for a in &d.atividades {
        if !agentes.contains(&a.agente_id) {
            return Err(Invalido::AgenteInexistente { atividade: a.id, agente: a.agente_id });
        }
        if let Some(&u) = a.entidades_usadas.iter().find(|u| !entidades.contains(u)) {
            return Err(Invalido::EntidadeUsadaInexistente { atividade: a.id, entidade: u });
        }
    }
    let mut geradoras: HashSet<Uuid> = HashSet::new();
    for e in &d.entidades {
        let Some(geradora) = atividades.get(&e.gerada_por_atividade_id) else {
            return Err(Invalido::GeradoraInexistente { entidade: e.id, atividade: e.gerada_por_atividade_id });
        };
        if let Some(rv) = e.revisao_de_id.filter(|rv| !entidades.contains(rv)) {
            return Err(Invalido::RevisaoInexistente { entidade: e.id, revisao: rv });
        }
        // Derivação/sem relação ignoram revisaoDeId (v1).
        if e.tipo_relacao_origem == Some(TipoRelacaoOrigem::Revisao) {
            let regra = match e.revisao_de_id {
                None => Some("Revisão precisa indicar de qual Entidade de entrada é uma nova versão."),
                Some(rv) if !geradora.entidades_usadas.contains(&rv) => {
                    Some("A Entidade revisada precisa estar entre as Entidades usadas pela Atividade.")
                }
                _ => None,
            };
            if let Some(regra) = regra {
                return Err(Invalido::Revisao { entidade: e.id, regra });
            }
        }
        geradoras.insert(geradora.id);
    }
    for a in &d.atividades {
        let (usa, gera) = (!a.entidades_usadas.is_empty(), geradoras.contains(&a.id));
        let regra = match a.tipo {
            TipoAtividade::Criacao if !gera => "Criação deve gerar 1 ou mais Entidades.",
            TipoAtividade::Transformacao if !usa => "Transformação usa 1 ou mais Entidades.",
            TipoAtividade::Transformacao if !gera => "Transformação deve gerar 1 ou mais Entidades.",
            TipoAtividade::Analise if !usa => "Análise usa 1 ou mais Entidades.",
            _ => continue,
        };
        return Err(Invalido::Cardinalidade { atividade: a.id, regra });
    }
    aciclico(d)
}

/// Kahn sobre as Atividades: aresta geradora(u) → a para cada u em `a.entidades_usadas`.
/// Pressupõe referências já checadas.
fn aciclico(d: &Documento) -> Result<(), Invalido> {
    let geradora: HashMap<Uuid, Uuid> = d.entidades.iter().map(|e| (e.id, e.gerada_por_atividade_id)).collect();
    let mut entrada: HashMap<Uuid, usize> = d.atividades.iter().map(|a| (a.id, a.entidades_usadas.len())).collect();
    let mut seguintes: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for a in &d.atividades {
        for u in &a.entidades_usadas {
            seguintes.entry(geradora[u]).or_default().push(a.id);
        }
    }
    let mut fila: Vec<Uuid> = entrada.iter().filter(|(_, n)| **n == 0).map(|(id, _)| *id).collect();
    while let Some(id) = fila.pop() {
        for s in seguintes.get(&id).into_iter().flatten() {
            let n = entrada.get_mut(s).expect("atividade existe");
            *n -= 1;
            if *n == 0 {
                fila.push(*s);
            }
        }
        entrada.remove(&id);
    }
    // Sobra = Atividades no ciclo ou depois dele; reporta a primeira na ordem do arquivo.
    match d.atividades.iter().find(|a| entrada.contains_key(&a.id)) {
        Some(a) => Err(Invalido::Ciclo { atividade: a.id }),
        None => Ok(()),
    }
}

impl TipoAtividade {
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::Criacao => "Criação",
            Self::Transformacao => "Transformação",
            Self::Analise => "Análise",
        }
    }
}

impl TipoAgente {
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::Pessoa => "Pessoa",
            Self::Instituicao => "Instituição",
            Self::Software => "Software",
        }
    }
}

impl Status {
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::Rascunho => "Rascunho",
            Self::Finalizado => "Finalizado",
        }
    }
}

pub fn agora() -> String {
    jiff::Timestamp::now().round(jiff::Unit::Millisecond).expect("arredondar ms").to_string()
}

pub fn novo_documento(titulo: &str) -> Documento {
    let agora = agora();
    Documento {
        schema_version: SCHEMA_VERSION,
        registro: Registro {
            id: Uuid::now_v7(),
            titulo: titulo.into(),
            descricao: None,
            status: Status::Rascunho,
            direcao_diagrama: Direcao::TD,
            criado_em: agora.clone(),
            atualizado_em: agora,
            finalizado_em: None,
        },
        agentes: vec![],
        entidades: vec![],
        atividades: vec![],
    }
}

/// Igual ao `slugify` da v1 (`src/lib/slug.ts`).
pub fn slug(texto: &str) -> String {
    use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};
    let mut s = String::new();
    for c in texto.nfd().filter(|c| !is_combining_mark(*c)).flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            s.push(c);
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_matches('-');
    if s.is_empty() { "registro".into() } else { s.into() }
}

/// Registro de exemplo válido (Rascunho, 3 Atividades, Revisão), base dos testes.
#[cfg(test)]
pub fn exemplo() -> serde_json::Value {
    serde_json::json!({
      "schemaVersion": 4,
      "registro": {
        "id": "0199a8b0-1c2d-7e3f-8a4b-5c6d7e8f9a01",
        "titulo": "Levantamento de aves — Mata Atlântica 2025",
        "descricao": "## Objetivo\n\nDocumentar a **origem** das planilhas.",
        "status": "rascunho",
        "direcaoDiagrama": "TD",
        "criadoEm": "2025-03-02T12:00:00.000Z",
        "atualizadoEm": "2025-10-01T18:30:00.000Z",
        "finalizadoEm": null
      },
      "agentes": [
        {"id": "0199a8b0-0000-7000-8000-00000000a001", "nome": "Ana Souza", "tipo": "pessoa", "afiliacao": "JBRJ", "identificadorExterno": null},
        {"id": "0199a8b0-0000-7000-8000-00000000a002", "nome": "R 4.4", "tipo": "software", "afiliacao": null, "identificadorExterno": null}
      ],
      "entidades": [
        {"id": "0199a8b0-0000-7000-8000-00000000e001", "nome": "Planilha de campo", "descricao": null, "formato": "XLSX", "localizacao": null, "licenca": "CC BY 4.0", "geradaPorAtividadeId": "0199a8b0-0000-7000-8000-00000000c001", "tipoRelacaoOrigem": null, "revisaoDeId": null},
        {"id": "0199a8b0-0000-7000-8000-00000000e002", "nome": "Planilha limpa", "descricao": null, "formato": "CSV", "localizacao": null, "licenca": null, "geradaPorAtividadeId": "0199a8b0-0000-7000-8000-00000000c002", "tipoRelacaoOrigem": "revisao", "revisaoDeId": "0199a8b0-0000-7000-8000-00000000e001"}
      ],
      "atividades": [
        {"id": "0199a8b0-0000-7000-8000-00000000c001", "tipo": "criacao", "agenteId": "0199a8b0-0000-7000-8000-00000000a001", "dataHora": "2025-03-10T09:00:00-03:00", "descricao": "Coleta em campo", "entidadesUsadas": [], "local": "PARNA Tijuca", "instrumento": null, "processo": null, "parametros": null, "ambienteExecucao": null},
        {"id": "0199a8b0-0000-7000-8000-00000000c002", "tipo": "transformacao", "agenteId": "0199a8b0-0000-7000-8000-00000000a002", "dataHora": "2025-04-02T14:00:00-03:00", "descricao": null, "entidadesUsadas": ["0199a8b0-0000-7000-8000-00000000e001"], "local": null, "instrumento": null, "processo": "limpeza.R", "parametros": [{"chave": "taxonomia", "valor": "CBRO 2024"}], "ambienteExecucao": {"sistemaOperacional": "Windows 11", "pacotes": [{"nome": "dplyr", "versao": "1.1.4"}]}}
      ]
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn doc(ajuste: impl FnOnce(&mut serde_json::Value)) -> Result<(), String> {
        let mut v = exemplo();
        ajuste(&mut v);
        let d: Documento = serde_json::from_value(v).map_err(|e| e.to_string())?;
        validar(&d).map_err(|e| e.to_string())
    }

    #[test]
    fn exemplo_e_valido() {
        assert_eq!(doc(|_| {}), Ok(()));
    }

    #[test]
    fn limites_contam_utf16_depois_do_trim() {
        assert_eq!(doc(|v| v["registro"]["titulo"] = format!("  {}  ", "a".repeat(300)).into()), Ok(()));
        let erro = doc(|v| v["registro"]["titulo"] = "a".repeat(301).into()).unwrap_err();
        assert!(erro.contains("registro.titulo: mais de 300"), "{erro}");
        // Emoji fora do BMP = 2 unidades UTF-16, como no zod.
        assert!(doc(|v| v["registro"]["titulo"] = "🦜".repeat(151).into()).is_err());
        assert_eq!(doc(|v| v["registro"]["titulo"] = "🦜".repeat(150).into()), Ok(()));
        let erro = doc(|v| v["registro"]["titulo"] = "   ".into()).unwrap_err();
        assert!(erro.contains("obrigatório"), "{erro}");
        let erro = doc(|v| v["atividades"][1]["parametros"][0]["chave"] = "".into()).unwrap_err();
        assert!(erro.contains("atividades[1].parametros[0].chave"), "{erro}");
    }

    #[test]
    fn data_sem_fuso_e_invalida() {
        assert!(doc(|v| v["atividades"][0]["dataHora"] = "2025-03-10T09:00:00".into()).is_err());
    }

    #[test]
    fn integridade() {
        let erro = doc(|v| v["registro"]["status"] = "finalizado".into()).unwrap_err();
        assert!(erro.contains("status incoerente"), "{erro}");
        let erro = doc(|v| v["atividades"][1]["entidadesUsadas"][0] = "0199a8b0-0000-7000-8000-0000000000ff".into()).unwrap_err();
        assert!(erro.contains("entidade usada inexistente"), "{erro}");
        let erro = doc(|v| v["entidades"][1]["revisaoDeId"] = "0199a8b0-0000-7000-8000-0000000000ff".into()).unwrap_err();
        assert!(erro.contains("revisaoDeId inexistente"), "{erro}");
        let erro = doc(|v| v["entidades"][1]["id"] = "0199a8b0-0000-7000-8000-00000000a001".into()).unwrap_err();
        assert!(erro.contains("id repetido"), "{erro}");
    }

    const E1: &str = "0199a8b0-0000-7000-8000-00000000e001";
    const E2: &str = "0199a8b0-0000-7000-8000-00000000e002";

    /// Exemplo + Atividade nova (`tipo`, usando E1 ou nada, gerando `gera` Entidades novas).
    fn com_atividade(tipo: &str, usa: bool, gera: usize) -> Result<(), String> {
        doc(|v| {
            let id = "0199a8b0-0000-7000-8000-00000000c0f0";
            let usadas = if usa { vec![E1] } else { vec![] };
            v["atividades"].as_array_mut().unwrap().push(serde_json::json!({
                "id": id, "tipo": tipo, "agenteId": "0199a8b0-0000-7000-8000-00000000a001",
                "dataHora": "2025-05-01T10:00:00Z", "descricao": null, "entidadesUsadas": usadas, "local": null,
                "instrumento": null, "processo": null, "parametros": null, "ambienteExecucao": null
            }));
            for i in 0..gera {
                v["entidades"].as_array_mut().unwrap().push(serde_json::json!({
                    "id": format!("0199a8b0-0000-7000-8000-00000000ef{i:02}"), "nome": "saída", "descricao": null,
                    "formato": null, "localizacao": null, "licenca": null, "geradaPorAtividadeId": id,
                    "tipoRelacaoOrigem": null, "revisaoDeId": null
                }));
            }
        })
    }

    /// Spec: `cardinalidade.test.ts` da v1 (validarCardinalidade).
    #[test]
    fn cardinalidade_por_tipo() {
        let casos = [
            ("criacao", false, 1, None),
            ("criacao", true, 1, None),
            ("criacao", false, 0, Some("Criação deve gerar")),
            ("transformacao", true, 1, None),
            ("transformacao", false, 1, Some("Transformação usa")),
            ("transformacao", true, 0, Some("Transformação deve gerar")),
            ("analise", true, 0, None),
            ("analise", true, 2, None),
            ("analise", false, 0, Some("Análise usa")),
        ];
        for (tipo, usa, gera, esperado) in casos {
            match (com_atividade(tipo, usa, gera), esperado) {
                (Ok(()), None) => {}
                (Err(e), Some(m)) if e.contains(m) => {}
                (r, _) => panic!("{tipo} usa={usa} gera={gera}: {r:?}"),
            }
        }
    }

    /// Spec: `cardinalidade.test.ts` da v1 (validarRelacoesGeradas). E2 = revisão de E1, gerada por c002 que usa E1.
    #[test]
    fn revisao_exige_fonte_entre_as_usadas() {
        let erro = doc(|v| v["entidades"][1]["revisaoDeId"] = serde_json::Value::Null).unwrap_err();
        assert!(erro.contains("Revisão precisa indicar"), "{erro}");
        let erro = doc(|v| v["entidades"][1]["revisaoDeId"] = E2.into()).unwrap_err();
        assert!(erro.contains("precisa estar entre as Entidades usadas"), "{erro}");
        assert_eq!(doc(|v| v["entidades"][1]["tipoRelacaoOrigem"] = "derivacao".into()), Ok(()));
        assert_eq!(
            doc(|v| {
                v["entidades"][1]["tipoRelacaoOrigem"] = serde_json::Value::Null;
                v["entidades"][1]["revisaoDeId"] = serde_json::Value::Null;
            }),
            Ok(())
        );
    }

    #[test]
    fn ciclo_e_invalido() {
        // c002 usa E2, que ela mesma gera.
        let erro = doc(|v| v["atividades"][1]["entidadesUsadas"].as_array_mut().unwrap().push(E2.into())).unwrap_err();
        assert!(erro.contains("ciclo"), "{erro}");
        // c001 (Criação com entrada) usa E2 ← c002 usa E1 ← c001.
        let erro = doc(|v| v["atividades"][0]["entidadesUsadas"] = serde_json::json!([E2])).unwrap_err();
        assert!(erro.contains("ciclo"), "{erro}");
    }

    #[test]
    fn slug_igual_v1() {
        assert_eq!(slug("Ação nº 1 — Teste"), "acao-n-1-teste");
        assert_eq!(slug("—"), "registro");
    }
}
