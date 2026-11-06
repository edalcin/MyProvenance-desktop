//! Port de `mermaid.ts`, `report.ts` e `relacao.ts` da v1 (só pt-BR, ADR-0007).
//! `export.ts` não tem port: na v2 o arquivo do Registro já é o JSON (`store::serializar`, ADR-0014).
// ponytail: a Etapa 3 liga isto ao botão Exportar; até lá só os testes usam.
#![cfg_attr(not(test), allow(dead_code))]
use crate::model::{Direcao, Documento, Entidade, TipoRelacaoOrigem};
use jiff::Zoned;
use std::collections::HashMap;
use uuid::Uuid;

/// Texto não vazio (truthiness do JS na v1).
fn cheio(s: &Option<String>) -> Option<&str> {
    s.as_deref().filter(|s| !s.is_empty())
}

fn nome_entidade(d: &Documento, id: Uuid) -> Option<&str> {
    d.entidades.iter().find(|e| e.id == id).map(|e| e.nome.as_str())
}

/// " (revisão de X)" / " (derivação)" para o nome de uma Entidade gerada; "" sem relação.
pub fn sufixo_relacao(d: &Documento, e: &Entidade) -> String {
    match e.tipo_relacao_origem {
        Some(TipoRelacaoOrigem::Revisao) => {
            let nome = e.revisao_de_id.and_then(|id| nome_entidade(d, id)).unwrap_or("");
            format!(" (revisão de {nome})")
        }
        Some(TipoRelacaoOrigem::Derivacao) => " (derivação)".into(),
        None => String::new(),
    }
}

fn geradas<'a>(d: &'a Documento, atividade: Uuid) -> impl Iterator<Item = &'a Entidade> {
    d.entidades.iter().filter(move |e| e.gerada_por_atividade_id == atividade)
}

/// Diagrama Mermaid (texto) do grafo do Registro.
pub fn mermaid(d: &Documento) -> String {
    fn escapar(s: &str) -> String {
        s.replace('"', "'").split(['\r', '\n']).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
    }
    let no: HashMap<Uuid, String> = d.entidades.iter().enumerate().map(|(i, e)| (e.id, format!("E{}", i + 1))).collect();
    let direcao = match d.registro.direcao_diagrama {
        Direcao::LR => "LR",
        Direcao::TD => "TD",
    };
    let mut linhas = vec![format!("flowchart {direcao}")];
    for e in &d.entidades {
        let rotulo = cheio(&e.formato).map_or(e.nome.clone(), |f| format!("{} ({f})", e.nome));
        linhas.push(format!("  {}[\"{}\"]", no[&e.id], escapar(&rotulo)));
    }
    for a in &d.atividades {
        // Criação sem entrada = raiz da linhagem, sem seta. Análise sem saída não aparece (só no relatório).
        if a.entidades_usadas.is_empty() {
            continue;
        }
        let agente = d.agentes.iter().find(|g| g.id == a.agente_id).map_or("Agente", |g| g.nome.as_str());
        let resumo = cheio(&a.descricao).map_or(String::new(), |t| format!(": {}", escapar(t)));
        let data = a.data_hora.get(..10).unwrap_or(&a.data_hora);
        let rotulo = format!("{}{resumo} ({}, {data})", a.tipo.rotulo(), escapar(agente));
        for g in geradas(d, a.id) {
            for u in &a.entidades_usadas {
                let (Some(origem), destino) = (no.get(u), &no[&g.id]) else { continue };
                match g.tipo_relacao_origem {
                    // Revisão é 1:1 com a fonte: as outras Entidades usadas não geraram esta.
                    Some(TipoRelacaoOrigem::Revisao) if g.revisao_de_id != Some(*u) => {}
                    Some(TipoRelacaoOrigem::Revisao) => linhas.push(format!("  {origem} -.->|\"{rotulo} (revisão)\"| {destino}")),
                    Some(TipoRelacaoOrigem::Derivacao) => linhas.push(format!("  {origem} -->|\"{rotulo} (derivação)\"| {destino}")),
                    None => linhas.push(format!("  {origem} -->|\"{rotulo}\"| {destino}")),
                }
            }
        }
    }
    let revisadas: Vec<&str> = d
        .entidades
        .iter()
        .filter(|e| e.tipo_relacao_origem == Some(TipoRelacaoOrigem::Revisao))
        .map(|e| no[&e.id].as_str())
        .collect();
    if !revisadas.is_empty() {
        linhas.push("  classDef revisada stroke-dasharray:5 5,stroke-width:2px".into());
        linhas.push(format!("  class {} revisada", revisadas.join(",")));
    }
    linhas.join("\n")
}

const MESES: [&str; 12] = ["jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.", "dez."];

/// `Intl.DateTimeFormat('pt-BR', { dateStyle: 'medium' })`: "5 de mar. de 2026".
fn data(z: &Zoned) -> String {
    format!("{} de {} de {}", z.day(), MESES[z.month() as usize - 1], z.year())
}

/// Relatório Markdown: cabeçalho, diagrama, Entidades, linha do tempo, Agentes.
/// Datas no fuso de `exportado_em` (o do sistema, na chamada real: `Zoned::now()`).
pub fn relatorio(d: &Documento, exportado_em: &Zoned) -> String {
    fn celula(s: &str) -> String {
        s.replace('|', "\\|").split(['\r', '\n']).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
    }
    let r = &d.registro;
    let nome = |id: Uuid| nome_entidade(d, id).map_or(id.to_string(), str::to_owned);
    let mut l: Vec<String> = vec![format!("# {}", r.titulo), String::new()];
    if let Some(desc) = cheio(&r.descricao) {
        l.extend([desc.to_owned(), String::new()]);
    }
    l.push(format!("**Status:** {}", r.status.rotulo()));
    l.push(format!(
        "**Exportado em:** {}, {:02}:{:02}",
        data(exportado_em),
        exportado_em.hour(),
        exportado_em.minute()
    ));
    l.push(String::new());
    l.extend(["## Diagrama".into(), String::new(), "```mermaid".into(), mermaid(d), "```".into(), String::new()]);

    l.extend(["## Entidades".into(), String::new()]);
    if d.entidades.is_empty() {
        l.extend(["_Nenhuma Entidade._".into(), String::new()]);
    } else {
        l.extend(["| Nome | Descrição |".into(), "|---|---|".into()]);
        for e in &d.entidades {
            let desc = cheio(&e.descricao).map_or("—".into(), celula);
            l.push(format!("| {} | {desc} |", celula(&(e.nome.clone() + &sufixo_relacao(d, e)))));
        }
        l.push(String::new());
    }

    l.extend(["## Linha do tempo de Atividades".into(), String::new()]);
    if d.atividades.is_empty() {
        l.extend(["_Nenhuma Atividade._".into(), String::new()]);
    } else {
        l.extend([
            "| Tipo | Data | Agente | Entidades usadas → Entidades geradas | Detalhes |".into(),
            "|---|---|---|---|---|".into(),
        ]);
        for a in &d.atividades {
            let usadas: Vec<String> = a.entidades_usadas.iter().map(|&u| nome(u)).collect();
            let geradas: Vec<String> = geradas(d, a.id).map(|e| e.nome.clone() + &sufixo_relacao(d, e)).collect();
            let geradas = if geradas.is_empty() { "(sem saída)".into() } else { geradas.join(", ") };
            let fluxo = if usadas.is_empty() { geradas } else { format!("{} → {geradas}", usadas.join(", ")) };

            let mut det: Vec<String> = vec![];
            det.extend(cheio(&a.descricao).map(str::to_owned));
            det.extend(cheio(&a.local).map(|s| format!("Local: {s}")));
            det.extend(cheio(&a.instrumento).map(|s| format!("Ferramenta ou Software: {s}")));
            det.extend(cheio(&a.processo).map(|s| format!("Processo: {s}")));
            if let Some(p) = a.parametros.as_ref().filter(|p| !p.is_empty()) {
                let p: Vec<String> = p.iter().map(|p| format!("{}={}", p.chave, p.valor)).collect();
                det.push(format!("Parâmetros: {}", p.join(", ")));
            }
            if let Some(amb) = &a.ambiente_execucao {
                det.extend(cheio(&amb.sistema_operacional).map(|s| format!("SO: {s}")));
                if let Some(p) = amb.pacotes.as_ref().filter(|p| !p.is_empty()) {
                    let p: Vec<String> = p.iter().map(|p| format!("{}@{}", p.nome, p.versao)).collect();
                    det.push(format!("Pacotes: {}", p.join(", ")));
                }
            }
            let det = if det.is_empty() { "—".into() } else { det.join("; ") };
            let quando = a.data_hora.parse::<jiff::Timestamp>().map_or(a.data_hora.clone(), |t| data(&t.to_zoned(exportado_em.time_zone().clone())));
            let agente = d.agentes.iter().find(|g| g.id == a.agente_id).map_or("—".into(), |g| celula(&g.nome));
            l.push(format!("| {} | {quando} | {agente} | {} | {} |", a.tipo.rotulo(), celula(&fluxo), celula(&det)));
        }
        l.push(String::new());
    }

    // Agentes envolvidos = os usados por alguma Atividade (ADR-0013: o arquivo pode trazer outros até gravar).
    let envolvidos: Vec<_> = d.agentes.iter().filter(|g| d.atividades.iter().any(|a| a.agente_id == g.id)).collect();
    l.extend(["## Agentes envolvidos".into(), String::new()]);
    if envolvidos.is_empty() {
        l.extend(["_Nenhum Agente._".into(), String::new()]);
    } else {
        l.extend(["| Nome | Tipo | Afiliação |".into(), "|---|---|---|".into()]);
        for g in envolvidos {
            let afiliacao = g.afiliacao.as_deref().map_or("—".into(), celula);
            l.push(format!("| {} | {} | {afiliacao} |", celula(&g.nome), g.tipo.rotulo()));
        }
        l.push(String::new());
    }

    l.extend([
        "---".into(),
        String::new(),
        "Relatório de proveniência gerado pelo MyProvenance".into(),
        "https://github.com/edalcin/MyProvenance-desktop".into(),
    ]);
    l.join("\n")
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::model::{self, Agente, Atividade, TipoAgente, TipoAtividade};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    const AGENTE: u128 = 0xa1;

    fn ent(n: u128, nome: &str, gerada_por: u128) -> Entidade {
        Entidade {
            id: id(n),
            nome: nome.into(),
            descricao: None,
            formato: Some("CSV".into()),
            localizacao: None,
            licenca: None,
            gerada_por_atividade_id: id(gerada_por),
            tipo_relacao_origem: None,
            revisao_de_id: None,
        }
    }

    fn revisao(mut e: Entidade, de: u128) -> Entidade {
        e.tipo_relacao_origem = Some(TipoRelacaoOrigem::Revisao);
        e.revisao_de_id = Some(id(de));
        e
    }

    fn ativ(n: u128, tipo: TipoAtividade, usadas: &[u128], descricao: Option<&str>) -> Atividade {
        Atividade {
            id: id(n),
            tipo,
            agente_id: id(AGENTE),
            data_hora: "2026-03-05T10:00:00.000Z".into(),
            descricao: descricao.map(Into::into),
            entidades_usadas: usadas.iter().map(|&u| id(u)).collect(),
            local: None,
            instrumento: None,
            processo: None,
            parametros: None,
            ambiente_execucao: None,
        }
    }

    fn documento(entidades: Vec<Entidade>, atividades: Vec<Atividade>) -> Documento {
        let mut d = model::novo_documento("Registro vazio");
        d.registro.direcao_diagrama = Direcao::LR;
        d.agentes = vec![Agente { id: id(AGENTE), nome: "Fulana".into(), tipo: TipoAgente::Pessoa, afiliacao: None, identificador_externo: None }];
        d.entidades = entidades;
        d.atividades = atividades;
        d
    }

    use TipoAtividade::{Analise, Criacao, Transformacao};

    // e1/e2/… = 0xe1…, a1/a2/… = 0x1…
    fn bruto_e_limpo(e2: Entidade, descricao: Option<&str>) -> Documento {
        documento(
            vec![ent(0xe1, "bruto.csv", 1), e2],
            vec![ativ(1, Criacao, &[], None), ativ(2, Transformacao, &[0xe1], descricao)],
        )
    }

    // Spec: `mermaid.test.ts` da v1.
    #[test]
    fn criacao_vira_no_raiz_sem_seta() {
        let s = mermaid(&documento(vec![ent(0xe1, "campo_bruto.csv", 1)], vec![ativ(1, Criacao, &[], None)]));
        assert!(s.contains(r#"E1["campo_bruto.csv (CSV)"]"#), "{s}");
        assert!(!s.contains("-->"), "{s}");
        assert!(!s.contains("classDef revisada"), "{s}");
    }

    #[test]
    fn transformacao_desenha_seta_rotulada() {
        let s = mermaid(&bruto_e_limpo(ent(0xe2, "limpo.csv", 2), Some("limpeza")));
        assert!(s.contains(r#"E1 -->|"Transformação: limpeza (Fulana, 2026-03-05)"| E2"#), "{s}");
        assert!(s.starts_with("flowchart LR"), "{s}");
    }

    #[test]
    fn revisao_seta_pontilhada_e_destaque() {
        let s = mermaid(&bruto_e_limpo(revisao(ent(0xe2, "bruto_v2.csv", 2), 0xe1), Some("nova versao")));
        assert!(s.contains(r#"E1 -.->|"Transformação: nova versao (Fulana, 2026-03-05) (revisão)"| E2"#), "{s}");
        assert!(s.contains("classDef revisada stroke-dasharray:5 5,stroke-width:2px"), "{s}");
        assert!(s.contains("class E2 revisada"), "{s}");
    }

    #[test]
    fn derivacao_seta_solida_rotulada() {
        let mut e2 = ent(0xe2, "derivado.csv", 2);
        e2.tipo_relacao_origem = Some(TipoRelacaoOrigem::Derivacao);
        let s = mermaid(&bruto_e_limpo(e2, Some("derivado")));
        assert!(s.contains(r#"E1 -->|"Transformação: derivado (Fulana, 2026-03-05) (derivação)"| E2"#), "{s}");
    }

    #[test]
    fn revisao_de_duas_liga_cada_uma_a_sua_fonte() {
        let d = documento(
            vec![
                ent(0xe1, "vernacularMartius", 1),
                ent(0xe2, "dicVernacularMartius", 1),
                revisao(ent(0xe3, "vernacularMartius", 2), 0xe1),
                revisao(ent(0xe4, "dicVernacularMartius", 2), 0xe2),
            ],
            vec![ativ(1, Criacao, &[], None), ativ(2, Transformacao, &[0xe1, 0xe2], Some("revisao"))],
        );
        let s = mermaid(&d);
        let setas: Vec<_> = s.lines().filter(|l| l.contains("->")).collect();
        assert_eq!(setas.len(), 2, "{s}");
        assert!(setas[0].trim().starts_with("E1 -.->") && setas[0].ends_with("E3"), "{s}");
        assert!(setas[1].trim().starts_with("E2 -.->") && setas[1].ends_with("E4"), "{s}");
    }

    #[test]
    fn fusao_e_saida_multipla() {
        let fusao = documento(
            vec![ent(0xe1, "a.csv", 1), ent(0xe2, "b.csv", 2), ent(0xe3, "combinado.csv", 3)],
            vec![ativ(1, Criacao, &[], None), ativ(2, Criacao, &[], None), ativ(3, Transformacao, &[0xe1, 0xe2], None)],
        );
        let s = mermaid(&fusao);
        assert!(s.contains(r#"E1 -->|"Transformação (Fulana, 2026-03-05)"| E3"#), "{s}");
        assert!(s.contains(r#"E2 -->|"Transformação (Fulana, 2026-03-05)"| E3"#), "{s}");

        let multipla = documento(
            vec![ent(0xe1, "bruto.csv", 1), ent(0xe2, "parte1.csv", 2), ent(0xe3, "parte2.csv", 2)],
            vec![ativ(1, Criacao, &[], None), ativ(2, Transformacao, &[0xe1], None)],
        );
        let s = mermaid(&multipla);
        assert!(s.contains(r#"E1 -->|"Transformação (Fulana, 2026-03-05)"| E2"#), "{s}");
        assert!(s.contains(r#"E1 -->|"Transformação (Fulana, 2026-03-05)"| E3"#), "{s}");
    }

    #[test]
    fn analise_sem_saida_fora_do_diagrama_e_aspas_escapadas() {
        let d = documento(
            vec![ent(0xe1, r#"arquivo "especial".csv"#, 1)],
            vec![ativ(1, Criacao, &[], None), ativ(2, Analise, &[0xe1], None)],
        );
        let s = mermaid(&d);
        assert!(!s.contains("Análise"), "{s}");
        assert!(s.contains("arquivo 'especial'.csv"), "{s}");
    }

    fn utc(s: &str) -> Zoned {
        s.parse::<jiff::Timestamp>().unwrap().to_zoned(jiff::tz::TimeZone::UTC)
    }

    // Spec: `report.test.ts` da v1.
    #[test]
    fn relatorio_vazio_tem_cabecalho_e_secoes() {
        let mut d = documento(vec![], vec![]);
        d.agentes.clear();
        let md = relatorio(&d, &utc("2026-03-15T18:00:00Z"));
        for trecho in [
            "# Registro vazio",
            "**Status:** Rascunho",
            "**Exportado em:** 15 de mar. de 2026, 18:00",
            "```mermaid\nflowchart LR\n```",
            "## Entidades",
            "_Nenhuma Entidade._",
            "## Linha do tempo de Atividades",
            "_Nenhuma Atividade._",
            "## Agentes envolvidos",
            "_Nenhum Agente._",
        ] {
            assert!(md.contains(trecho), "falta {trecho:?} em:\n{md}");
        }
    }

    #[test]
    fn relatorio_escapa_pipe_e_quebra_de_linha_nas_celulas() {
        let mut d = documento(vec![ent(0xe1, "arquivo|com|pipe.csv", 1)], vec![ativ(1, Criacao, &[], Some("linha 1\nlinha 2"))]);
        d.agentes[0].afiliacao = Some("A|B".into());
        let md = relatorio(&d, &utc("2026-03-15T18:00:00Z"));
        assert!(md.contains("| arquivo\\|com\\|pipe.csv | — |"), "{md}");
        assert!(md.contains("| Criação | 5 de mar. de 2026 | Fulana | arquivo\\|com\\|pipe.csv | linha 1 linha 2 |"), "{md}");
        assert!(md.contains("| Fulana | Pessoa | A\\|B |"), "{md}");
    }

    #[test]
    fn relatorio_mostra_revisao_de_x() {
        let d = bruto_e_limpo(revisao(ent(0xe2, "bruto_v2.csv", 2), 0xe1), None);
        let md = relatorio(&d, &utc("2026-03-15T18:00:00Z"));
        assert!(md.contains("| bruto_v2.csv (revisão de bruto.csv) | — |"), "{md}");
        assert!(md.contains("| bruto.csv → bruto_v2.csv (revisão de bruto.csv) |"), "{md}");
    }

    #[test]
    fn relatorio_data_no_fuso_da_exportacao() {
        // 2026-03-05T01:00Z = 4 de mar. em São Paulo (-03:00).
        let mut d = documento(vec![ent(0xe1, "x", 1)], vec![ativ(1, Criacao, &[], None)]);
        d.atividades[0].data_hora = "2026-03-05T01:00:00Z".into();
        let sp = "2026-03-15T18:00:00Z".parse::<jiff::Timestamp>().unwrap().in_tz("America/Sao_Paulo").unwrap();
        let md = relatorio(&d, &sp);
        assert!(md.contains("**Exportado em:** 15 de mar. de 2026, 15:00"), "{md}");
        assert!(md.contains("| Criação | 4 de mar. de 2026 |"), "{md}");
    }
}
