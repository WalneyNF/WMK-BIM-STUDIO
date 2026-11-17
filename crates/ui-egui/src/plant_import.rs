//! WMK BIM Studio: primeiro estágio de importação de plantas raster na janela nativa.
//! JPG/PNG são referências de estudo: o traçado não altera o modelo BIM, e é exportado como JSON.
//! PDF, persistência no projeto e conversão semântica para paredes exigem outras etapas.
use std::path::{Path, PathBuf};
use egui::{Color32, ColorImage, Context, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2};
use serde_json::json;
use crate::BimApp;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Calibrate, Trace, Measure, Pan }

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
    /// Ampliação relativa ao enquadramento na janela: 1 = ajustar.
    zoom: f32,
    show_measurements: bool,
    snap: bool,
    opacity: f32,
    measurements: Vec<(Pos2,Pos2)>,
}

impl Default for PlantImport {
    fn default() -> Self {
        Self {
            open: false, path: None, texture: None, pixels: [1, 1],
            meters: 4.0, pixels_per_meter: None, mode: Mode::Calibrate,
            first: None, lines: Vec::new(),
            message: "Escolha uma planta JPG ou PNG para começar.".to_owned(),
            zoom: 1.0,
            show_measurements: true,
            snap: true,
            opacity: 1.0,
            measurements: Vec::new(),
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
            self.measurements.clear();
            self.first = None;
            self.pixels_per_meter = None;
            self.mode = Mode::Calibrate;
            self.zoom = 1.0;
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
        let measures: Vec<_> = self.measurements.iter().map(|(a,b)| json!({
            "start_px": [a.x,a.y], "end_px":[b.x,b.y], "length_m": a.distance(*b)/scale
        })).collect();
        let data = json!({
            "measurements": measures,
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
            ui.selectable_value(&mut self.mode, Mode::Measure, "3. Trena virtual");
            if ui.button("Desfazer").clicked() { self.lines.pop(); self.first = None; }
            if ui.button("Limpar").clicked() { self.lines.clear(); self.first = None; }
            if ui.button("Exportar JSON").clicked() { self.save_json(); }
            if ui.button("Limpar trena").clicked() { self.measurements.clear(); self.first = None; }
        });
        if let Some(px) = self.pixels_per_meter {
            ui.label(format!("Escala: {:.2} pixels/metro · {} segmentos", px, self.lines.len()));
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Zoom:");
            if ui.button("−").on_hover_text("Diminuir zoom").clicked() {
                self.zoom = (self.zoom / 1.25).clamp(0.1, 5.0);
            }
            if ui.button("+").on_hover_text("Aumentar zoom").clicked() {
                self.zoom = (self.zoom * 1.25).clamp(0.1, 5.0);
            }
            if ui.button("Ajustar").clicked() {
                self.zoom = 1.0;
            }
            ui.add(egui::Slider::new(&mut self.zoom, 0.1..=5.0).text("x").logarithmic(true));
            ui.label(format!("{:.0}%", self.zoom * 100.0));
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.show_measurements, "Mostrar comprimentos");
            ui.checkbox(&mut self.snap, "Snap nos extremos");
            ui.add(egui::Slider::new(&mut self.opacity, 0.05..=1.0).text("Opacidade da planta"));
        });
        ui.label(&self.message);
        ui.label("Use as barras de rolagem horizontal/vertical para navegar na imagem ampliada. O zoom não altera as medidas reais.");
        ui.label("O traçado é uma referência visual. Não modifica as paredes do projeto BIM.");
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let Some(texture) = &self.texture else { return };
        let native = Vec2::new(self.pixels[0] as f32, self.pixels[1] as f32);
        // A área visível (clip_rect) permanece estável quando a ScrollArea amplia o conteúdo.
        let visible = ui.clip_rect().size();
        let fit = ((visible.x - 16.0) / native.x)
            .min((visible.y - 16.0) / native.y)
            .clamp(0.01, 1.0);
        let ratio = fit * self.zoom;
        let (rect, response) = ui.allocate_exact_size(native * ratio, Sense::click());
        let tint = Color32::from_white_alpha((self.opacity * 255.0) as u8);
        ui.painter().image(texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
        let to_screen = |p: Pos2| rect.min + p.to_vec2() * ratio;
        for (a, b) in &self.lines {
            ui.painter().line_segment([to_screen(*a), to_screen(*b)], Stroke::new(3.0, Color32::RED));
            if self.show_measurements {
                if let Some(px) = self.pixels_per_meter {
                    let length = a.distance(*b) / px;
                    let center = to_screen(Pos2::new((a.x+b.x)*0.5,(a.y+b.y)*0.5));
                    ui.painter().text(center, egui::Align2::CENTER_BOTTOM, format!("{length:.2} m"), egui::FontId::proportional(13.0), Color32::YELLOW);
                }
            }
        }
        for (a, b) in &self.measurements {
            ui.painter().line_segment([to_screen(*a), to_screen(*b)], Stroke::new(2.0, Color32::LIGHT_BLUE));
            if let Some(px) = self.pixels_per_meter {
                let value = a.distance(*b) / px;
                let center = to_screen(Pos2::new((a.x+b.x)*0.5,(a.y+b.y)*0.5));
                ui.painter().text(center, egui::Align2::CENTER_BOTTOM, format!("{value:.2} m"), egui::FontId::proportional(13.0), Color32::LIGHT_BLUE);
            }
        }
        if let Some(a) = self.first {
            ui.painter().circle_filled(to_screen(a), 5.0, Color32::YELLOW);
        }
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let mut p = Pos2::new((pointer.x - rect.min.x) / ratio, (pointer.y - rect.min.y) / ratio);
                if self.mode == Mode::Trace && self.snap {
                    let max_dist = 12.0 / ratio;
                    let mut nearest = max_dist;
                    for (a,b) in &self.lines {
                        for end in [a,b] {
                            let distance = p.distance(*end);
                            if distance < nearest {
                                p = *end;
                                nearest = distance;
                            }
                        }
                    }
                }
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
                    } else if self.mode == Mode::Measure {
                        self.measurements.push((a,p));
                        let value = self.pixels_per_meter.map(|px| format!("{:.2} m",distance/px))
                            .unwrap_or_else(|| format!("{distance:.0} pixels (calibre primeiro)"));
                        self.message = format!("Trena: {value}. Medições feitas: {}",self.measurements.len());
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
                self.plant_import.controls(ui);
                ui.separator();
                egui::ScrollArea::both()
                    .id_salt("wmk-planta-native-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.plant_import.canvas(ui);
                    });
            });
        if !open { self.plant_import.open = false; }
    }
}
