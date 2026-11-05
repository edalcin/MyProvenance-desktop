//! Tela do Registro: título, descrição Markdown (abas Editar/Visualizar) e conteúdo só leitura.
//! Formulários de Atividade/Entidade e grafo nativo: Etapas 2–3.
use super::{Acao, App, Ir, Tela, data, icone, nome, selo_status};
use crate::model::{self, Documento, Status};
use eframe::egui::{self, RichText};
use egui_commonmark::CommonMarkViewer;

impl App {
    pub(super) fn tela_registro(&mut self, ui: &mut egui::Ui) -> Option<Acao> {
        let Tela::Registro(e) = &mut self.tela else { return None };
        let arquivo = self.arquivos.iter().find(|a| a.doc.registro.id == e.id).map(nome).unwrap_or_default();
        let invalido = model::validar(&e.doc).err();
        let mut acao = None;

        ui.horizontal(|ui| {
            if ui.button(format!("{} Registros", icone::VOLTAR)).clicked() {
                acao = Some(Acao::Ir(Ir::Lista));
            }
            ui.separator();
            selo_status(ui, e.doc.registro.status);
            if e.sujo {
                ui.label(RichText::new(format!("{} não gravado", icone::PENDENTE)).color(ui.visuals().warn_fg_color));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("{} Excluir", icone::EXCLUIR)).clicked() {
                    acao = Some(Acao::Excluir);
                }
                if e.doc.registro.status == Status::Rascunho
                    && ui.add_enabled(invalido.is_none(), egui::Button::new(format!("{} Finalizar", icone::FINALIZAR))).clicked()
                {
                    acao = Some(Acao::Finalizar);
                }
                let gravar = egui::Button::new(format!("{} Gravar", icone::GRAVAR)).shortcut_text("Ctrl+S");
                if ui.add_enabled(e.sujo && invalido.is_none(), gravar).clicked() {
                    acao = Some(Acao::Gravar);
                }
            });
        });
        if let Some(erro) = &invalido {
            ui.colored_label(ui.visuals().error_fg_color, format!("Não dá para gravar: {erro}"));
        }
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut e.doc.registro.titulo)
                    .font(egui::TextStyle::Heading)
                    .hint_text("Título (obrigatório)")
                    .desired_width(f32::INFINITY),
            );
            e.sujo |= r.changed();
            let reg = &e.doc.registro;
            let finalizado = reg.finalizado_em.as_deref().map(|f| format!(" · Finalizado {}", data(f))).unwrap_or_default();
            ui.label(
                RichText::new(format!("Criado {} · Atualizado {}{finalizado} · {arquivo}", data(&reg.criado_em), data(&reg.atualizado_em)))
                    .weak()
                    .small(),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.selectable_value(&mut e.visualizar, false, format!("{} Editar", icone::EDITAR));
                ui.selectable_value(&mut e.visualizar, true, format!("{} Visualizar", icone::VISUALIZAR));
            });
            if e.visualizar {
                match e.doc.registro.descricao.as_deref() {
                    Some(t) => drop(CommonMarkViewer::new().show(ui, &mut self.md, t)),
                    None => drop(ui.label(RichText::new("(sem descrição)").weak().italics())),
                };
            } else {
                let mut texto = e.doc.registro.descricao.take().unwrap_or_default();
                let r = ui.add(
                    egui::TextEdit::multiline(&mut texto)
                        .code_editor()
                        .hint_text("Descrição em Markdown")
                        .desired_rows(16)
                        .desired_width(f32::INFINITY),
                );
                e.sujo |= r.changed();
                e.doc.registro.descricao = (!texto.is_empty()).then_some(texto);
            }
            ui.separator();
            conteudo(ui, &e.doc);
            ui.add_space(24.0);
        });
        acao
    }
}

fn conteudo(ui: &mut egui::Ui, d: &Documento) {
    let nome_agente = |id| d.agentes.iter().find(|g| g.id == id).map_or("?", |g| g.nome.as_str());
    let nome_entidade = |id| d.entidades.iter().find(|e| e.id == id).map_or("?", |e| e.nome.as_str());
    egui::CollapsingHeader::new(format!("Atividades ({})", d.atividades.len())).default_open(true).show(ui, |ui| {
        for at in &d.atividades {
            let usadas: Vec<_> = at.entidades_usadas.iter().map(|&u| nome_entidade(u)).collect();
            let geradas: Vec<_> = d.entidades.iter().filter(|e| e.gerada_por_atividade_id == at.id).map(|e| e.nome.as_str()).collect();
            ui.label(format!(
                "{} · {} · {}{}  —  usa [{}]; gera [{}]",
                at.tipo.rotulo(),
                data(&at.data_hora),
                nome_agente(at.agente_id),
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
