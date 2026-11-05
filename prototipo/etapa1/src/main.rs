//! PROTÓTIPO DESCARTÁVEL (#8): três layouts radicalmente diferentes de lista + registro em egui,
//! trocados pela barra flutuante embaixo (◀ ▶ ou setas do teclado sem foco em campo de texto).
//! Uso: cargo run --release -- [--variante A|B|C] [--pasta <dir>]
mod amostra;
mod model;
mod store;

use eframe::egui::{self, Color32, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use model::Status;
use std::path::PathBuf;
use store::{Arquivo, Conflito};

// Ubuntu-Light (fonte padrão do egui) não tem setas "→"/"←": usar ➡/⬅ do NotoEmoji.
const VARIANTES: [&str; 3] = ["A (Mestre-detalhe)", "B (Telas: tabela ➡ registro)", "C (Cards + janelas)"];

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let (mut variante, mut pasta, mut abrir) = (0, None, false);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--variante" => variante = args.next().and_then(|v| "ABC".find(&v.to_uppercase())).unwrap_or(0),
            "--pasta" => pasta = args.next().map(PathBuf::from),
            "--abrir" => abrir = true,
            _ => {}
        }
    }
    // ponytail: pasta PROTÓTIPO em %TEMP%, nunca Documentos\MyProvenance; troca de pasta nas Configurações fica para o app real.
    let pasta = pasta.unwrap_or_else(|| std::env::temp_dir().join("MyProvenance-PROTOTIPO-apagar"));
    std::fs::create_dir_all(&pasta).expect("pasta");
    if std::fs::read_dir(&pasta).map(|mut d| d.next().is_none()).unwrap_or(true) {
        amostra::semear(&pasta);
    }
    let (arquivos, avisos) = store::carregar(&pasta);
    let mut app = App { pasta, arquivos, avisos, variante, ..Default::default() };
    if let Some(i) = abrir.then(|| app.visiveis().first().copied()).flatten() {
        app.executar(Acao::Abrir(app.arquivos[i].doc.registro.id.clone()));
    }
    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("MyProvenance — PROTÓTIPO").with_inner_size([1200.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native("MyProvenance-prototipo", opcoes, Box::new(|_| Ok(Box::new(app))))
}

#[derive(Default)]
struct App {
    pasta: PathBuf,
    arquivos: Vec<Arquivo>,
    avisos: Vec<String>,
    variante: usize,
    /// A e B: registro aberto (por `registro.id`, sobrevive ao Recarregar).
    sel: Option<String>,
    /// C: janelas abertas.
    abertos: Vec<String>,
    busca: String,
    /// B: aba da descrição (false = Editar, true = Visualizar).
    b_preview: bool,
    md: CommonMarkCache,
    modal: Option<Modal>,
    ver_avisos: bool,
    msg: String,
}

enum Modal {
    Novo(String),
    Finalizar(String),
    Excluir(String),
    Conflito(String, Conflito),
    Recarregar,
}

enum Acao {
    Abrir(String),
    Salvar(String),
    Finalizar(String),
    Excluir(String),
    Fechar(String),
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let mut acoes = Vec::new();

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            acoes.extend(self.arquivos.iter().filter(|a| a.sujo).map(|a| Acao::Salvar(a.doc.registro.id.clone())));
        }
        if self.modal.is_none() && ctx.memory(|m| m.focused().is_none()) {
            let (esq, dir) = ctx.input(|i| (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight)));
            if esq { self.variante = (self.variante + 2) % 3 }
            if dir { self.variante = (self.variante + 1) % 3 }
        }

        self.barra_topo(ui);
        match self.variante {
            0 => self.variante_a(ui, &mut acoes),
            1 => self.variante_b(ui, &mut acoes),
            _ => self.variante_c(ui, &mut acoes),
        }
        self.switcher(&ctx);
        self.janela_avisos(&ctx);
        self.modais(&ctx);
        for a in acoes {
            self.executar(a);
        }
    }
}

// ---------- comum ----------

impl App {
    fn barra_topo(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topo").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("MyProvenance");
                ui.separator();
                if ui.button("➕ Novo registro").clicked() {
                    self.modal = Some(Modal::Novo(String::new()));
                }
                if ui.button("⟳ Recarregar").on_hover_text("Relê a pasta de dados").clicked() {
                    if self.arquivos.iter().any(|a| a.sujo) {
                        self.modal = Some(Modal::Recarregar);
                    } else {
                        self.recarregar();
                    }
                }
                if !self.avisos.is_empty() {
                    let t = RichText::new(format!("⚠ {} aviso(s)", self.avisos.len())).color(ui.visuals().warn_fg_color);
                    if ui.button(t).clicked() {
                        self.ver_avisos = !self.ver_avisos;
                    }
                }
                ui.label(RichText::new(&self.msg).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::global_theme_preference_buttons(ui);
                    ui.separator();
                    ui.label(RichText::new(self.pasta.display().to_string()).weak().small())
                        .on_hover_text("Pasta de dados (PROTÓTIPO, pode apagar)");
                });
            });
        });
    }

    fn switcher(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::unique("switcher"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgb(255, 200, 0))
                    .corner_radius(16.0)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .shadow(egui::Shadow { offset: [0, 2], blur: 8, spread: 0, color: Color32::from_black_alpha(90) })
                    .show(ui, |ui| {
                        ui.visuals_mut().override_text_color = Some(Color32::BLACK);
                        ui.horizontal(|ui| {
                            if ui.button("◀").clicked() { self.variante = (self.variante + 2) % 3 }
                            ui.strong(format!("PROTÓTIPO · {}", VARIANTES[self.variante]));
                            if ui.button("▶").clicked() { self.variante = (self.variante + 1) % 3 }
                        });
                    });
            });
    }

    fn janela_avisos(&mut self, ctx: &egui::Context) {
        let mut aberto = self.ver_avisos;
        egui::Window::new("Arquivos ignorados").open(&mut aberto).default_width(560.0).show(ctx, |ui| {
            ui.label("Nenhum destes arquivos foi alterado.");
            for a in &self.avisos {
                ui.label(RichText::new(a).monospace().small());
            }
        });
        self.ver_avisos = aberto;
    }

    fn modais(&mut self, ctx: &egui::Context) {
        let Some(modal) = &mut self.modal else { return };
        let mut fechar = false;
        let mut depois: Option<Box<dyn FnOnce(&mut App)>> = None;
        egui::Modal::new(egui::Id::unique("modal")).show(ctx, |ui| {
            ui.set_width(380.0);
            match modal {
                Modal::Novo(titulo) => {
                    ui.heading("Novo registro");
                    let r = ui.add(egui::TextEdit::singleline(titulo).hint_text("Título (obrigatório)").desired_width(f32::INFINITY));
                    r.request_focus();
                    ui.horizontal(|ui| {
                        let ok = !titulo.trim().is_empty();
                        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.add_enabled(ok, egui::Button::new("Criar")).clicked() || (ok && enter) {
                            let t = titulo.trim().to_owned();
                            depois = Some(Box::new(move |app| app.criar(&t)));
                        }
                        if ui.button("Cancelar").clicked() { fechar = true }
                    });
                }
                Modal::Finalizar(id) => {
                    ui.heading("Finalizar registro?");
                    ui.label("Sem volta. Depois de finalizado, o histórico existente fica imutável; só é possível adicionar novas Entidades/Atividades. Título e descrição continuam editáveis.");
                    ui.horizontal(|ui| {
                        if ui.button("Finalizar").clicked() {
                            let id = id.clone();
                            depois = Some(Box::new(move |app| app.finalizar(&id)));
                        }
                        if ui.button("Cancelar").clicked() { fechar = true }
                    });
                }
                Modal::Excluir(id) => {
                    ui.heading("Excluir registro?");
                    ui.label("O arquivo .json é apagado da pasta de dados.");
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Excluir").color(ui.visuals().error_fg_color)).clicked() {
                            let id = id.clone();
                            depois = Some(Box::new(move |app| app.excluir(&id)));
                        }
                        if ui.button("Cancelar").clicked() { fechar = true }
                    });
                }
                Modal::Conflito(id, Conflito::Alterado) => {
                    ui.heading("Arquivo mudou fora do app");
                    ui.label("Outro programa (Dropbox, Git, editor) alterou o arquivo depois da leitura.");
                    ui.horizontal(|ui| {
                        let id = id.clone();
                        if ui.button("Sobrescrever").clicked() {
                            let id = id.clone();
                            depois = Some(Box::new(move |app| app.gravar(&id, true)));
                        }
                        if ui.button("Recarregar (perde minhas edições)").clicked() {
                            depois = Some(Box::new(move |app| app.reler(&id)));
                        }
                    });
                }
                Modal::Conflito(id, Conflito::Removido) => {
                    ui.heading("Arquivo removido fora do app");
                    ui.horizontal(|ui| {
                        let id = id.clone();
                        if ui.button("Gravar de novo").clicked() {
                            let id = id.clone();
                            depois = Some(Box::new(move |app| app.gravar(&id, true)));
                        }
                        if ui.button("Descartar").clicked() {
                            depois = Some(Box::new(move |app| app.descartar(&id)));
                        }
                    });
                }
                Modal::Recarregar => {
                    ui.heading("Recarregar a pasta?");
                    ui.label("Há edições não gravadas. Recarregar descarta essas edições.");
                    ui.horizontal(|ui| {
                        if ui.button("Recarregar").clicked() {
                            depois = Some(Box::new(|app| app.recarregar()));
                        }
                        if ui.button("Cancelar").clicked() { fechar = true }
                    });
                }
            }
        });
        if fechar || depois.is_some() {
            self.modal = None;
        }
        if let Some(f) = depois {
            f(self);
        }
    }

    fn executar(&mut self, acao: Acao) {
        match acao {
            Acao::Abrir(id) => {
                if !self.abertos.contains(&id) { self.abertos.push(id.clone()) }
                self.sel = Some(id);
            }
            Acao::Salvar(id) => self.gravar(&id, false),
            Acao::Finalizar(id) => self.modal = Some(Modal::Finalizar(id)),
            Acao::Excluir(id) => self.modal = Some(Modal::Excluir(id)),
            Acao::Fechar(id) => self.abertos.retain(|x| *x != id),
        }
    }

    fn idx(&self, id: &str) -> Option<usize> {
        self.arquivos.iter().position(|a| a.doc.registro.id == id)
    }

    fn gravar(&mut self, id: &str, forcar: bool) {
        let Some(i) = self.idx(id) else { return };
        match store::gravar(&mut self.arquivos[i], forcar) {
            Ok(()) => self.msg = format!("Gravado: {}", self.arquivos[i].caminho.file_name().unwrap().to_string_lossy()),
            Err(c) => self.modal = Some(Modal::Conflito(id.into(), c)),
        }
    }

    fn criar(&mut self, titulo: &str) {
        let a = store::criar(&self.pasta, store::novo_documento(titulo));
        self.msg = format!("Criado: {}", a.caminho.file_name().unwrap().to_string_lossy());
        let id = a.doc.registro.id.clone();
        self.arquivos.push(a);
        self.executar(Acao::Abrir(id));
    }

    fn finalizar(&mut self, id: &str) {
        let Some(i) = self.idx(id) else { return };
        let r = &mut self.arquivos[i].doc.registro;
        r.status = Status::Finalizado;
        r.finalizado_em = Some(model::agora());
        self.gravar(id, false);
    }

    fn excluir(&mut self, id: &str) {
        let Some(i) = self.idx(id) else { return };
        let a = self.arquivos.remove(i);
        let _ = std::fs::remove_file(&a.caminho);
        self.msg = format!("Excluído: {}", a.caminho.file_name().unwrap().to_string_lossy());
        self.sel = None;
        self.abertos.retain(|x| x != id);
    }

    fn descartar(&mut self, id: &str) {
        if let Some(i) = self.idx(id) {
            self.arquivos.remove(i);
        }
        self.sel = None;
        self.abertos.retain(|x| x != id);
    }

    fn reler(&mut self, id: &str) {
        let Some(i) = self.idx(id) else { return };
        match store::ler(&self.arquivos[i].caminho) {
            Ok(a) => self.arquivos[i] = a,
            Err(e) => {
                self.avisos.push(e);
                self.descartar(id);
            }
        }
    }

    fn recarregar(&mut self) {
        (self.arquivos, self.avisos) = store::carregar(&self.pasta);
        self.msg = format!("{} registro(s) lido(s)", self.arquivos.len());
    }

    /// Ordem da lista: mais recente primeiro, filtrada pela busca.
    fn visiveis(&self) -> Vec<usize> {
        let q = self.busca.to_lowercase();
        let mut v: Vec<usize> = (0..self.arquivos.len())
            .filter(|&i| q.is_empty() || self.arquivos[i].doc.registro.titulo.to_lowercase().contains(&q))
            .collect();
        v.sort_by(|&a, &b| self.arquivos[b].doc.registro.atualizado_em.cmp(&self.arquivos[a].doc.registro.atualizado_em));
        v
    }
}

fn data(iso: &str) -> String {
    iso.parse::<jiff::Timestamp>()
        .map(|t| t.to_zoned(jiff::tz::TimeZone::system()).strftime("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|_| iso.into())
}

fn selo_status(ui: &mut egui::Ui, s: Status) {
    let cor = match s {
        Status::Rascunho => Color32::from_rgb(70, 130, 220),
        Status::Finalizado => Color32::from_rgb(40, 150, 80),
    };
    egui::Frame::new().fill(cor).corner_radius(8.0).inner_margin(egui::Margin::symmetric(6, 1)).show(ui, |ui| {
        ui.label(RichText::new(s.rotulo()).color(Color32::WHITE).small());
    });
}

/// Título + status + datas + botões. Retorna ações.
fn cabecalho(ui: &mut egui::Ui, a: &mut Arquivo, acoes: &mut Vec<Acao>) {
    let id = a.doc.registro.id.clone();
    ui.horizontal(|ui| {
        selo_status(ui, a.doc.registro.status);
        if a.sujo { ui.label(RichText::new("● não gravado").color(ui.visuals().warn_fg_color)); }
    });
    let r = ui.add(egui::TextEdit::singleline(&mut a.doc.registro.titulo).font(egui::TextStyle::Heading).desired_width(f32::INFINITY));
    if r.changed() { a.sujo = true }
    let reg = &a.doc.registro;
    ui.label(RichText::new(format!(
        "Criado {} · Atualizado {}{}",
        data(&reg.criado_em),
        data(&reg.atualizado_em),
        reg.finalizado_em.as_deref().map(|f| format!(" · Finalizado {}", data(f))).unwrap_or_default()
    )).weak().small());
    ui.label(RichText::new(a.caminho.file_name().unwrap().to_string_lossy()).monospace().weak().small());
    ui.horizontal(|ui| {
        let ok = a.sujo && !a.doc.registro.titulo.trim().is_empty();
        if ui.add_enabled(ok, egui::Button::new("💾 Gravar (Ctrl+S)")).clicked() { acoes.push(Acao::Salvar(id.clone())) }
        if a.doc.registro.status == Status::Rascunho && ui.button("✔ Finalizar").clicked() {
            acoes.push(Acao::Finalizar(id.clone()));
        }
        if ui.button("🗑 Excluir").clicked() { acoes.push(Acao::Excluir(id.clone())) }
    });
}

fn editor_md(ui: &mut egui::Ui, a: &mut Arquivo, linhas: usize) {
    let d = a.doc.registro.descricao.get_or_insert_with(String::new);
    let r = ui.add(
        egui::TextEdit::multiline(d)
            .code_editor()
            .hint_text("Descrição em Markdown")
            .desired_rows(linhas)
            .desired_width(f32::INFINITY),
    );
    let vazio = d.is_empty();
    if r.changed() { a.sujo = true }
    if vazio { a.doc.registro.descricao = None }
}

fn preview_md(ui: &mut egui::Ui, cache: &mut CommonMarkCache, a: &Arquivo) {
    match a.doc.registro.descricao.as_deref() {
        Some(t) => { CommonMarkViewer::new().show(ui, cache, t); }
        None => { ui.label(RichText::new("(sem descrição)").weak().italics()); }
    }
}

/// Grafo como texto: só leitura no protótipo (formulário de Atividade e grafo nativo = Etapas 2–3).
fn conteudo(ui: &mut egui::Ui, a: &Arquivo) {
    let d = &a.doc;
    let nome_ag = |id: &str| d.agentes.iter().find(|g| g.id == id).map_or("?", |g| g.nome.as_str());
    let nome_en = |id: &str| d.entidades.iter().find(|e| e.id == id).map_or("?".to_string(), |e| e.nome.clone());
    egui::CollapsingHeader::new(format!("Atividades ({})", d.atividades.len())).default_open(true).show(ui, |ui| {
        for at in &d.atividades {
            let usadas: Vec<_> = at.entidades_usadas.iter().map(|u| nome_en(u)).collect();
            let geradas: Vec<_> = d.entidades.iter().filter(|e| e.gerada_por_atividade_id == at.id).map(|e| e.nome.as_str()).collect();
            ui.label(format!(
                "{} · {} · {}{}  —  usa [{}] ➡ gera [{}]",
                at.tipo.rotulo(),
                data(&at.data_hora),
                nome_ag(&at.agente_id),
                at.descricao.as_deref().map(|s| format!(" · {s}")).unwrap_or_default(),
                usadas.join(", "),
                geradas.join(", ")
            ));
        }
    });
    egui::CollapsingHeader::new(format!("Entidades ({})", d.entidades.len())).show(ui, |ui| {
        for e in &d.entidades {
            ui.label(format!("{} · {} · {}", e.nome, e.formato.as_deref().unwrap_or("—"), e.licenca.as_deref().unwrap_or("sem licença")));
        }
    });
    egui::CollapsingHeader::new(format!("Agentes ({})", d.agentes.len())).show(ui, |ui| {
        for g in &d.agentes {
            ui.label(format!("{} · {}{}", g.nome, g.tipo.rotulo(), g.afiliacao.as_deref().map(|s| format!(" · {s}")).unwrap_or_default()));
        }
    });
}

fn busca(ui: &mut egui::Ui, q: &mut String) {
    ui.add(egui::TextEdit::singleline(q).hint_text("🔍 Buscar por título").desired_width(f32::INFINITY));
}

// ---------- A: mestre-detalhe (lista à esquerda, editor e preview lado a lado) ----------

impl App {
    fn variante_a(&mut self, ui: &mut egui::Ui, acoes: &mut Vec<Acao>) {
        egui::Panel::left("lista_a").default_size(300.0).show(ui, |ui| {
            busca(ui, &mut self.busca);
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for i in self.visiveis() {
                    let a = &self.arquivos[i];
                    let r = &a.doc.registro;
                    let ativo = self.sel.as_deref() == Some(r.id.as_str());
                    let txt = format!("{}{}\n{} · {}", if a.sujo { "● " } else { "" }, r.titulo, r.status.rotulo(), data(&r.atualizado_em));
                    if ui.add(egui::Button::selectable(ativo, txt).wrap()).clicked() {
                        acoes.push(Acao::Abrir(r.id.clone()));
                    }
                }
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let Some(i) = self.sel.as_deref().and_then(|id| self.idx(id)) else {
                ui.centered_and_justified(|ui| ui.label(RichText::new("Selecione um registro à esquerda").weak()));
                return;
            };
            let (a, md) = (&mut self.arquivos[i], &mut self.md);
            egui::ScrollArea::vertical().show(ui, |ui| {
                cabecalho(ui, a, acoes);
                ui.separator();
                ui.columns(2, |c| {
                    c[0].strong("Descrição (Markdown)");
                    editor_md(&mut c[0], a, 16);
                    c[1].strong("Visualização");
                    preview_md(&mut c[1], md, a);
                });
                ui.separator();
                conteudo(ui, a);
                ui.add_space(48.0);
            });
        });
    }
}

// ---------- B: telas (tabela em tela cheia → tela do registro com "← Voltar"; descrição em abas) ----------

impl App {
    fn variante_b(&mut self, ui: &mut egui::Ui, acoes: &mut Vec<Acao>) {
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(i) = self.sel.as_deref().and_then(|id| self.idx(id)) {
                if ui.button("⬅ Registros").clicked() { self.sel = None; return }
                ui.separator();
                let (a, md, prev) = (&mut self.arquivos[i], &mut self.md, &mut self.b_preview);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    cabecalho(ui, a, acoes);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.selectable_value(prev, false, "✏ Editar");
                        ui.selectable_value(prev, true, "👁 Visualizar");
                    });
                    if *prev { preview_md(ui, md, a) } else { editor_md(ui, a, 20) }
                    ui.separator();
                    conteudo(ui, a);
                    ui.add_space(48.0);
                });
                return;
            }
            ui.heading(format!("Registros ({})", self.arquivos.len()));
            busca(ui, &mut self.busca);
            ui.add_space(6.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("tabela_b").striped(true).num_columns(5).spacing([24.0, 8.0]).show(ui, |ui| {
                    for h in ["Título", "Status", "Atividades", "Entidades", "Atualizado"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for i in self.visiveis() {
                        let a = &self.arquivos[i];
                        let r = &a.doc.registro;
                        if ui.link(format!("{}{}", if a.sujo { "● " } else { "" }, r.titulo)).clicked() {
                            acoes.push(Acao::Abrir(r.id.clone()));
                        }
                        selo_status(ui, r.status);
                        ui.label(a.doc.atividades.len().to_string());
                        ui.label(a.doc.entidades.len().to_string());
                        ui.label(data(&r.atualizado_em));
                        ui.end_row();
                    }
                });
                ui.add_space(48.0);
            });
        });
    }
}

// ---------- C: cards + uma janela flutuante por registro (vários abertos ao mesmo tempo) ----------

impl App {
    fn variante_c(&mut self, ui: &mut egui::Ui, acoes: &mut Vec<Acao>) {
        egui::CentralPanel::default().show(ui, |ui| {
            busca(ui, &mut self.busca);
            ui.add_space(6.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for i in self.visiveis() {
                        let a = &self.arquivos[i];
                        let r = &a.doc.registro;
                        egui::Frame::group(ui.style()).inner_margin(10.0).show(ui, |ui| {
                            ui.set_width(260.0);
                            ui.set_height(130.0);
                            ui.vertical(|ui| {
                                selo_status(ui, r.status);
                                ui.add(egui::Label::new(RichText::new(&r.titulo).strong()).wrap());
                                let resumo: String = r.descricao.as_deref().unwrap_or("").chars().take(110).collect();
                                ui.label(RichText::new(resumo.replace(['#', '*', '_', '|', '>'], "")).weak().small());
                                ui.horizontal(|ui| {
                                    if ui.button("Abrir").clicked() { acoes.push(Acao::Abrir(r.id.clone())) }
                                    ui.label(RichText::new(format!("{} ativ. · {}", a.doc.atividades.len(), data(&r.atualizado_em))).weak().small());
                                });
                            });
                        });
                    }
                });
                ui.add_space(48.0);
            });
        });
        let ctx = ui.ctx().clone();
        for id in self.abertos.clone() {
            let Some(i) = self.idx(&id) else { continue };
            let (a, md) = (&mut self.arquivos[i], &mut self.md);
            let mut aberto = true;
            let titulo = format!("{}{}", if a.sujo { "● " } else { "" }, a.doc.registro.titulo);
            egui::Window::new(titulo).id(egui::Id::unique(&id)).open(&mut aberto).default_size([620.0, 560.0]).show(&ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    cabecalho(ui, a, acoes);
                    ui.separator();
                    editor_md(ui, a, 8);
                    ui.separator();
                    preview_md(ui, md, a);
                    ui.separator();
                    conteudo(ui, a);
                });
            });
            if !aberto { acoes.push(Acao::Fechar(id)) }
        }
    }
}
