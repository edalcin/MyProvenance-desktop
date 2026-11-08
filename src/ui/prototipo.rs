//! PROTÓTIPO DESCARTÁVEL (ticket #13 "Formulários de Atividade, Entidade e Agente na tela do Registro").
//! Três variantes de onde/como editar, trocadas pela barra flutuante (◀ ▶) ou ←/→ (sem foco em campo):
//! A = tela própria com Voltar (formulário transacional, como a v1);
//! B = painel lateral ao lado do Registro (o mesmo formulário transacional);
//! C = edição inline nos cards das Atividades (sem formulário: edita direto a cópia de trabalho, erro no card).
//! Variante inicial: env `VARIANTE=A|B|C`. Vive só na tag `prototipo-formularios`; nunca no `main`.
use super::{data, icone};
use crate::model::{
    self, Agente, AmbienteExecucao, Atividade, Documento, Entidade, Invalido, Parametro, TipoAgente, TipoAtividade,
    TipoRelacaoOrigem,
};
use eframe::egui::{self, RichText};
use uuid::Uuid;

const VARIANTES: [&str; 3] = ["A · Tela própria", "B · Painel lateral", "C · Edição inline"];
const LICENCAS: [&str; 7] =
    ["CC0 1.0", "CC BY 4.0", "CC BY-SA 4.0", "CC BY-ND 4.0", "CC BY-NC 4.0", "CC BY-NC-SA 4.0", "CC BY-NC-ND 4.0"];

pub struct Estado {
    pub variante: usize,
    doc: Uuid,
    form: Option<Form>,
    aberta: Option<Uuid>,
    filtro: String,
}

impl Default for Estado {
    fn default() -> Self {
        let variante = match std::env::var("VARIANTE").as_deref() {
            Ok("B") => 1,
            Ok("C") => 2,
            _ => 0,
        };
        Self { variante, doc: Uuid::nil(), form: None, aberta: None, filtro: String::new() }
    }
}

impl Estado {
    /// Outro Registro aberto: o formulário do anterior não vale mais.
    pub fn registro(&mut self, id: Uuid) {
        if self.doc != id {
            *self = Self { variante: self.variante, doc: id, ..Self::default() };
        }
    }

    pub fn tela_cheia(&self) -> bool {
        self.variante == 0 && self.form.is_some()
    }
}

/// Cadastro derivado (ADR-0013): Agentes de todos os Registros + nº de Registros que usam cada um.
pub struct Cadastro(Vec<(Agente, usize)>);

pub fn cadastro<'a>(docs: impl Iterator<Item = &'a Documento>) -> Cadastro {
    let mut v: Vec<(Agente, usize)> = Vec::new();
    for d in docs {
        for g in &d.agentes {
            match v.iter_mut().find(|(x, _)| x.id == g.id) {
                Some((_, n)) => *n += 1,
                None => v.push((g.clone(), 1)),
            }
        }
    }
    v.sort_by_key(|(g, _)| g.nome.to_lowercase());
    Cadastro(v)
}

/// Rascunho do formulário (A e B): a cópia de trabalho só muda no "Aplicar".
struct Form {
    nova: bool,
    at: Atividade,
    geradas: Vec<Entidade>,
    agente: Option<Agente>,
    editar_agente: bool,
}

enum Saida {
    Nada,
    Cancelar,
    Aplicar(Documento),
}

impl Form {
    fn nova() -> Self {
        Self { nova: true, at: nova_atividade(), geradas: Vec::new(), agente: None, editar_agente: false }
    }

    fn de(doc: &Documento, id: Uuid) -> Option<Self> {
        let at = doc.atividades.iter().find(|a| a.id == id)?.clone();
        Some(Self {
            nova: false,
            geradas: doc.entidades.iter().filter(|e| e.gerada_por_atividade_id == id).cloned().collect(),
            agente: doc.agentes.iter().find(|g| g.id == at.agente_id).cloned(),
            at,
            editar_agente: false,
        })
    }

    fn titulo(&self) -> String {
        if self.nova { "Nova Atividade".into() } else { format!("Atividade: {} de {}", self.at.tipo.rotulo(), data(&self.at.data_hora)) }
    }

    /// Cópia de trabalho com o formulário aplicado; `validar` decide se o "Aplicar" libera.
    fn aplicado(&self, doc: &Documento) -> Result<Documento, String> {
        let g = self.agente.as_ref().ok_or("Escolha o Agente.")?;
        let mut d = doc.clone();
        let mut at = self.at.clone();
        at.agente_id = g.id;
        match d.atividades.iter().position(|a| a.id == at.id) {
            Some(i) => d.atividades[i] = at,
            None => d.atividades.push(at),
        }
        d.entidades.retain(|e| e.gerada_por_atividade_id != self.at.id);
        d.entidades.extend(self.geradas.iter().cloned());
        match d.agentes.iter().position(|x| x.id == g.id) {
            Some(i) => d.agentes[i] = g.clone(),
            None => d.agentes.push(g.clone()),
        }
        podar(&mut d);
        model::validar(&d).map_err(|e| explicar(&e, &d).1)?;
        Ok(d)
    }
}

fn excluida(doc: &Documento, id: Uuid) -> Result<Documento, String> {
    let mut d = doc.clone();
    d.atividades.retain(|a| a.id != id);
    d.entidades.retain(|e| e.gerada_por_atividade_id != id);
    podar(&mut d);
    model::validar(&d).map_err(|e| explicar(&e, &d).1)?;
    Ok(d)
}

fn nova_atividade() -> Atividade {
    Atividade {
        id: Uuid::now_v7(),
        tipo: TipoAtividade::Criacao,
        agente_id: Uuid::nil(),
        data_hora: model::agora(),
        descricao: None,
        entidades_usadas: Vec::new(),
        local: None,
        instrumento: None,
        processo: None,
        parametros: None,
        ambiente_execucao: None,
    }
}

fn nova_entidade(atividade: Uuid, nome: String) -> Entidade {
    Entidade {
        id: Uuid::now_v7(),
        nome,
        descricao: None,
        formato: None,
        localizacao: None,
        licenca: None,
        gerada_por_atividade_id: atividade,
        tipo_relacao_origem: None,
        revisao_de_id: None,
    }
}

/// Agentes embutidos só existem enquanto alguma Atividade os usa (ADR-0013/0014).
fn podar(d: &mut Documento) {
    let usados: Vec<Uuid> = d.atividades.iter().map(|a| a.agente_id).collect();
    d.agentes.retain(|g| usados.contains(&g.id));
}

/// Erro de `validar` em texto para o usuário + a Atividade onde ele aparece (card na variante C).
fn explicar(e: &Invalido, d: &Documento) -> (Option<Uuid>, String) {
    let at = |id: &Uuid| {
        d.atividades.iter().find(|a| a.id == *id).map_or("Atividade".to_owned(), |a| format!("{} de {}", a.tipo.rotulo(), data(&a.data_hora)))
    };
    let en = |id: &Uuid| d.entidades.iter().find(|e| e.id == *id);
    let nome = |id: &Uuid| en(id).map_or("Entidade".to_owned(), |e| format!("Entidade \"{}\"", e.nome));
    let geradora = |id: &Uuid| en(id).map(|e| e.gerada_por_atividade_id);
    match e {
        Invalido::AgenteInexistente { atividade, .. } => (Some(*atividade), format!("{}: escolha o Agente.", at(atividade))),
        Invalido::Cardinalidade { atividade, regra } => (Some(*atividade), format!("{}: {regra}", at(atividade))),
        Invalido::Ciclo { atividade } => {
            (Some(*atividade), format!("{}: usa uma Entidade gerada por ela mesma ou por uma Atividade seguinte.", at(atividade)))
        }
        Invalido::EntidadeUsadaInexistente { atividade, .. } => {
            (Some(*atividade), format!("{}: usa uma Entidade que não existe mais.", at(atividade)))
        }
        Invalido::Revisao { entidade, regra } => (geradora(entidade), format!("{}: {regra}", nome(entidade))),
        _ => (None, e.to_string()),
    }
}

pub fn mensagem(e: &Invalido, d: &Documento) -> String {
    explicar(e, d).1
}

fn rotulo_entidade(d: &Documento, e: &Entidade) -> String {
    match d.atividades.iter().find(|a| a.id == e.gerada_por_atividade_id) {
        Some(a) => format!("{} ({} {})", e.nome, a.tipo.rotulo(), data(&a.data_hora)),
        None => e.nome.clone(),
    }
}

/// Entidades que a Atividade pode usar: todas, menos as que ela mesma gera.
fn disponiveis(d: &Documento, atividade: Uuid) -> Vec<(Uuid, String)> {
    d.entidades.iter().filter(|e| e.gerada_por_atividade_id != atividade).map(|e| (e.id, rotulo_entidade(d, e))).collect()
}

fn resumo(d: &Documento, at: &Atividade) -> String {
    let agente = d.agentes.iter().find(|g| g.id == at.agente_id).map_or("sem Agente", |g| g.nome.as_str());
    let nomes = |ids: Vec<&str>| if ids.is_empty() { "—".to_owned() } else { ids.join(", ") };
    let usa = at.entidades_usadas.iter().filter_map(|u| d.entidades.iter().find(|e| e.id == *u)).map(|e| e.nome.as_str()).collect();
    let gera = d.entidades.iter().filter(|e| e.gerada_por_atividade_id == at.id).map(|e| e.nome.as_str()).collect();
    format!("{} · {} · {agente}   usa [{}] · gera [{}]", at.tipo.rotulo(), data(&at.data_hora), nomes(usa), nomes(gera))
}

// ---------- campos (iguais nas três variantes: o que muda é onde/como aparecem) ----------

fn rotulo(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).small().weak());
}

fn opt(ui: &mut egui::Ui, t: &str, v: &mut Option<String>, linhas: usize) -> bool {
    rotulo(ui, t);
    let mut s = v.take().unwrap_or_default();
    let ed = if linhas > 1 { egui::TextEdit::multiline(&mut s).desired_rows(linhas) } else { egui::TextEdit::singleline(&mut s) };
    let r = ui.add(ed.desired_width(f32::INFINITY));
    *v = (!s.is_empty()).then_some(s);
    r.changed()
}

/// Autocomplete do cadastro derivado; "Criar" quando o nome ainda não existe. Devolve a escolha nova.
fn escolher_agente(ui: &mut egui::Ui, salt: Uuid, atual: Option<&Agente>, cad: &Cadastro, filtro: &mut String) -> Option<Agente> {
    let mut escolha = None;
    let texto = atual.map_or("Escolha o Agente…".to_owned(), |g| format!("{} · {}", g.nome, g.tipo.rotulo()));
    egui::ComboBox::from_id_salt(("agente", salt))
        .selected_text(texto)
        .width(340.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.add(egui::TextEdit::singleline(filtro).hint_text("Buscar ou criar…")).request_focus();
            let q = filtro.trim().to_lowercase();
            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                for (g, n) in cad.0.iter().filter(|(g, _)| g.nome.to_lowercase().contains(&q)) {
                    let sel = atual.is_some_and(|a| a.id == g.id);
                    if ui.selectable_label(sel, format!("{} · {} · em {n} Registro(s)", g.nome, g.tipo.rotulo())).clicked() {
                        escolha = Some(g.clone());
                    }
                }
            });
            if !q.is_empty()
                && !cad.0.iter().any(|(g, _)| g.nome.to_lowercase() == q)
                && ui.button(format!("{} Criar \"{}\"", icone::NOVO, filtro.trim())).clicked()
            {
                escolha = Some(Agente {
                    id: Uuid::now_v7(),
                    nome: filtro.trim().to_owned(),
                    tipo: TipoAgente::Pessoa,
                    afiliacao: None,
                    identificador_externo: None,
                });
            }
            if escolha.is_some() {
                filtro.clear();
                ui.close();
            }
        });
    escolha
}

fn outros_registros(cad: &Cadastro, doc: &Documento, g: &Agente) -> usize {
    let aqui = doc.agentes.iter().any(|x| x.id == g.id) as usize;
    cad.0.iter().find(|(x, _)| x.id == g.id).map_or(0, |(_, n)| n - aqui)
}

fn campos_agente(ui: &mut egui::Ui, g: &mut Agente, outros: usize) -> bool {
    let mut m = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Nome");
            m |= ui.text_edit_singleline(&mut g.nome).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Tipo");
            for t in [TipoAgente::Pessoa, TipoAgente::Instituicao, TipoAgente::Software] {
                m |= ui.selectable_value(&mut g.tipo, t, t.rotulo()).changed();
            }
        });
        m |= opt(ui, "Afiliação", &mut g.afiliacao, 1);
        m |= opt(ui, "Identificador externo (ORCID, ROR…)", &mut g.identificador_externo, 1);
        if outros > 0 {
            ui.label(
                RichText::new(format!(
                    "{} Ao gravar, pergunta se atualiza este Agente nos outros {outros} Registro(s); os Finalizados ficam como estão.",
                    icone::AVISO
                ))
                .small()
                .color(ui.visuals().warn_fg_color),
            );
        }
    });
    m
}

fn campos_atividade(ui: &mut egui::Ui, at: &mut Atividade, disp: &[(Uuid, String)]) -> bool {
    let mut m = false;
    ui.horizontal(|ui| {
        for t in [TipoAtividade::Criacao, TipoAtividade::Transformacao, TipoAtividade::Analise] {
            m |= ui.selectable_value(&mut at.tipo, t, t.rotulo()).changed();
        }
    });
    rotulo(ui, "Data/hora (ISO 8601)");
    m |= ui.text_edit_singleline(&mut at.data_hora).changed();
    m |= opt(ui, "Descrição do processo (o que foi feito)", &mut at.descricao, 3);
    let regra = if at.tipo == TipoAtividade::Criacao { "opcional" } else { "1 ou mais" };
    rotulo(ui, &format!("Entidades usadas ({regra})"));
    if disp.is_empty() {
        ui.label(RichText::new("nenhuma Entidade no Registro ainda").weak().italics());
    }
    for (id, nome) in disp {
        let mut marcado = at.entidades_usadas.contains(id);
        if ui.checkbox(&mut marcado, nome).changed() {
            m = true;
            if marcado {
                at.entidades_usadas.push(*id);
            } else {
                at.entidades_usadas.retain(|u| u != id);
            }
        }
    }
    if at.tipo == TipoAtividade::Criacao {
        m |= opt(ui, "Local", &mut at.local, 1);
        m |= opt(ui, "Ferramenta ou Software", &mut at.instrumento, 1);
    } else {
        m |= opt(ui, "Processo", &mut at.processo, 4);
        rotulo(ui, "Parâmetros");
        let ps = at.parametros.get_or_insert_with(Vec::new);
        let mut tirar = None;
        for (i, p) in ps.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                m |= ui.add(egui::TextEdit::singleline(&mut p.chave).hint_text("chave").desired_width(160.0)).changed();
                m |= ui.add(egui::TextEdit::singleline(&mut p.valor).hint_text("valor").desired_width(240.0)).changed();
                if ui.small_button(icone::EXCLUIR).clicked() {
                    tirar = Some(i);
                }
            });
        }
        if let Some(i) = tirar {
            ps.remove(i);
            m = true;
        }
        if ui.small_button("+ parâmetro").clicked() {
            ps.push(Parametro { chave: String::new(), valor: String::new() });
            m = true;
        }
        if ps.is_empty() {
            at.parametros = None;
        }
        let amb = at.ambiente_execucao.get_or_insert_with(|| AmbienteExecucao { sistema_operacional: None, pacotes: None });
        m |= opt(ui, "Ambiente de execução: sistema operacional", &mut amb.sistema_operacional, 1);
        if amb.sistema_operacional.is_none() && amb.pacotes.is_none() {
            at.ambiente_execucao = None;
        }
    }
    m
}

/// (mudou, pediu remover)
fn campos_entidade(ui: &mut egui::Ui, e: &mut Entidade, usadas: &[(Uuid, String)]) -> (bool, bool) {
    let (mut m, mut tirar) = (false, false);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label("Nome");
            m |= ui.add(egui::TextEdit::singleline(&mut e.nome).hint_text("obrigatório").desired_width(280.0)).changed();
            if ui.small_button(format!("{} remover", icone::EXCLUIR)).clicked() {
                tirar = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Licença");
            let mut t = e.licenca.take().unwrap_or_default();
            m |= ui.add(egui::TextEdit::singleline(&mut t).hint_text("URL ou texto livre").desired_width(220.0)).changed();
            ui.menu_button("CC…", |ui| {
                for l in LICENCAS {
                    if ui.button(l).clicked() {
                        t = l.to_owned();
                        m = true;
                        ui.close();
                    }
                }
            });
            e.licenca = (!t.is_empty()).then_some(t);
        });
        m |= opt(ui, "Formato", &mut e.formato, 1);
        m |= opt(ui, "Localização", &mut e.localizacao, 1);
        m |= opt(ui, "Descrição do artefato (o que ele contém)", &mut e.descricao, 2);
        if !usadas.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Relação com a origem");
                for (v, r) in [(None, "Nenhuma"), (Some(TipoRelacaoOrigem::Derivacao), "Derivação"), (Some(TipoRelacaoOrigem::Revisao), "Revisão")] {
                    m |= ui.selectable_value(&mut e.tipo_relacao_origem, v, r).changed();
                }
            });
            if e.tipo_relacao_origem == Some(TipoRelacaoOrigem::Revisao) {
                let atual = e.revisao_de_id.and_then(|id| usadas.iter().find(|(u, _)| *u == id)).map_or("escolha…", |(_, n)| n.as_str());
                ui.horizontal(|ui| {
                    ui.label("Revisão de");
                    egui::ComboBox::from_id_salt(("revisao", e.id)).selected_text(atual).show_ui(ui, |ui| {
                        for (id, n) in usadas {
                            m |= ui.selectable_value(&mut e.revisao_de_id, Some(*id), n).changed();
                        }
                    });
                });
            }
        }
    });
    (m, tirar)
}

// ---------- formulário transacional (A e B) ----------

fn formulario(ui: &mut egui::Ui, f: &mut Form, doc: &Documento, cad: &Cadastro, filtro: &mut String, bloqueada: bool) -> Saida {
    let disp = disponiveis(doc, f.at.id);
    let mut saida = Saida::Nada;
    if bloqueada {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "Registro Finalizado: esta Atividade já está gravada e não muda mais. Só dá para adicionar Atividades novas.",
        );
    }
    ui.add_enabled_ui(!bloqueada, |ui| {
        ui.strong("Agente responsável");
        if let Some(g) = escolher_agente(ui, f.at.id, f.agente.as_ref(), cad, filtro) {
            f.agente = Some(g);
        }
        if let Some(g) = &mut f.agente {
            ui.toggle_value(&mut f.editar_agente, format!("{} Dados do Agente", icone::EDITAR));
            if f.editar_agente {
                let outros = outros_registros(cad, doc, g);
                campos_agente(ui, g, outros);
            }
        }
        ui.separator();
        ui.strong("Atividade");
        campos_atividade(ui, &mut f.at, &disp);
        ui.separator();
        let regra = if f.at.tipo == TipoAtividade::Analise { "opcional" } else { "1 ou mais" };
        ui.strong(format!("Entidades geradas ({regra})"));
        let usadas: Vec<(Uuid, String)> = disp.iter().filter(|(id, _)| f.at.entidades_usadas.contains(id)).cloned().collect();
        let mut tirar = None;
        for (i, e) in f.geradas.iter_mut().enumerate() {
            if campos_entidade(ui, e, &usadas).1 {
                tirar = Some(i);
            }
        }
        if let Some(i) = tirar {
            f.geradas.remove(i);
        }
        ui.horizontal(|ui| {
            if ui.button(format!("{} Adicionar Entidade gerada", icone::NOVO)).clicked() {
                f.geradas.push(nova_entidade(f.at.id, String::new()));
            }
            let pendentes: Vec<Uuid> = usadas
                .iter()
                .map(|(u, _)| *u)
                .filter(|u| !f.geradas.iter().any(|e| e.revisao_de_id == Some(*u) && e.tipo_relacao_origem == Some(TipoRelacaoOrigem::Revisao)))
                .collect();
            if pendentes.len() > 1 && ui.button(format!("Revisar Entidades usadas ({})", pendentes.len())).clicked() {
                for u in pendentes {
                    let nome = doc.entidades.iter().find(|e| e.id == u).map(|e| e.nome.clone()).unwrap_or_default();
                    let mut e = nova_entidade(f.at.id, nome);
                    e.tipo_relacao_origem = Some(TipoRelacaoOrigem::Revisao);
                    e.revisao_de_id = Some(u);
                    f.geradas.push(e);
                }
            }
        });
    });
    ui.separator();
    let r = f.aplicado(doc);
    match &r {
        Err(msg) if !bloqueada => drop(ui.colored_label(ui.visuals().error_fg_color, format!("{} {msg}", icone::AVISO))),
        Ok(_) if !bloqueada => drop(ui.label(RichText::new("Pronto para aplicar.").color(egui::Color32::from_rgb(60, 160, 90)))),
        _ => {}
    }
    ui.horizontal(|ui| {
        let rot = if f.nova { "Adicionar Atividade" } else { "Aplicar" };
        if ui.add_enabled(!bloqueada && r.is_ok(), egui::Button::new(rot)).clicked()
            && let Ok(d) = &r
        {
            saida = Saida::Aplicar(d.clone());
        }
        if ui.button(if bloqueada { "Fechar" } else { "Cancelar" }).clicked() {
            saida = Saida::Cancelar;
        }
        if !f.nova && !bloqueada {
            let ex = excluida(doc, f.at.id);
            let b = ui.add_enabled(ex.is_ok(), egui::Button::new(RichText::new(format!("{} Excluir Atividade", icone::EXCLUIR)).color(ui.visuals().error_fg_color)));
            let b = match &ex {
                Err(msg) => b.on_disabled_hover_text(format!("Não dá: {msg}")),
                Ok(_) => b.on_hover_text("Remove a Atividade e as Entidades que ela gera"),
            };
            if b.clicked()
                && let Ok(d) = ex
            {
                saida = Saida::Aplicar(d);
            }
        }
    });
    saida
}

fn concluir(st: &mut Estado, doc: &mut Documento, s: Saida) -> bool {
    match s {
        Saida::Nada => false,
        Saida::Cancelar => {
            st.form = None;
            false
        }
        Saida::Aplicar(d) => {
            *doc = d;
            st.form = None;
            true
        }
    }
}

/// Lista de Atividades das variantes A e B: cada linha abre o formulário.
pub fn lista(ui: &mut egui::Ui, st: &mut Estado, doc: &Documento, gravadas: &[Uuid]) {
    ui.horizontal(|ui| {
        ui.heading(format!("Atividades ({})", doc.atividades.len()));
        if ui.button(format!("{} Adicionar Atividade", icone::NOVO)).clicked() {
            st.form = Some(Form::nova());
            st.filtro.clear();
        }
    });
    egui::Grid::new("prototipo-atividades").striped(true).num_columns(2).show(ui, |ui| {
        for at in &doc.atividades {
            let sel = st.form.as_ref().is_some_and(|f| f.at.id == at.id);
            let (ic, t) = if gravadas.contains(&at.id) { (icone::VISUALIZAR, "Ver") } else { (icone::EDITAR, "Editar") };
            if ui.selectable_label(sel, format!("{ic} {t}")).clicked() {
                st.form = Form::de(doc, at.id);
            }
            ui.label(resumo(doc, at));
            ui.end_row();
        }
    });
}

/// Variante A: o formulário ocupa a tela do Registro, com Voltar.
pub fn tela_form(ui: &mut egui::Ui, st: &mut Estado, doc: &mut Documento, cad: &Cadastro, gravadas: &[Uuid]) -> bool {
    let Some(f) = &mut st.form else { return false };
    let mut s = Saida::Nada;
    if ui.button(format!("{} Registro", icone::VOLTAR)).on_hover_text("Descarta o formulário").clicked() {
        s = Saida::Cancelar;
    }
    ui.heading(f.titulo());
    ui.add_space(6.0);
    if matches!(s, Saida::Nada) {
        s = formulario(ui, f, doc, cad, &mut st.filtro, gravadas.contains(&f.at.id));
    }
    concluir(st, doc, s)
}

/// Variante B: o formulário fica num painel à direita; a tela do Registro continua visível.
pub fn painel(ui: &mut egui::Ui, st: &mut Estado, doc: &mut Documento, cad: &Cadastro, gravadas: &[Uuid]) -> bool {
    let Some(f) = &mut st.form else { return false };
    let mut s = Saida::Nada;
    let filtro = &mut st.filtro;
    egui::Panel::right("prototipo-form").resizable(true).default_size(500.0).min_size(380.0).show(ui, |ui| {
        ui.heading(f.titulo());
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            s = formulario(ui, f, doc, cad, filtro, gravadas.contains(&f.at.id));
        });
    });
    concluir(st, doc, s)
}

/// Variante C: sem formulário. Cards expansíveis editam direto a cópia de trabalho; o erro de `validar` vai para o card.
pub fn inline(ui: &mut egui::Ui, st: &mut Estado, doc: &mut Documento, cad: &Cadastro, gravadas: &[Uuid]) -> bool {
    let mut m = false;
    let erro = model::validar(doc).err().map(|e| explicar(&e, doc));
    ui.horizontal(|ui| {
        ui.heading(format!("Atividades ({})", doc.atividades.len()));
        if ui.button(format!("{} Atividade", icone::NOVO)).clicked() {
            let at = nova_atividade();
            st.aberta = Some(at.id);
            doc.atividades.push(at);
            m = true;
        }
    });
    ui.label(RichText::new("Edição direta: cada mudança vai para a cópia de trabalho. Com erro, Gravar fica desligado.").weak().small());
    if let Some((None, msg)) = &erro {
        ui.colored_label(ui.visuals().error_fg_color, format!("{} {msg}", icone::AVISO));
    }
    let ids: Vec<Uuid> = doc.atividades.iter().map(|a| a.id).collect();
    let mut excluir = None;
    for id in ids {
        let aberta = st.aberta == Some(id);
        let bloq = gravadas.contains(&id);
        let i = doc.atividades.iter().position(|a| a.id == id).expect("id da lista");
        let titulo = resumo(doc, &doc.atividades[i]);
        let erro_aqui = erro.as_ref().filter(|(alvo, _)| *alvo == Some(id)).map(|(_, msg)| msg.clone());
        let mut fr = egui::Frame::group(ui.style());
        if erro_aqui.is_some() {
            fr = fr.stroke(egui::Stroke::new(1.5, ui.visuals().error_fg_color));
        }
        fr.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if ui.selectable_label(aberta, if aberta { "Fechar" } else { "Abrir" }).clicked() {
                    st.aberta = if aberta { None } else { Some(id) };
                }
                ui.label(RichText::new(titulo).strong());
                if bloq {
                    ui.label(RichText::new("(gravada no Finalizado: imutável)").weak().small());
                }
            });
            if let Some(msg) = &erro_aqui {
                ui.colored_label(ui.visuals().error_fg_color, format!("{} {msg}", icone::AVISO));
            }
            if !aberta {
                return;
            }
            ui.add_enabled_ui(!bloq, |ui| {
                let aid = doc.atividades[i].agente_id;
                let atual = doc.agentes.iter().find(|g| g.id == aid).cloned();
                if let Some(g) = escolher_agente(ui, id, atual.as_ref(), cad, &mut st.filtro) {
                    doc.atividades[i].agente_id = g.id;
                    if !doc.agentes.iter().any(|x| x.id == g.id) {
                        doc.agentes.push(g);
                    }
                    podar(doc);
                    m = true;
                }
                let aid = doc.atividades[i].agente_id;
                if let Some(g) = atual.filter(|g| g.id == aid) {
                    let outros = outros_registros(cad, doc, &g);
                    egui::CollapsingHeader::new(format!("{} Dados do Agente", icone::EDITAR)).id_salt(("ag", id)).show(ui, |ui| {
                        if let Some(g) = doc.agentes.iter_mut().find(|x| x.id == aid) {
                            m |= campos_agente(ui, g, outros);
                        }
                    });
                }
                let disp = disponiveis(doc, id);
                m |= campos_atividade(ui, &mut doc.atividades[i], &disp);
                let usadas: Vec<(Uuid, String)> = disp.iter().filter(|(u, _)| doc.atividades[i].entidades_usadas.contains(u)).cloned().collect();
                ui.strong("Entidades geradas");
                let mut tirar = None;
                for e in doc.entidades.iter_mut().filter(|e| e.gerada_por_atividade_id == id) {
                    let (c, t) = campos_entidade(ui, e, &usadas);
                    m |= c;
                    if t {
                        tirar = Some(e.id);
                    }
                }
                if let Some(t) = tirar {
                    doc.entidades.retain(|e| e.id != t);
                    m = true;
                }
                ui.horizontal(|ui| {
                    if ui.button(format!("{} Entidade gerada", icone::NOVO)).clicked() {
                        doc.entidades.push(nova_entidade(id, String::new()));
                        m = true;
                    }
                    if ui.button(RichText::new(format!("{} Excluir Atividade", icone::EXCLUIR)).color(ui.visuals().error_fg_color)).clicked() {
                        excluir = Some(id);
                    }
                });
            });
        });
    }
    if let Some(id) = excluir {
        doc.atividades.retain(|a| a.id != id);
        doc.entidades.retain(|e| e.gerada_por_atividade_id != id);
        podar(doc);
        m = true;
    }
    m
}

/// Barra flutuante do protótipo: ◀ variante ▶ (←/→ quando nenhum campo tem foco).
pub fn barra(ctx: &egui::Context, st: &mut Estado) {
    let n = VARIANTES.len();
    if ctx.memory(|m| m.focused().is_none()) {
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            st.variante = (st.variante + n - 1) % n;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            st.variante = (st.variante + 1) % n;
        }
    }
    egui::Area::new(egui::Id::unique("prototipo-barra"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).fill(egui::Color32::from_rgb(40, 40, 48)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let txt = |s: &str| RichText::new(s).color(egui::Color32::WHITE).strong();
                    if ui.button("<").clicked() {
                        st.variante = (st.variante + n - 1) % n;
                    }
                    ui.label(txt(&format!("PROTÓTIPO  {}", VARIANTES[st.variante])));
                    if ui.button(">").clicked() {
                        st.variante = (st.variante + 1) % n;
                    }
                });
            });
        });
}
