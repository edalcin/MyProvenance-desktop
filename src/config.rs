//! `%APPDATA%\MyProvenance\config.json` (decisão #5). Sem `deny_unknown_fields`: config não é dado do usuário.
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// `None` = pasta padrão (`Documentos\MyProvenance`).
    pub pasta_dados: Option<PathBuf>,
    pub tema: Tema,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tema {
    #[default]
    Sistema,
    Claro,
    Escuro,
}

fn caminho() -> io::Result<PathBuf> {
    directories::BaseDirs::new()
        .map(|d| d.config_dir().join("MyProvenance").join("config.json"))
        .ok_or_else(|| io::Error::other("pasta de configuração do usuário não encontrada"))
}

pub fn pasta_padrao() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|d| d.document_dir().map(|p| p.join("MyProvenance")))
        .unwrap_or_else(|| PathBuf::from("MyProvenance"))
}

impl Config {
    /// Config ausente = padrão. Config ilegível = padrão + mensagem para o aviso.
    pub fn ler() -> (Self, Option<String>) {
        let lido = caminho().and_then(fs::read).map_err(|e| (e.kind() != io::ErrorKind::NotFound).then(|| e.to_string()));
        match lido.map(|b| serde_json::from_slice(&b)) {
            Ok(Ok(c)) => (c, None),
            Ok(Err(e)) => (Self::default(), Some(format!("config.json inválido, usando o padrão: {e}"))),
            Err(e) => (Self::default(), e),
        }
    }

    pub fn gravar(&self) -> io::Result<()> {
        let c = caminho()?;
        fs::create_dir_all(c.parent().expect("config.json tem pasta"))?;
        let mut s = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        s.push('\n');
        crate::store::escrever(&c, &s, false).map(drop)
    }

    pub fn pasta(&self) -> PathBuf {
        self.pasta_dados.clone().unwrap_or_else(pasta_padrao)
    }
}
