//! WMK BIM Studio: primeiro estágio de importação de plantas raster na janela nativa.
//! JPG/PNG são referências de estudo: o traçado não altera o modelo BIM, e é exportado como JSON.
//! PDF, persistência no projeto e conversão semântica para paredes exigem outras etapas.
use std::path::{Path, PathBuf};
use egui::{Color32, ColorImage, Context, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use serde_json::json;
use crate::BimApp;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Calibrate, Trace, Pan }

pub(crate) struct PlantImport {
    open: bool,
    path: Option<PathBuf>,
    texture: Option<TextureHandle>,
    pixels: [usize; 2],
    meters: f32,
    pixels_per_meter: Option<f32>,
    mode: Mode,
    first: Option<Pos2>,
    lines: Vec<(Pos2, Pos2)>,
    message: String,
}

impl Default for PlantImport {
    fn default() -> Self {
        Self {
            open: false, path: None, texture: None, pixels: [1, 1],
            meters: 4.0, pixels_per_meter: None, mode: Mode::Calibrate,
            first: None, lines: Vec::new(),
            message: "Escolha uma planta JPG ou PNG para começar.".to_owned(),
        }
    }
}

impl PlantImport {
    fn load(&mut self, ctx: &Context, path: &Path) {
        self.open = true;
        let outcome = (|| -> Result<(), String> {
            let meta = std::fs::metadata(path).map_err(|e| format!("Arquivo inacessível: {e}"))?;
            if meta.len() > 24 * 1024 * 1024 { return Err("Imagem acima de 24 MB. Use uma planta menor.".into()); }
            let bytes = std::fs::read(path).map_err(|e| format!("Não foi possível ler: {e}"))?;
            let format = image::guess_format(&bytes).map_err(|e| format!("Formato não reconhecido: {e}"))?;
            if !matches!(format, image::ImageFormat::Png | image::ImageFormat::Jpeg) {
                return Err("Este primeiro importador nativo aceita JPG e PNG. PDF está no roadmap.".into());
            }
            let decoded = image::load_from_memory_with_format(&bytes, format)
                .map_err(|e| format!("Imagem inválida: {e}"))?;
            if decoded.width() > 6000 || decoded.height() > 6000 {
                return Err("Dimensão acima de 6000 pixels; reduza a imagem antes de importar.".into());
            }
            let rgba = decoded.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            self.texture = Some(ctx.load_texture("wmk-planta-baixa", image, TextureOptions::LINEAR));
            self.path = Some(path.to_path_buf());
            self.pixels = size;
            self.lines.clear();
            self.first = None;
            self.pixels_per_meter = None;
            self.mode = Mode::Calibrate;
            Ok(())
        })();
        self.message = match outcome {
            Ok(()) => "Planta carregada. Informe a medida real, clique em Calibrar e marque seus dois extremos.".into(),
            Err(reason) => reason,
        };
    }

    fn save_json(&mut self) {
        let Some(scale) = self.pixels_per_meter else {
            self.message = "Calibre uma distância conhecida antes de exportar.".into();
            return;
        };
        if self.lines.is_empty() {
            self.message = "Marque pelo menos um segmento antes de exportar.".into();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("wmk-planta-tracada.json")
            .save_file() else { return };
        let segments: Vec<_> = self.lines.iter().enumerate().map(|(n, (a,b))| json!({
            "id": n+1,
            "start_mm": {"x": a.x as f64 * 1000.0 / scale as f64, "y": a.y as f64 * 1000.0 / scale as f64},
            "end_mm": {"x": b.x as f64 * 1000.0 / scale as f64, "y": b.y as f64 * 1000.0 / scale as f64}
        })).collect();
        let data = json!({
            "format": "wmk-plan-trace-v1",
            "source": self.path.as_ref().and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string()),
            "units": "mm",
            "meters_per_pixel": 1.0 / scale,
            "walls": segments,
            "note": "Linhas CAD de referência; não são paredes BIM"
        });
        let result = serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())
            .and_then(|data| std::fs::write(path, data).map_err(|e| e.to_string()));
        self.message = match result {
            Ok(()) => "Traçado JSON salvo. Ainda não é um modelo BIM.".into(),
            Err(e) => format!("Erro ao salvar: {e}"),
        };
    }

    fn controls(&mut self, ui: &mut Ui) {
        if let Some(path) = &self.path {
            ui.label(format!("Arquivo: {}", path.display()));
        }
        ui.horizontal(|ui| {
            ui.label("Distância conhecida (m):");
            ui.add(egui::DragValue::new(&mut self.meters).speed(0.1).range(0.01..=100_000.0));
        });
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.mode, Mode::Calibrate, "1. Calibrar");
            ui.selectable_value(&mut self.mode, Mode::Trace, "2. Traçar parede");
            if ui.button("Desfazer").clicked() { self.lines.pop(); self.first = None; }
            if ui.button("Limpar").clicked() { self.lines.clear(); self.first = None; }
            if ui.button("Exportar JSON").clicked() { self.save_json(); }
        });
        if let Some(px) = self.pixels_per_meter {
            ui.label(format!("Escala: {:.2} pixels/metro · {} segmentos", px, self.lines.len()));
        }
        ui.label(&self.message);
        ui.label("O traçado é uma referência visual. Não modifica as paredes do projeto BIM.");
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let Some(texture) = &self.texture else { return };
        let native = Vec2::new(self.pixels[0] as f32, self.pixels[1] as f32);
        let max_size = ui.available_size();
        let ratio = (max_size.x / native.x).min((max_size.y / native.y).max(0.01)).clamp(0.01, 1.0);
        let (rect, response) = ui.allocate_exact_size(native * ratio, Sense::click());
        ui.painter().image(texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        let to_screen = |p: Pos2| rect.min + p.to_vec2() * ratio;
        for (a, b) in &self.lines {
            ui.painter().line_segment([to_screen(*a), to_screen(*b)], Stroke::new(3.0, Color32::RED));
        }
        if let Some(a) = self.first {
            ui.painter().circle_filled(to_screen(a), 5.0, Color32::YELLOW);
        }
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let p = Pos2::new((pointer.x - rect.min.x) / ratio, (pointer.y - rect.min.y) / ratio);
                if let Some(a) = self.first.take() {
                    let distance = a.distance(p);
                    if distance < 2.0 {
                        self.message = "Pontos muito próximos; tente novamente.".into();
                        self.first = Some(a);
                    } else if self.mode == Mode::Calibrate {
                        if self.meters > 0.0 && self.meters.is_finite() {
                            self.pixels_per_meter = Some(distance / self.meters);
                            self.mode = Mode::Trace;
                            self.message = "Escala definida. Agora marque o início e o fim de cada parede.".into();
                        }
                    } else {
                        self.lines.push((a, p));
                        self.message = format!("Segmentos marcados: {}. Use Exportar JSON para guardar.", self.lines.len());
                    }
                } else {
                    self.first = Some(p);
                    self.message = "Primeiro ponto marcado. Clique no segundo.".into();
                }
            }
        }
    }
}

impl BimApp {
    pub(crate) fn import_plant_dialog(&mut self, ctx: &Context) {
        let dialog = rfd::AsyncFileDialog::new().add_filter("Plantas JPG/PNG", &["jpg", "jpeg", "png"]);
        let context = ctx.clone();
        self.ask_files(ctx, dialog, move |app, paths| {
            if let Some(path) = paths.into_iter().next() { app.plant_import.load(&context, &path); }
        });
    }

    pub(crate) fn plant_import_window(&mut self, ctx: &Context) {
        if !self.plant_import.open { return; }
        let mut open = true;
        egui::Window::new("WMK • Importar planta baixa (JPG/PNG)")
            .open(&mut open)
            .default_size([950.0, 730.0])
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.plant_import.controls(ui);
                    ui.separator();
                    self.plant_import.canvas(ui);
                });
            });
        if !open { self.plant_import.open = false; }
    }
}
