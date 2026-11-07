//! Tabela de Registros em tela cheia. Linhas virtualizadas: só as visíveis são desenhadas (rolagem infinita).
use super::{Acao, App, data, icone, selo_status};
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};

impl App {
    /// Índices de `arquivos` que passam na busca, mais recente primeiro.
    fn visiveis(&self) -> Vec<usize> {
        let q = self.busca.trim().to_lowercase();
        let contem = |s: &str| s.to_lowercase().contains(&q);
        let mut v: Vec<(jiff::Timestamp, usize)> = self
            .arquivos
            .iter()
            .enumerate()
            .filter(|(_, a)| q.is_empty() || contem(&a.doc.registro.titulo) || a.doc.registro.descricao.as_deref().is_some_and(contem))
            .map(|(i, a)| (crate::store::atualizado(a), i))
            .collect();
        v.sort_by(|a, b| b.cmp(a));
        v.into_iter().map(|(_, i)| i).collect()
    }

    pub(super) fn tela_lista(&mut self, ui: &mut egui::Ui) -> Option<Acao> {
        let mut acao = None;
        ui.horizontal(|ui| {
            ui.heading(format!("Registros ({})", self.arquivos.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("{} Importar da v1…", icone::IMPORTAR)).on_hover_text("JSON exportado pela v1 web").clicked() {
                    acao = Some(Acao::Importar);
                }
            });
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.busca)
                .hint_text(format!("{} Buscar no título e na descrição", icone::BUSCAR))
                .desired_width(f32::INFINITY),
        );
        ui.add_space(6.0);
        let linhas = self.visiveis();
        if linhas.is_empty() {
            let texto = if self.arquivos.is_empty() {
                format!("Nenhum Registro em {}. Use “Novo Registro” ou “Importar da v1…”.", self.pasta.display())
            } else {
                "Nenhum Registro encontrado.".into()
            };
            ui.label(RichText::new(texto).weak());
            return acao;
        }
        let mut abrir = None;
        let altura = ui.text_style_height(&egui::TextStyle::Body) + 10.0;
        TableBuilder::new(ui)
            .striped(true)
            .sense(egui::Sense::click())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::remainder().clip(true))
            .column(Column::auto().at_least(90.0))
            .columns(Column::auto().at_least(80.0), 2)
            .column(Column::auto().at_least(120.0))
            .header(altura + 4.0, |mut h| {
                for t in ["Título", "Status", "Atividades", "Entidades", "Atualizado"] {
                    h.col(|ui| {
                        ui.strong(t);
                    });
                }
            })
            .body(|corpo| {
                corpo.rows(altura, linhas.len(), |mut linha| {
                    let a = &self.arquivos[linhas[linha.index()]];
                    let r = &a.doc.registro;
                    linha.col(|ui| {
                        ui.add(egui::Label::new(&r.titulo).truncate().selectable(false));
                    });
                    linha.col(|ui| selo_status(ui, r.status));
                    linha.col(|ui| {
                        ui.label(a.doc.atividades.len().to_string());
                    });
                    linha.col(|ui| {
                        ui.label(a.doc.entidades.len().to_string());
                    });
                    linha.col(|ui| {
                        ui.label(data(&r.atualizado_em));
                    });
                    if linha.response().on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        abrir = Some(r.id);
                    }
                });
            });
        abrir.map(Acao::Abrir).or(acao)
    }
}
