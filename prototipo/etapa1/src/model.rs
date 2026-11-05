//! Formato JSON v2 = `schemaVersion: 4` (ADR-0014). Ordem dos campos = ordem de serialização.
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 4;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Documento {
    pub schema_version: u32,
    pub registro: Registro,
    pub agentes: Vec<Agente>,
    pub entidades: Vec<Entidade>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Registro {
    pub id: String,
    pub titulo: String,
    /// Markdown (v1: HTML do TipTap).
    pub descricao: Option<String>,
    pub status: Status,
    pub direcao_diagrama: Direcao,
    pub criado_em: String,
    pub atualizado_em: String,
    pub finalizado_em: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TipoAgente {
    Pessoa,
    Instituicao,
    Software,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Agente {
    pub id: String,
    pub nome: String,
    pub tipo: TipoAgente,
    pub afiliacao: Option<String>,
    pub identificador_externo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TipoRelacaoOrigem {
    Derivacao,
    Revisao,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entidade {
    pub id: String,
    pub nome: String,
    pub descricao: Option<String>,
    pub formato: Option<String>,
    pub localizacao: Option<String>,
    pub licenca: Option<String>,
    pub gerada_por_atividade_id: String,
    pub tipo_relacao_origem: Option<TipoRelacaoOrigem>,
    pub revisao_de_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TipoAtividade {
    Criacao,
    Transformacao,
    Analise,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Parametro {
    pub chave: String,
    pub valor: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Pacote {
    pub nome: String,
    pub versao: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AmbienteExecucao {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sistema_operacional: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pacotes: Option<Vec<Pacote>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Atividade {
    pub id: String,
    pub tipo: TipoAtividade,
    pub agente_id: String,
    pub data_hora: String,
    pub descricao: Option<String>,
    pub entidades_usadas: Vec<String>,
    pub local: Option<String>,
    pub instrumento: Option<String>,
    pub processo: Option<String>,
    pub parametros: Option<Vec<Parametro>>,
    pub ambiente_execucao: Option<AmbienteExecucao>,
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

/// Regras de "Registro válido" que cabem no protótipo. Ciclo e cardinalidade/Revisão entram no port (Etapa 2).
pub fn validar(d: &Documento) -> Result<(), String> {
    use std::collections::HashSet;
    let r = &d.registro;
    if r.titulo.trim().is_empty() || r.titulo.chars().count() > 300 {
        return Err("registro.titulo vazio ou > 300 caracteres".into());
    }
    if r.descricao.as_deref().is_some_and(|s| s.chars().count() > 20000) {
        return Err("registro.descricao > 20000 caracteres".into());
    }
    for (campo, v) in [("criadoEm", Some(&r.criado_em)), ("atualizadoEm", Some(&r.atualizado_em)), ("finalizadoEm", r.finalizado_em.as_ref())] {
        if let Some(v) = v {
            v.parse::<jiff::Timestamp>().map_err(|e| format!("registro.{campo} inválido: {e}"))?;
        }
    }
    if (r.status == Status::Finalizado) != r.finalizado_em.is_some() {
        return Err("status incoerente com finalizadoEm".into());
    }
    let mut ids = HashSet::new();
    let todos = d.agentes.iter().map(|a| &a.id).chain(d.entidades.iter().map(|e| &e.id)).chain(d.atividades.iter().map(|a| &a.id));
    for id in todos.chain([&r.id]) {
        uuid::Uuid::parse_str(id).map_err(|_| format!("id não é UUID: {id}"))?;
        if !ids.insert(id.as_str()) {
            return Err(format!("id repetido: {id}"));
        }
    }
    let agentes: HashSet<_> = d.agentes.iter().map(|a| a.id.as_str()).collect();
    let entidades: HashSet<_> = d.entidades.iter().map(|e| e.id.as_str()).collect();
    let atividades: HashSet<_> = d.atividades.iter().map(|a| a.id.as_str()).collect();
    for a in &d.atividades {
        if !agentes.contains(a.agente_id.as_str()) {
            return Err(format!("atividade {}: agente inexistente {}", a.id, a.agente_id));
        }
        if let Some(u) = a.entidades_usadas.iter().find(|u| !entidades.contains(u.as_str())) {
            return Err(format!("atividade {}: entidade usada inexistente {u}", a.id));
        }
        a.data_hora.parse::<jiff::Timestamp>().map_err(|e| format!("atividade {} dataHora inválida: {e}", a.id))?;
    }
    for e in &d.entidades {
        if !atividades.contains(e.gerada_por_atividade_id.as_str()) {
            return Err(format!("entidade {}: atividade geradora inexistente", e.id));
        }
        if e.revisao_de_id.as_deref().is_some_and(|r| !entidades.contains(r)) {
            return Err(format!("entidade {}: revisaoDeId inexistente", e.id));
        }
    }
    Ok(())
}

pub fn agora() -> String {
    jiff::Timestamp::now().round(jiff::Unit::Millisecond).unwrap().to_string()
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
