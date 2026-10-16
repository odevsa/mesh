use crate::i18n::{Language, TextKey};
use egui::{Align2, Context, Margin, Vec2};

pub fn render_loading_overlay(ctx: &Context, lang: Language) {
    egui::Area::new(egui::Id::new("loading_overlay"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.global_style())
                .shadow(egui::Shadow::NONE)
                .inner_margin(Margin::symmetric(20, 20))
                .show(ui, |ui| {
                    ui.set_min_width(200.0);
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.add_space(8.0);
                        ui.label(lang.t(TextKey::LoadingModel));
                    });
                });
        });
}

pub fn render_empty_overlay(ctx: &Context, lang: Language) -> bool {
    let mut open_dialog = false;
    egui::Area::new(egui::Id::new("empty_overlay"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.global_style())
                .shadow(egui::Shadow::NONE)
                .inner_margin(Margin::symmetric(20, 20))
                .show(ui, |ui| {
                    ui.set_min_width(200.0);
                    ui.vertical_centered(|ui| {
                        if ui.button(lang.t(TextKey::Load3dModel)).clicked() {
                            open_dialog = true;
                        }
                    });
                });
        });
    open_dialog
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
}

pub fn render_about_dialog(ctx: &Context, show_about: &mut bool, lang: Language) {
    if !*show_about {
        return;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *show_about = false;
        return;
    }

    let icon_bytes = include_bytes!("../../assets/icons/128x128/apps/mesh.png");
    let icon_image = image::load_from_memory(icon_bytes).ok().map(|img| {
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_flat_samples().as_slice())
    });

    egui::Window::new(lang.t(TextKey::About))
        .open(show_about)
        .order(egui::Order::Tooltip)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .pivot(Align2::CENTER_CENTER)
        .fixed_pos(ctx.viewport_rect().center())
        .frame(egui::Frame::window(&ctx.global_style()).shadow(egui::Shadow::NONE))
        .show(ctx, |ui| {
            ui.style_mut().interaction.selectable_labels = false;
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                if let Some(color_img) = icon_image {
                    let texture = ctx.load_texture("about_mesh_icon", color_img, Default::default());
                    ui.image((texture.id(), Vec2::new(64.0, 64.0)));
                    ui.add_space(4.0);
                }
                ui.heading(egui::RichText::new("Mesh").strong().size(22.0));
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .weak(),
                );
                ui.add_space(8.0);
                ui.label(lang.t(TextKey::AboutDescription));
                ui.add_space(6.0);
                let link_resp = ui.hyperlink_to("github.com/odevsa/mesh", "https://github.com/odevsa/mesh");
                if link_resp.clicked() {
                    open_url("https://github.com/odevsa/mesh");
                }
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(egui::RichText::new(lang.t(TextKey::SupportedFormats)).strong());
                ui.add_space(4.0);
                ui.label("• STL (.stl)");
                ui.label("• 3MF (.3mf)");
                ui.label("• Wavefront OBJ (.obj)");
                ui.label("• glTF / GLB (.gltf, .glb)");
                ui.label("• Autodesk FBX (.fbx)");
                ui.add_space(6.0);
            });
        });
}

#[derive(Debug, Clone, Copy)]
pub struct ModelStats {
    pub vertices: usize,
    pub lines: usize,
    pub faces: usize,
}

fn format_count(val: usize) -> String {
    let s = val.to_string();
    let bytes = s.as_bytes();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    let len = bytes.len();
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(b as char);
    }
    result
}

pub fn render_details_overlay(
    ctx: &Context,
    lang: Language,
    size: glam::Vec3,
    stats: Option<ModelStats>,
) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("details_overlay"),
    ));

    fn format_dim(val: f32) -> String {
        let rounded = (val * 100.0).round() / 100.0;
        if (rounded - rounded.round()).abs() < 1e-4 {
            format!("{:.0}", rounded)
        } else if (rounded * 10.0 - (rounded * 10.0).round()).abs() < 1e-4 {
            format!("{:.1}", rounded)
        } else {
            format!("{:.2}", rounded)
        }
    }

    let mut lines: Vec<(String, bool)> = Vec::new();
    lines.push((lang.t(TextKey::Dimensions).to_string(), true));
    lines.push((
        format!(
            "X: {}   Y: {}   Z: {}",
            format_dim(size.x),
            format_dim(size.y),
            format_dim(size.z)
        ),
        false,
    ));

    if let Some(st) = stats {
        lines.push((lang.t(TextKey::Statistics).to_string(), true));
        lines.push((
            format!("{}: {}", lang.t(TextKey::Vertices), format_count(st.vertices)),
            false,
        ));
        lines.push((
            format!("{}: {}", lang.t(TextKey::Lines), format_count(st.lines)),
            false,
        ));
        lines.push((
            format!("{}: {}", lang.t(TextKey::Faces), format_count(st.faces)),
            false,
        ));
    }

    let line_height = 16.0;
    let right_x = screen.right() - 15.0;
    let bottom_y = screen.bottom() - 15.0;

    let font_regular = egui::FontId::proportional(12.0);
    let font_bold = egui::FontId::proportional(12.0);
    let text_color = egui::Color32::from_gray(210);
    let strong_color = egui::Color32::WHITE;

    let total_lines = lines.len() as f32;
    for (i, (text, is_strong)) in lines.into_iter().enumerate() {
        let y = bottom_y - (total_lines - 1.0 - i as f32) * line_height;
        let color = if is_strong { strong_color } else { text_color };
        let font = if is_strong { font_bold.clone() } else { font_regular.clone() };
        painter.text(
            egui::pos2(right_x, y),
            Align2::RIGHT_BOTTOM,
            text,
            font,
            color,
        );
    }
}
