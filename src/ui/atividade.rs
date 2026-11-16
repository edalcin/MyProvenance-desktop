//! Formulário de Atividade no painel lateral (decisão #13, variante B). Transacional: o formulário tem
//! rascunho próprio; a cópia de trabalho só muda no "Aplicar", que só libera com `validar` ok no documento aplicado.
//! Registro Finalizado: Atividades já gravadas abrem só para ver; Atividades novas continuam livres.
use super::{Edicao, data, icone};
use crate::{
    model::{
        self, Agente, AmbienteExecucao, Atividade, Documento, Entidade, Invalido, Pacote, Parametro, Status, TipoAgente,
        TipoAtividade, TipoRelacaoOrigem,
    },
    store::Arquivo,
};
use eframe::egui::{self, RichText};
use uuid::Uuid;

const OCUPADO: &str = "Aplique ou cancele o formulário aberto";

#[derive(Clone, PartialEq)]
struct Campos {
    at: Atividade,
    geradas: Vec<Entidade>,
    agente: Option<Agente>,
    /// Como o usuário vê: `dd/mm/aaaa hh:mm` no fuso do sistema. Sem mudança, `at.data_hora` fica como está.
    data: String,
}

pub(super) struct Form {
    nova: bool,
    c: Campos,
    orig: Campos,
    editar_agente: bool,
    busca_agente: String,
    busca_usadas: String,
}

enum Saida {
    Fechar,
    Aplicar(Documento),
}

impl Form {
    fn com(nova: bool, c: Campos) -> Self {
        Self { nova, orig: c.clone(), c, editar_agente: false, busca_agente: String::new(), busca_usadas: String::new() }
    }

    fn nova() -> Self {
        let agora = data(&model::agora());
        let at = Atividade {
            id: Uuid::now_v7(),
            tipo: TipoAtividade::Criacao,
            agente_id: Uuid::nil(),
            data_hora: iso_local(&agora).unwrap_or_else(model::agora),
            descricao: None,
            entidades_usadas: Vec::new(),
            local: None,
            instrumento: None,
            processo: None,
            parametros: None,
            ambiente_execucao: None,
        };
        Self::com(true, Campos { at, geradas: Vec::new(), agente: None, data: agora })
    }

    fn de(doc: &Documento, id: Uuid) -> Option<Self> {
        let at = doc.atividades.iter().find(|a| a.id == id)?.clone();
        let c = Campos {
            geradas: doc.entidades.iter().filter(|e| e.gerada_por_atividade_id == id).cloned().collect(),
            agente: doc.agentes.iter().find(|g| g.id == at.agente_id).cloned(),
            data: data(&at.data_hora),
            at,
        };
        Some(Self::com(false, c))
    }

    /// Há mudança não aplicada: sair do Registro descarta o formulário.
    pub(super) fn alterado(&self) -> bool {
        self.c != self.orig
    }

    fn titulo(&self) -> String {
        if self.nova { "Nova Atividade".into() } else { format!("Atividade: {} de {}", self.orig.at.tipo.rotulo(), self.orig.data) }
    }

    /// Cópia de trabalho com o formulário aplicado, se `validar` passa.
    fn aplicado(&self, doc: &Documento) -> Result<Documento, String> {
        let g = self.c.agente.as_ref().ok_or("Escolha o Agente.")?;
        let mut at = self.c.at.clone();
        at.agente_id = g.id;
        if self.c.data != self.orig.data {
            at.data_hora = iso_local(&self.c.data).ok_or("Data/hora inválida: use dd/mm/aaaa hh:mm.")?;
        }
        let mut geradas = self.c.geradas.clone();
        for e in &mut geradas {
            // Relação só existe com entrada; revisaoDeId só vale para Revisão.
            if at.entidades_usadas.is_empty() {
                e.tipo_relacao_origem = None;
            }
            if e.tipo_relacao_origem != Some(TipoRelacaoOrigem::Revisao) {
                e.revisao_de_id = None;
            }
        }
        let mut d = doc.clone();
        match d.atividades.iter().position(|a| a.id == at.id) {
            Some(i) => d.atividades[i] = at,
            None => d.atividades.push(at),
        }
        d.entidades.retain(|e| e.gerada_por_atividade_id != self.c.at.id);
        d.entidades.extend(geradas);
        match d.agentes.iter().position(|x| x.id == g.id) {
            Some(i) => d.agentes[i] = g.clone(),
            None => d.agentes.push(g.clone()),
        }
        podar(&mut d);
        model::validar(&d).map_err(|e| explicar(&e, &d))?;
        Ok(d)
    }
}

/// `dd/mm/aaaa hh:mm` no fuso do sistema → ISO 8601 com o offset local; o fuso fica gravado no arquivo.
fn iso_local(texto: &str) -> Option<String> {
    let dt = jiff::civil::DateTime::strptime("%d/%m/%Y %H:%M", texto.trim()).ok()?;
    let z = dt.to_zoned(jiff::tz::TimeZone::system()).ok()?;
    Some(z.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string())
}

fn excluida(doc: &Documento, id: Uuid) -> Result<Documento, String> {
    let mut d = doc.clone();
    d.atividades.retain(|a| a.id != id);
    d.entidades.retain(|e| e.gerada_por_atividade_id != id);
    podar(&mut d);
    model::validar(&d).map_err(|e| explicar(&e, &d))?;
    Ok(d)
}

/// Agentes embutidos só existem enquanto alguma Atividade os usa (ADR-0013/0014).
fn podar(d: &mut Documento) {
    let usados: Vec<Uuid> = d.atividades.iter().map(|a| a.agente_id).collect();
    d.agentes.retain(|g| usados.contains(&g.id));
}

/// Erro de `validar` em texto para o usuário, com o nome da Atividade/Entidade no lugar do id.
pub(super) fn explicar(e: &Invalido, d: &Documento) -> String {
    let at = |a: &Atividade| format!("{} de {}", a.tipo.rotulo(), data(&a.data_hora));
    let at_id = |id: &Uuid| d.atividades.iter().find(|a| a.id == *id).map_or("Atividade".to_owned(), at);
    let en = |e: &Entidade| if e.nome.trim().is_empty() { "Entidade sem nome".to_owned() } else { format!("Entidade \"{}\"", e.nome) };
    let en_id = |id: &Uuid| d.entidades.iter().find(|e| e.id == *id).map_or("Entidade".to_owned(), en);
    // Caminho do garde (`entidades[19].nome`) → `Entidade "x" · nome`.
    let campo = |caminho: String| {
        let (raiz, resto) = caminho.split_once('.').unwrap_or((&caminho, ""));
        let (lista, i) = raiz.strip_suffix(']').and_then(|r| r.split_once('[')).unwrap_or(("", ""));
        let quem = i.parse::<usize>().ok().and_then(|i| match lista {
            "atividades" => d.atividades.get(i).map(at),
            "entidades" => d.entidades.get(i).map(en),
            "agentes" => d.agentes.get(i).map(|g| format!("Agente \"{}\"", g.nome)),
            _ => None,
        });
        quem.map_or_else(|| caminho.clone(), |q| format!("{q} · {resto}"))
    };
    match e {
        Invalido::Limites(r) => r.iter().map(|(p, e)| format!("{}: {}", campo(p.to_string()), e.message())).collect::<Vec<_>>().join("; "),
        Invalido::AgenteInexistente { atividade, .. } => format!("{}: escolha o Agente.", at_id(atividade)),
        Invalido::Cardinalidade { atividade, regra } => format!("{}: {regra}", at_id(atividade)),
        Invalido::Ciclo { atividade } => format!("{}: usa uma Entidade gerada por ela mesma ou por uma Atividade seguinte.", at_id(atividade)),
        Invalido::EntidadeUsadaInexistente { atividade, .. } => format!("{} usa uma Entidade gerada por esta Atividade.", at_id(atividade)),
        Invalido::Revisao { entidade, regra } => format!("{}: {regra}", en_id(entidade)),
        _ => e.to_string(),
    }
}

/// Cadastro do autocomplete: o derivado dos outros Registros (ADR-0013) e a cópia de trabalho, que vence.
fn cadastro(arquivos: &[Arquivo], doc: &Documento) -> Vec<(Agente, usize)> {
    let mut cad = model::cadastro(arquivos.iter().map(|a| &a.doc).filter(|d| d.registro.id != doc.registro.id));
    for g in &doc.agentes {
        match cad.iter_mut().find(|(x, _)| x.id == g.id) {
            Some((x, n)) => (*x, *n) = (g.clone(), *n + 1),
            None => cad.push((g.clone(), 1)),
        }
    }
    cad.sort_by_cached_key(|(g, _)| g.nome.to_lowercase());
    cad
}

fn rotulo_entidade(d: &Documento, e: &Entidade) -> String {
    match d.atividades.iter().find(|a| a.id == e.gerada_por_atividade_id) {
        Some(a) => format!("{} ({} {})", e.nome, a.tipo.rotulo(), data(&a.data_hora)),
        None => e.nome.clone(),
    }
}

fn resumo(d: &Documento, at: &Atividade) -> String {
    let agente = d.agentes.iter().find(|g| g.id == at.agente_id).map_or("sem Agente", |g| g.nome.as_str());
    let nomes = |v: Vec<&str>| if v.is_empty() { "—".to_owned() } else { v.join(", ") };
    let usa = at.entidades_usadas.iter().filter_map(|u| d.entidades.iter().find(|e| e.id == *u)).map(|e| e.nome.as_str()).collect();
    let gera = d.entidades.iter().filter(|e| e.gerada_por_atividade_id == at.id).map(|e| e.nome.as_str()).collect();
    format!("{} · {} · {agente}   usa [{}] · gera [{}]", at.tipo.rotulo(), data(&at.data_hora), nomes(usa), nomes(gera))
}

/// Lista de Atividades da tela do Registro; cada linha abre o formulário no painel.
/// `fixo` = versão gravada, só quando o Registro está Finalizado.
pub(super) fn lista(ui: &mut egui::Ui, e: &mut Edicao, fixo: Option<&Documento>) {
    let livre = !e.form.as_ref().is_some_and(Form::alterado);
    let mut abrir = None;
    ui.horizontal(|ui| {
        ui.heading(format!("Atividades ({})", e.doc.atividades.len()));
        let b = ui.add_enabled(livre, egui::Button::new(format!("{} Adicionar Atividade", icone::NOVO)));
        if b.on_disabled_hover_text(OCUPADO).clicked() {
            e.form = Some(Form::nova());
        }
    });
    if e.doc.atividades.is_empty() {
        ui.label(RichText::new("Nenhuma Atividade ainda.").weak().italics());
    }
    egui::Grid::new("atividades").striped(true).num_columns(2).show(ui, |ui| {
        for at in &e.doc.atividades {
            let aberta = e.form.as_ref().is_some_and(|f| f.c.at.id == at.id);
            let ver = fixo.is_some_and(|d| d.atividades.iter().any(|x| x.id == at.id));
            let (ic, t) = if ver { (icone::VISUALIZAR, "Ver") } else { (icone::EDITAR, "Editar") };
            if ui.add_enabled(livre, egui::Button::selectable(aberta, format!("{ic} {t}"))).on_disabled_hover_text(OCUPADO).clicked() {
                abrir = Some(at.id);
            }
            ui.label(resumo(&e.doc, at));
            ui.end_row();
        }
    });
    if let Some(id) = abrir {
        e.form = Form::de(&e.doc, id);
    }
}

/// Painel à direita com o formulário aberto; a tela do Registro continua visível.
pub(super) fn painel(ui: &mut egui::Ui, e: &mut Edicao, arquivos: &[Arquivo], fixo: Option<&Documento>) {
    let Some(f) = &mut e.form else { return };
    let mut saida = None;
    egui::Panel::right("form-atividade").resizable(true).default_size(520.0).min_size(380.0).show(ui, |ui| {
        ui.heading(f.titulo());
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| saida = formulario(ui, f, &e.doc, arquivos, fixo));
    });
    match saida {
        Some(Saida::Aplicar(d)) => (e.doc, e.sujo, e.form) = (d, true, None),
        Some(Saida::Fechar) => e.form = None,
        None => {}
    }
}

fn formulario(ui: &mut egui::Ui, f: &mut Form, doc: &Documento, arquivos: &[Arquivo], fixo: Option<&Documento>) -> Option<Saida> {
    let bloqueada = fixo.is_some_and(|d| d.atividades.iter().any(|a| a.id == f.c.at.id));
    if bloqueada {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "Registro Finalizado: esta Atividade já está gravada e não muda mais. Só dá para adicionar Atividades novas.",
        );
    }
    let proibidas = model::descendentes(doc, f.c.at.id);
    let disp: Vec<(Uuid, String)> =
        doc.entidades.iter().filter(|e| !proibidas.contains(&e.id)).map(|e| (e.id, rotulo_entidade(doc, e))).collect();
    ui.add_enabled_ui(!bloqueada, |ui| {
        ui.strong("Agente responsável");
        let cad = cadastro(arquivos, doc);
        if let Some(g) = escolher_agente(ui, f.c.agente.as_ref(), &cad, &mut f.busca_agente) {
            // Agente recém-criado: abre os dados para completar.
            f.editar_agente = !cad.iter().any(|(x, _)| x.id == g.id);
            f.c.agente = Some(g);
        }
        if let Some(g) = &mut f.c.agente {
            if fixo.is_some_and(|d| d.agentes.iter().any(|x| x.id == g.id)) {
                ui.label(RichText::new("Dados do Agente fixos: ele já está no histórico deste Registro Finalizado.").weak().small());
            } else {
                ui.toggle_value(&mut f.editar_agente, format!("{} Dados do Agente", icone::EDITAR));
                if f.editar_agente {
                    let id = doc.registro.id;
                    let em_rascunho = arquivos
                        .iter()
                        .filter(|a| a.doc.registro.id != id && a.doc.registro.status == Status::Rascunho)
                        .filter(|a| a.doc.agentes.iter().any(|x| x.id == g.id))
                        .count();
                    campos_agente(ui, g, em_rascunho);
                }
            }
        }
        ui.separator();
        ui.strong("Atividade");
        campos_atividade(ui, &mut f.c, &disp, &mut f.busca_usadas);
        ui.separator();
        geradas(ui, &mut f.c, &disp, doc);
    });
    ui.separator();
    let r = f.aplicado(doc);
    if !bloqueada {
        match &r {
            Err(msg) => drop(ui.colored_label(ui.visuals().error_fg_color, format!("{} {msg}", icone::AVISO))),
            Ok(_) => drop(ui.label(RichText::new("Pronto para aplicar.").color(egui::Color32::from_rgb(60, 160, 90)))),
        }
    }
    let mut saida = None;
    ui.horizontal(|ui| {
        if !bloqueada {
            let rot = if f.nova { "Adicionar Atividade" } else { "Aplicar" };
            if ui.add_enabled(r.is_ok(), egui::Button::new(rot)).clicked()
                && let Ok(d) = r
            {
                saida = Some(Saida::Aplicar(d));
            }
        }
        if ui.button(if bloqueada { "Fechar" } else { "Cancelar" }).clicked() {
            saida = Some(Saida::Fechar);
        }
        if !f.nova && !bloqueada {
            let ex = excluida(doc, f.c.at.id);
            let rot = RichText::new(format!("{} Excluir Atividade", icone::EXCLUIR)).color(ui.visuals().error_fg_color);
            let b = ui.add_enabled(ex.is_ok(), egui::Button::new(rot));
            let b = match &ex {
                Err(msg) => b.on_disabled_hover_text(format!("Não dá: {msg}")),
                Ok(_) => b.on_hover_text("Remove a Atividade e as Entidades que ela gera"),
            };
            if b.clicked()
                && let Ok(d) = ex
            {
                saida = Some(Saida::Aplicar(d));
            }
        }
    });
    saida
}

fn rotulo(ui: &mut egui::Ui, t: &str) {
    ui.label(RichText::new(t).small().weak());
}

fn opt(ui: &mut egui::Ui, t: &str, v: &mut Option<String>, linhas: usize) {
    rotulo(ui, t);
    let mut s = v.take().unwrap_or_default();
    let ed = if linhas > 1 { egui::TextEdit::multiline(&mut s).desired_rows(linhas) } else { egui::TextEdit::singleline(&mut s) };
    ui.add(ed.desired_width(f32::INFINITY));
    *v = (!s.is_empty()).then_some(s);
}

/// Autocomplete do cadastro derivado; "Criar" quando o nome ainda não existe. Devolve a escolha nova.
fn escolher_agente(ui: &mut egui::Ui, atual: Option<&Agente>, cad: &[(Agente, usize)], busca: &mut String) -> Option<Agente> {
    let mut escolha = None;
    let texto = atual.map_or("Escolha o Agente…".to_owned(), |g| format!("{} · {}", g.nome, g.tipo.rotulo()));
    egui::ComboBox::from_id_salt("agente")
        .selected_text(texto)
        .width(340.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.add(egui::TextEdit::singleline(busca).hint_text("Buscar ou criar…")).request_focus();
            let q = busca.trim().to_lowercase();
            egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                for (g, n) in cad.iter().filter(|(g, _)| g.nome.to_lowercase().contains(&q)) {
                    let sel = atual.is_some_and(|a| a.id == g.id);
                    if ui.selectable_label(sel, format!("{} · {} · em {n} Registro(s)", g.nome, g.tipo.rotulo())).clicked() {
                        escolha = Some(g.clone());
                    }
                }
            });
            if !q.is_empty()
                && !cad.iter().any(|(g, _)| g.nome.trim().to_lowercase() == q)
                && ui.button(format!("{} Criar \"{}\"", icone::NOVO, busca.trim())).clicked()
            {
                escolha = Some(Agente {
                    id: Uuid::now_v7(),
                    nome: busca.trim().to_owned(),
                    tipo: TipoAgente::Pessoa,
                    afiliacao: None,
                    identificador_externo: None,
                });
            }
            if escolha.is_some() {
                busca.clear();
                ui.close();
            }
        });
    escolha
}

fn campos_agente(ui: &mut egui::Ui, g: &mut Agente, outros: usize) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Nome");
            ui.text_edit_singleline(&mut g.nome);
        });
        ui.horizontal(|ui| {
            ui.label("Tipo");
            for t in [TipoAgente::Pessoa, TipoAgente::Instituicao, TipoAgente::Software] {
                ui.selectable_value(&mut g.tipo, t, t.rotulo());
            }
        });
        opt(ui, "Afiliação", &mut g.afiliacao, 1);
        opt(ui, "Identificador externo (ORCID, ROR…)", &mut g.identificador_externo, 1);
        if outros > 0 {
            let aviso = format!(
                "{} Ao gravar, pergunta se atualiza este Agente nos outros {outros} Registro(s) em Rascunho; os Finalizados ficam como estão.",
                icone::AVISO
            );
            ui.label(RichText::new(aviso).small().color(ui.visuals().warn_fg_color));
        }
    });
}

fn campos_atividade(ui: &mut egui::Ui, c: &mut Campos, disp: &[(Uuid, String)], busca: &mut String) {
    let at = &mut c.at;
    ui.horizontal(|ui| {
        for t in [TipoAtividade::Criacao, TipoAtividade::Transformacao, TipoAtividade::Analise] {
            ui.selectable_value(&mut at.tipo, t, t.rotulo());
        }
    });
    rotulo(ui, "Data/hora (fuso do sistema)");
    ui.add(egui::TextEdit::singleline(&mut c.data).hint_text("dd/mm/aaaa hh:mm").desired_width(160.0));
    opt(ui, "Descrição do processo (o que foi feito)", &mut at.descricao, 3);

    let regra = if at.tipo == TipoAtividade::Criacao { "opcional" } else { "1 ou mais" };
    rotulo(ui, &format!("Entidades usadas ({regra}) · {} marcada(s)", at.entidades_usadas.len()));
    if disp.is_empty() {
        ui.label(RichText::new("nenhuma Entidade disponível").weak().italics());
    } else {
        ui.add(egui::TextEdit::singleline(busca).hint_text(format!("{} Buscar Entidade…", icone::BUSCAR)).desired_width(f32::INFINITY));
        let q = busca.trim().to_lowercase();
        egui::ScrollArea::vertical().id_salt("usadas").max_height(220.0).show(ui, |ui| {
            for (id, nome) in disp {
                let mut marcado = at.entidades_usadas.contains(id);
                // Marcadas aparecem sempre, mesmo fora da busca.
                if !marcado && !nome.to_lowercase().contains(&q) {
                    continue;
                }
                if ui.checkbox(&mut marcado, nome).changed() {
                    if marcado {
                        at.entidades_usadas.push(*id);
                    } else {
                        at.entidades_usadas.retain(|u| u != id);
                    }
                }
            }
        });
    }

    if at.tipo == TipoAtividade::Criacao {
        opt(ui, "Local", &mut at.local, 1);
        opt(ui, "Ferramenta ou Software", &mut at.instrumento, 1);
    } else {
        opt(ui, "Processo", &mut at.processo, 4);
        let novo = || Parametro { chave: String::new(), valor: String::new() };
        pares(ui, "Parâmetros", &mut at.parametros, ["chave", "valor"], novo, |p| [&mut p.chave, &mut p.valor]);
        let amb = at.ambiente_execucao.get_or_insert_with(|| AmbienteExecucao { sistema_operacional: None, pacotes: None });
        opt(ui, "Ambiente de execução: sistema operacional", &mut amb.sistema_operacional, 1);
        let novo = || Pacote { nome: String::new(), versao: String::new() };
        pares(ui, "Ambiente de execução: pacotes", &mut amb.pacotes, ["nome", "versão"], novo, |p| [&mut p.nome, &mut p.versao]);
        if amb.sistema_operacional.is_none() && amb.pacotes.is_none() {
            at.ambiente_execucao = None;
        }
    }
}

/// Lista de pares de texto (Parâmetros, Pacotes); `None` quando vazia.
fn pares<T>(ui: &mut egui::Ui, titulo: &str, v: &mut Option<Vec<T>>, dicas: [&str; 2], novo: fn() -> T, campos: fn(&mut T) -> [&mut String; 2]) {
    rotulo(ui, titulo);
    let lista = v.get_or_insert_with(Vec::new);
    let mut tirar = None;
    for (i, p) in lista.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let [a, b] = campos(p);
            ui.add(egui::TextEdit::singleline(a).hint_text(dicas[0]).desired_width(160.0));
            ui.add(egui::TextEdit::singleline(b).hint_text(dicas[1]).desired_width(220.0));
            if ui.small_button(icone::EXCLUIR).on_hover_text("Remover").clicked() {
                tirar = Some(i);
            }
        });
    }
    if let Some(i) = tirar {
        lista.remove(i);
    }
    if ui.small_button(format!("{} {}", icone::NOVO, dicas[0])).clicked() {
        lista.push(novo());
    }
    if lista.is_empty() {
        *v = None;
    }
}

/// Entidades geradas, editadas dentro da Atividade (assim a Atividade nunca fica sem a saída que a cardinalidade pede).
fn geradas(ui: &mut egui::Ui, c: &mut Campos, disp: &[(Uuid, String)], doc: &Documento) {
    let regra = if c.at.tipo == TipoAtividade::Analise { "opcional" } else { "1 ou mais" };
    ui.strong(format!("Entidades geradas ({regra})"));
    let usadas: Vec<(Uuid, String)> = disp.iter().filter(|(id, _)| c.at.entidades_usadas.contains(id)).cloned().collect();
    let mut tirar = None;
    for (i, e) in c.geradas.iter_mut().enumerate() {
        if campos_entidade(ui, e, &usadas) {
            tirar = Some(i);
        }
    }
    if let Some(i) = tirar {
        c.geradas.remove(i);
    }
    let nova = |at: Uuid, nome: String| Entidade {
        id: Uuid::now_v7(),
        nome,
        descricao: None,
        formato: None,
        localizacao: None,
        licenca: None,
        gerada_por_atividade_id: at,
        tipo_relacao_origem: None,
        revisao_de_id: None,
    };
    ui.horizontal(|ui| {
        if ui.button(format!("{} Adicionar Entidade gerada", icone::NOVO)).clicked() {
            c.geradas.push(nova(c.at.id, String::new()));
        }
        let revisada = |u: Uuid| c.geradas.iter().any(|e| e.revisao_de_id == Some(u) && e.tipo_relacao_origem == Some(TipoRelacaoOrigem::Revisao));
        let pendentes: Vec<Uuid> = usadas.iter().map(|(u, _)| *u).filter(|u| !revisada(*u)).collect();
        if pendentes.len() > 1
            && ui.button(format!("Revisar Entidades usadas ({})", pendentes.len())).on_hover_text("Uma nova versão de cada uma").clicked()
        {
            for u in pendentes {
                let nome = doc.entidades.iter().find(|e| e.id == u).map(|e| e.nome.clone()).unwrap_or_default();
                let mut e = nova(c.at.id, nome);
                e.tipo_relacao_origem = Some(TipoRelacaoOrigem::Revisao);
                e.revisao_de_id = Some(u);
                c.geradas.push(e);
            }
        }
    });
}

/// `true` = pediu remover.
fn campos_entidade(ui: &mut egui::Ui, e: &mut Entidade, usadas: &[(Uuid, String)]) -> bool {
    let mut tirar = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label("Nome");
            ui.add(egui::TextEdit::singleline(&mut e.nome).hint_text("obrigatório").desired_width(280.0));
            tirar = ui.small_button(format!("{} remover", icone::EXCLUIR)).clicked();
        });
        opt(ui, "Formato", &mut e.formato, 1);
        opt(ui, "Licença", &mut e.licenca, 1);
        opt(ui, "Localização", &mut e.localizacao, 1);
        opt(ui, "Descrição do artefato (o que ele contém)", &mut e.descricao, 2);
        if usadas.is_empty() {
            return;
        }
        ui.horizontal(|ui| {
            ui.label("Relação com a origem");
            for (v, r) in [(None, "Nenhuma"), (Some(TipoRelacaoOrigem::Derivacao), "Derivação"), (Some(TipoRelacaoOrigem::Revisao), "Revisão")] {
                ui.selectable_value(&mut e.tipo_relacao_origem, v, r);
            }
        });
        if e.tipo_relacao_origem == Some(TipoRelacaoOrigem::Revisao) {
            let atual = e.revisao_de_id.and_then(|id| usadas.iter().find(|(u, _)| *u == id)).map_or("escolha…", |(_, n)| n.as_str());
            ui.horizontal(|ui| {
                ui.label("Revisão de");
                egui::ComboBox::from_id_salt(("revisao", e.id)).selected_text(atual).show_ui(ui, |ui| {
                    for (id, n) in usadas {
                        ui.selectable_value(&mut e.revisao_de_id, Some(*id), n);
                    }
                });
            });
        }
    });
    tirar
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn data_local_ida_e_volta() {
        let iso = iso_local("16/11/2026 10:30").unwrap();
        assert!(iso.parse::<jiff::Timestamp>().is_ok(), "{iso}");
        assert_eq!(data(&iso), "16/11/2026 10:30");
        assert!(iso_local("2026-11-16 10:30").is_none());
    }

    #[test]
    fn limite_aponta_a_entidade_pelo_nome() {
        let mut d: Documento = serde_json::from_value(model::exemplo()).unwrap();
        d.entidades[1].nome = " ".into();
        d.atividades[1].processo = Some("x".repeat(20001));
        let msg = explicar(&model::validar(&d).unwrap_err(), &d);
        assert!(msg.contains("Entidade sem nome · nome: "), "{msg}");
        assert!(msg.contains("Transformação de 02/04/2025 "), "{msg}");
    }
}
