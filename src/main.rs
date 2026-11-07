// Release sem janela de console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod export;
mod importar;
mod model;
mod store;
mod ui;

use eframe::egui;
use std::sync::Arc;

fn main() -> eframe::Result {
    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MyProvenance")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MyProvenance",
        opcoes,
        Box::new(|cc| {
            fontes(&cc.egui_ctx);
            Ok(Box::new(ui::App::new(&cc.egui_ctx)))
        }),
    )
}

/// egui 0.37 não traz emoji por padrão: os ícones vêm da fonte Boxicons, como fallback das fontes padrão.
fn fontes(ctx: &egui::Context) {
    let mut f = egui::FontDefinitions::default();
    f.font_data.insert("boxicons".into(), Arc::new(egui::FontData::from_static(include_bytes!("../assets/boxicons/boxicons.ttf"))));
    for familia in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        f.families.entry(familia).or_default().push("boxicons".into());
    }
    ctx.set_fonts(f);
}
