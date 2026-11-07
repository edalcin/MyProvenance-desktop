//! UI B (telas, decisão #8): Lista em tela cheia → tela do Registro com "Voltar"; Configurações.
//! Gravação explícita: Gravar / Ctrl+S; sair com edição pendente pergunta Gravar/Descartar/Cancelar.
mod lista;
mod registro;

use crate::{
    config::{self, Config, Tema},
    importar,
    model::{self, Documento, Status},
    store::{self, Arquivo, ErroGravar},
};
use eframe::egui::{self, Color32, RichText};
use egui_commonmark::CommonMarkCache;
use std::path::PathBuf;
use uuid::Uuid;

/// Boxicons 2.1.4 (`assets/boxicons`, MIT). Códigos de `css/boxicons.css`.
pub mod icone {
    pub const VOLTAR: &str = "\u{e9b4}";
    pub const FINALIZAR: &str = "\u{ea46}";
    pub const CONFIG: &str = "\u{ea6e}";
    pub const SISTEMA: &str = "\u{ea96}";
    pub const EDITAR: &str = "\u{eabf}";
    pub const AVISO: &str = "\u{eac5}";
    pub const PASTA: &str = "\u{eae1}";
    pub const ESCURO: &str = "\u{eb94}";
    pub const NOVO: &str = "\u{ebc0}";
    pub const RECARREGAR: &str = "\u{ebd9}";
    pub const GRAVAR: &str = "\u{ebf4}";
    pub const BUSCAR: &str = "\u{ebf7}";
    pub const VISUALIZAR: &str = "\u{ec0c}";
    pub const CLARO: &str = "\u{ec34}";
    pub const EXCLUIR: &str = "\u{ec50}";
    pub const PENDENTE: &str = "\u{ed47}";
    pub const IMPORTAR: &str = "\u{eb1f}";
}

pub struct App {
    cfg: Config,
    aviso_config: Option<String>,
    pasta: PathBuf,
    arquivos: Vec<Arquivo>,
    avisos: Vec<String>,
    tela: Tela,
    busca: String,
    md: CommonMarkCache,
    modal: Option<Modal>,
    ver_avisos: bool,
    msg: String,
    /// O usuário já respondeu à pergunta de pendência; o próximo pedido de fechar passa.
    pode_fechar: bool,
}

enum Tela {
    Lista,
    Registro(Edicao),
    Configuracoes,
}

/// Cópia de trabalho do Registro aberto. A lista mostra só o que está gravado.
struct Edicao {
    id: Uuid,
    doc: Documento,
    sujo: bool,
    visualizar: bool,
}

/// Destino pedido. Com edição pendente, passa antes pela pergunta Gravar/Descartar/Cancelar.
#[derive(Clone, Copy)]
enum Ir {
    Lista,
    Novo,
    Configuracoes,
    Recarregar,
    Fechar,
}

enum Acao {
    Ir(Ir),
    Abrir(Uuid),
    Gravar,
    Finalizar,
    Excluir,
    EscolherPasta,
    PastaPadrao,
    Tema(Tema),
    Importar,
}

enum Modal {
    Novo(String),
    Pendente(Ir),
    Finalizar,
    Excluir,
    Conflito { removido: bool },
    Erro(String),
    Importado { importados: Vec<String>, recusados: Vec<String> },
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let (cfg, aviso_config) = Config::ler();
        aplicar_tema(ctx, cfg.tema);
        let mut app = Self {
            cfg,
            aviso_config,
            pasta: PathBuf::new(),
            arquivos: Vec::new(),
            avisos: Vec::new(),
            tela: Tela::Lista,
            busca: String::new(),
            md: CommonMarkCache::default(),
            modal: None,
            ver_avisos: false,
            msg: String::new(),
            pode_fechar: false,
        };
        app.recarregar();
        app
    }

    fn recarregar(&mut self) {
        self.pasta = self.cfg.pasta();
        let mut avisos: Vec<String> = self.aviso_config.iter().cloned().collect();
        // Só a pasta padrão é criada; uma pasta escolhida que sumiu vira aviso.
        if self.cfg.pasta_dados.is_none()
            && let Err(e) = std::fs::create_dir_all(&self.pasta)
        {
            avisos.push(format!("{}: {e}", self.pasta.display()));
        }
        let carga = store::carregar(&self.pasta);
        avisos.extend(carga.avisos);
        (self.arquivos, self.avisos) = (carga.arquivos, avisos);
        if let Tela::Registro(e) = &mut self.tela {
            match self.arquivos.iter().find(|a| a.doc.registro.id == e.id) {
                Some(a) => (e.doc, e.sujo) = (a.doc.clone(), false),
                None => self.tela = Tela::Lista,
            }
        }
        self.msg = format!("{} registro(s) lido(s)", self.arquivos.len());
    }

    fn sujo(&self) -> bool {
        matches!(&self.tela, Tela::Registro(e) if e.sujo)
    }

    fn ir(&mut self, ctx: &egui::Context, ir: Ir) {
        if self.sujo() {
            self.modal = Some(Modal::Pendente(ir));
        } else {
            self.executar(ctx, ir);
        }
    }

    fn executar(&mut self, ctx: &egui::Context, ir: Ir) {
        match ir {
            Ir::Lista => self.tela = Tela::Lista,
            Ir::Novo => self.modal = Some(Modal::Novo(String::new())),
            Ir::Configuracoes => self.tela = Tela::Configuracoes,
            Ir::Recarregar => self.recarregar(),
            Ir::Fechar => {
                self.pode_fechar = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn aplicar(&mut self, ctx: &egui::Context, acao: Acao) {
        match acao {
            Acao::Ir(ir) => self.ir(ctx, ir),
            Acao::Abrir(id) => self.abrir(id),
            Acao::Gravar => {
                self.gravar(false);
            }
            Acao::Finalizar => self.modal = Some(Modal::Finalizar),
            Acao::Excluir => self.modal = Some(Modal::Excluir),
            Acao::EscolherPasta => {
                if let Some(p) = rfd::FileDialog::new().set_title("Pasta de dados").set_directory(&self.pasta).pick_folder() {
                    self.trocar_pasta(Some(p));
                }
            }
            Acao::PastaPadrao => self.trocar_pasta(None),
            Acao::Tema(t) => {
                self.cfg.tema = t;
                aplicar_tema(ctx, t);
                if let Err(e) = self.cfg.gravar() {
                    self.msg = format!("Tema não gravado no config.json: {e}");
                }
            }
            Acao::Importar => {
                let dialogo = rfd::FileDialog::new().set_title("Importar da v1").add_filter("JSON da v1", &["json"]);
                if let Some(origens) = dialogo.pick_files() {
                    self.importar(origens);
                }
            }
        }
    }

    fn abrir(&mut self, id: Uuid) {
        if let Some(a) = self.arquivos.iter().find(|a| a.doc.registro.id == id) {
            self.tela = Tela::Registro(Edicao { id, doc: a.doc.clone(), sujo: false, visualizar: false });
        }
    }

    fn trocar_pasta(&mut self, pasta: Option<PathBuf>) {
        let antes = std::mem::replace(&mut self.cfg.pasta_dados, pasta);
        if let Err(e) = self.cfg.gravar() {
            self.cfg.pasta_dados = antes;
            self.modal = Some(Modal::Erro(format!("Não foi possível gravar o config.json: {e}")));
            return;
        }
        self.recarregar();
    }

    /// Grava o Registro aberto. `true` = gravou.
    fn gravar(&mut self, forcar: bool) -> bool {
        let Tela::Registro(e) = &mut self.tela else { return false };
        let Some(a) = self.arquivos.iter_mut().find(|a| a.doc.registro.id == e.id) else { return false };
        match store::gravar(a, &e.doc, forcar) {
            Ok(()) => {
                (e.doc, e.sujo) = (a.doc.clone(), false);
                self.msg = format!("Gravado: {}", nome(a));
                return true;
            }
            Err(ErroGravar::Alterado) => self.modal = Some(Modal::Conflito { removido: false }),
            Err(ErroGravar::Removido) => self.modal = Some(Modal::Conflito { removido: true }),
            Err(erro) => self.modal = Some(Modal::Erro(format!("Não foi possível gravar: {erro}"))),
        }
        false
    }

    /// Volta a cópia de trabalho para o que está gravado.
    fn descartar(&mut self) {
        if let Tela::Registro(e) = &mut self.tela
            && let Some(a) = self.arquivos.iter().find(|a| a.doc.registro.id == e.id)
        {
            (e.doc, e.sujo) = (a.doc.clone(), false);
        }
    }

    /// Conflito "Alterado": relê o arquivo do disco e perde as edições.
    fn reler(&mut self) {
        let Tela::Registro(e) = &mut self.tela else { return };
        let Some(i) = self.arquivos.iter().position(|a| a.doc.registro.id == e.id) else { return };
        match store::ler(&self.arquivos[i].caminho) {
            Ok(a) if a.doc.registro.id == e.id => {
                (e.doc, e.sujo) = (a.doc.clone(), false);
                self.arquivos[i] = a;
            }
            lido => {
                let a = self.arquivos.remove(i);
                let motivo = lido.map_or_else(|e| e.to_string(), |_| "o registro.id mudou".into());
                self.avisos.push(format!("{}: {motivo}", nome(&a)));
                self.tela = Tela::Lista;
            }
        }
    }

    /// Conflito "Removido": tira o Registro da lista sem recriar o arquivo.
    fn esquecer(&mut self) {
        if let Tela::Registro(e) = &self.tela {
            let id = e.id;
            self.arquivos.retain(|a| a.doc.registro.id != id);
        }
        self.tela = Tela::Lista;
    }

    fn finalizar(&mut self) {
        let Tela::Registro(e) = &mut self.tela else { return };
        e.doc.registro.status = Status::Finalizado;
        e.doc.registro.finalizado_em = Some(model::agora());
        e.sujo = true;
        self.gravar(false);
    }

    fn excluir(&mut self) {
        let Tela::Registro(e) = &self.tela else { return };
        let Some(i) = self.arquivos.iter().position(|a| a.doc.registro.id == e.id) else { return };
        match std::fs::remove_file(&self.arquivos[i].caminho) {
            Err(erro) if erro.kind() != std::io::ErrorKind::NotFound => {
                self.modal = Some(Modal::Erro(format!("Não foi possível excluir: {erro}")));
            }
            _ => {
                let a = self.arquivos.remove(i);
                self.msg = format!("Excluído: {}", nome(&a));
                self.tela = Tela::Lista;
            }
        }
    }

    fn criar(&mut self, titulo: &str) {
        match store::criar(&self.pasta, model::novo_documento(titulo)) {
            Ok(a) => {
                self.msg = format!("Criado: {}", nome(&a));
                let id = a.doc.registro.id;
                self.arquivos.push(a);
                self.abrir(id);
            }
            Err(e) => self.modal = Some(Modal::Erro(format!("Não foi possível criar o Registro: {e}"))),
        }
    }

    /// Decisão #9: um arquivo novo por JSON da v1; recusados não param os outros. Nenhum Registro abre sozinho.
    fn importar(&mut self, origens: Vec<PathBuf>) {
        // Checa registro.id contra a pasta como está no disco agora (botão só na Lista: sem edição pendente).
        self.recarregar();
        let (mut importados, mut recusados) = (Vec::new(), Vec::new());
        for origem in origens {
            match importar::importar(&self.pasta, &origem, &self.arquivos) {
                Ok(a) => {
                    importados.push(format!("{}: importado como {}", store::nome(&origem), nome(&a)));
                    self.arquivos.push(a);
                }
                Err(e) => recusados.push(format!("{}: {e}", store::nome(&origem))),
            }
        }
        self.msg = format!("{} importado(s), {} não importado(s)", importados.len(), recusados.len());
        self.modal = Some(Modal::Importado { importados, recusados });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && self.sujo() && !self.pode_fechar {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.modal = Some(Modal::Pendente(Ir::Fechar));
        }
        if self.modal.is_none() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) && self.sujo() {
            self.gravar(false);
        }

        let mut acao = self.barra_topo(ui);
        egui::CentralPanel::default().show(ui, |ui| {
            let a = match self.tela {
                Tela::Lista => self.tela_lista(ui),
                Tela::Registro(_) => self.tela_registro(ui),
                Tela::Configuracoes => self.tela_configuracoes(ui),
            };
            acao = acao.take().or(a);
        });
        if let Some(a) = acao {
            self.aplicar(&ctx, a);
        }
        self.janela_avisos(&ctx);
        self.modais(&ctx);
    }
}

impl App {
    fn barra_topo(&mut self, ui: &mut egui::Ui) -> Option<Acao> {
        let mut acao = None;
        egui::Panel::top("topo").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("MyProvenance");
                ui.separator();
                if ui.button(format!("{} Novo Registro", icone::NOVO)).clicked() {
                    acao = Some(Acao::Ir(Ir::Novo));
                }
                if ui.button(format!("{} Recarregar", icone::RECARREGAR)).on_hover_text("Relê a pasta de dados").clicked() {
                    acao = Some(Acao::Ir(Ir::Recarregar));
                }
                if ui.button(format!("{} Configurações", icone::CONFIG)).clicked() {
                    acao = Some(Acao::Ir(Ir::Configuracoes));
                }
                if !self.avisos.is_empty() {
                    let t = RichText::new(format!("{} {} arquivo(s) ignorado(s)", icone::AVISO, self.avisos.len()))
                        .color(ui.visuals().warn_fg_color);
                    if ui.button(t).clicked() {
                        self.ver_avisos = !self.ver_avisos;
                    }
                }
                ui.label(RichText::new(&self.msg).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for (tema, ic, dica) in [(Tema::Escuro, icone::ESCURO, "Escuro"), (Tema::Claro, icone::CLARO, "Claro"), (Tema::Sistema, icone::SISTEMA, "Tema do sistema")] {
                        if ui.selectable_label(self.cfg.tema == tema, ic).on_hover_text(dica).clicked() {
                            acao = Some(Acao::Tema(tema));
                        }
                    }
                    ui.separator();
                    ui.add(egui::Label::new(RichText::new(self.pasta.display().to_string()).weak().small()).truncate())
                        .on_hover_text("Pasta de dados");
                });
            });
        });
        acao
    }

    fn tela_configuracoes(&mut self, ui: &mut egui::Ui) -> Option<Acao> {
        let mut acao = None;
        if ui.button(format!("{} Registros", icone::VOLTAR)).clicked() {
            acao = Some(Acao::Ir(Ir::Lista));
        }
        ui.separator();
        ui.heading("Configurações");
        ui.add_space(8.0);
        ui.strong("Pasta de dados");
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.pasta.display().to_string()).monospace());
            if self.cfg.pasta_dados.is_none() {
                ui.label(RichText::new("(padrão)").weak());
            }
        });
        ui.horizontal(|ui| {
            if ui.button(format!("{} Escolher pasta…", icone::PASTA)).clicked() {
                acao = Some(Acao::EscolherPasta);
            }
            let padrao = config::pasta_padrao();
            if ui
                .add_enabled(self.cfg.pasta_dados.is_some(), egui::Button::new("Usar a pasta padrão"))
                .on_hover_text(padrao.display().to_string())
                .clicked()
            {
                acao = Some(Acao::PastaPadrao);
            }
        });
        ui.label(
            RichText::new("Trocar a pasta não move os arquivos. O app lê só os .json do nível de cima da pasta.").weak(),
        );
        acao
    }

    fn janela_avisos(&mut self, ctx: &egui::Context) {
        let mut aberto = self.ver_avisos;
        egui::Window::new("Arquivos ignorados").open(&mut aberto).default_width(620.0).show(ctx, |ui| {
            ui.label("Nenhum destes arquivos foi alterado.");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for a in &self.avisos {
                    ui.label(RichText::new(a).monospace().small());
                }
            });
        });
        self.ver_avisos = aberto && !self.avisos.is_empty();
    }

    fn modais(&mut self, ctx: &egui::Context) {
        let Some(modal) = &mut self.modal else { return };
        let mut fechar = false;
        let mut depois: Option<Box<dyn FnOnce(&mut App)>> = None;
        let resposta = egui::Modal::new(egui::Id::unique("modal")).show(ctx, |ui| {
            ui.set_width(420.0);
            match modal {
                Modal::Novo(titulo) => {
                    ui.heading("Novo Registro");
                    let r = ui.add(egui::TextEdit::singleline(titulo).hint_text("Título (obrigatório)").desired_width(f32::INFINITY));
                    // Ler Enter antes de request_focus: lost_focus() é avaliado na hora e o request_focus o zera.
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    r.request_focus();
                    let t = titulo.trim().to_owned();
                    let erro = model::validar(&model::novo_documento(&t)).err();
                    if let Some(e) = &erro
                        && !t.is_empty()
                    {
                        ui.colored_label(ui.visuals().error_fg_color, e.to_string());
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(erro.is_none(), egui::Button::new("Criar")).clicked() || (erro.is_none() && enter) {
                            depois = Some(Box::new(move |app| app.criar(&t)));
                        }
                        if ui.button("Cancelar").clicked() {
                            fechar = true;
                        }
                    });
                }
                Modal::Pendente(ir) => {
                    let ir = *ir;
                    ui.heading("Edição não gravada");
                    ui.label("Este Registro tem edições que ainda não foram gravadas.");
                    ui.horizontal(|ui| {
                        let ctx = ui.ctx().clone();
                        if ui.button(format!("{} Gravar", icone::GRAVAR)).clicked() {
                            depois = Some(Box::new(move |app| {
                                if app.gravar(false) {
                                    app.executar(&ctx, ir);
                                }
                            }));
                        } else if ui.button("Descartar").clicked() {
                            depois = Some(Box::new(move |app| {
                                app.descartar();
                                app.executar(&ctx, ir);
                            }));
                        }
                        if ui.button("Cancelar").clicked() {
                            fechar = true;
                        }
                    });
                }
                Modal::Finalizar => {
                    ui.heading("Finalizar o Registro?");
                    ui.label(
                        "Sem volta. Depois de finalizado, o histórico existente fica imutável: só é possível adicionar \
                         novas Entidades/Atividades. Título e descrição continuam editáveis. Edições pendentes são gravadas junto.",
                    );
                    ui.horizontal(|ui| {
                        if ui.button(format!("{} Finalizar", icone::FINALIZAR)).clicked() {
                            depois = Some(Box::new(App::finalizar));
                        }
                        if ui.button("Cancelar").clicked() {
                            fechar = true;
                        }
                    });
                }
                Modal::Excluir => {
                    ui.heading("Excluir o Registro?");
                    ui.label("O arquivo .json é apagado da pasta de dados.");
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new(format!("{} Excluir", icone::EXCLUIR)).color(ui.visuals().error_fg_color)).clicked() {
                            depois = Some(Box::new(App::excluir));
                        }
                        if ui.button("Cancelar").clicked() {
                            fechar = true;
                        }
                    });
                }
                Modal::Conflito { removido: false } => {
                    ui.heading("O arquivo mudou fora do app");
                    ui.label("Outro programa (Dropbox, Git, editor) alterou o arquivo depois da leitura.");
                    ui.horizontal(|ui| {
                        if ui.button("Sobrescrever").clicked() {
                            depois = Some(Box::new(|app| {
                                app.gravar(true);
                            }));
                        }
                        if ui.button("Recarregar (perde as minhas edições)").clicked() {
                            depois = Some(Box::new(App::reler));
                        }
                        if ui.button("Cancelar").clicked() {
                            fechar = true;
                        }
                    });
                }
                Modal::Conflito { removido: true } => {
                    ui.heading("O arquivo foi removido fora do app");
                    ui.horizontal(|ui| {
                        if ui.button("Gravar de novo").clicked() {
                            depois = Some(Box::new(|app| {
                                app.gravar(true);
                            }));
                        }
                        if ui.button("Descartar").clicked() {
                            depois = Some(Box::new(App::esquecer));
                        }
                        if ui.button("Cancelar").clicked() {
                            fechar = true;
                        }
                    });
                }
                Modal::Erro(msg) => {
                    ui.heading("Erro");
                    ui.label(msg.as_str());
                    if ui.button("OK").clicked() {
                        fechar = true;
                    }
                }
                Modal::Importado { importados, recusados } => {
                    ui.set_width(620.0);
                    ui.heading("Importar da v1");
                    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        ui.strong(format!("Importados ({})", importados.len()));
                        for t in importados.iter() {
                            ui.label(RichText::new(t).monospace().small());
                        }
                        if !recusados.is_empty() {
                            ui.add_space(6.0);
                            ui.strong(format!("Não importados ({})", recusados.len()));
                            for t in recusados.iter() {
                                ui.label(RichText::new(t).monospace().small().color(ui.visuals().error_fg_color));
                            }
                        }
                    });
                    ui.label(RichText::new("Os arquivos de origem não foram alterados.").weak());
                    if ui.button("OK").clicked() {
                        fechar = true;
                    }
                }
            }
        });
        if fechar || depois.is_some() || resposta.should_close() {
            self.modal = None;
        }
        if let Some(f) = depois {
            f(self);
        }
    }
}

fn aplicar_tema(ctx: &egui::Context, t: Tema) {
    ctx.set_theme(match t {
        Tema::Sistema => egui::ThemePreference::System,
        Tema::Claro => egui::ThemePreference::Light,
        Tema::Escuro => egui::ThemePreference::Dark,
    });
}

fn nome(a: &Arquivo) -> String {
    a.caminho.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// ISO 8601 → `dd/mm/aaaa hh:mm` no fuso do sistema.
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
    // Button, não Frame: o Frame estica até a altura da linha da tabela.
    let selo = egui::Button::new(RichText::new(s.rotulo()).color(Color32::WHITE).small()).fill(cor).corner_radius(8.0);
    ui.add(selo.sense(egui::Sense::hover()));
}
