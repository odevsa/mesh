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

    let icon_bytes = include_bytes!("../../packaging/linux/assets/icons/hicolor/128x128/apps/mesh.png");
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

pub fn render_animation_controls(
    ctx: &Context,
    controller: &mut crate::loader::AnimationController,
    lang: Language,
) -> bool {
    if !controller.has_animation() {
        return false;
    }

    let mut user_interacted = false;

    egui::Area::new(egui::Id::new("animation_controls_overlay"))
        .anchor(Align2::LEFT_BOTTOM, Vec2::new(16.0, -16.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.global_style())
                .shadow(egui::Shadow::NONE)
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    let clips_len = controller.clips().len();
                    ui.set_width(if clips_len > 1 { 380.0 } else { 330.0 });

                    let duration = controller.duration();
                    let current_time = controller.current_time();

                    // Line 1: 100% width progress bar (dark gray background, light gray progress)
                    if duration > 0.0 {
                        let full_width = ui.available_width();
                        let bar_height = 8.0;
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(full_width, bar_height),
                            egui::Sense::click_and_drag(),
                        );

                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if response.clicked() || response.dragged() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let ratio = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                                controller.seek(ratio * duration);
                                user_interacted = true;
                            }
                        }

                        let progress = (current_time / duration).clamp(0.0, 1.0);
                        let bg_color = egui::Color32::from_gray(55); // cinza escuro
                        let fg_color = if response.hovered() || response.dragged() {
                            egui::Color32::from_gray(215) // cinza mais claro hover/drag
                        } else {
                            egui::Color32::from_gray(180) // cinza mais claro
                        };

                        // Barrinha cinza escuro (fundo 100%)
                        ui.painter().rect_filled(rect, 3.0, bg_color);

                        // Barrinha cinza mais claro (progresso)
                        if progress > 0.0 {
                            let mut prog_rect = rect;
                            prog_rect.set_width((rect.width() * progress).max(2.0));
                            ui.painter().rect_filled(prog_rect, 3.0, fg_color);
                        }
                    }

                    ui.add_space(4.0);

                    // Line 2: 3-column layout (Left: time [/ clip], Center: play/stop/loop, Right: speed)
                    ui.columns(3, |cols| {
                        // Left-aligned: current_time / duration
                        cols[0].with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.2}s / {:.2}s", current_time, duration))
                                    .size(11.0)
                                    .color(ui.visuals().weak_text_color()),
                            );

                            if clips_len > 1 {
                                let current_clip_name = controller.active_clip_name().to_string();
                                egui::ComboBox::from_id_salt("anim_clip_select")
                                    .selected_text(&current_clip_name)
                                    .width(60.0)
                                    .show_ui(ui, |ui| {
                                        for i in 0..clips_len {
                                            let name = if controller.clips()[i].name.is_empty() {
                                                format!("Clip {}", i + 1)
                                            } else {
                                                controller.clips()[i].name.clone()
                                            };
                                            if ui.selectable_label(controller.active_clip_index == i, name).clicked() {
                                                controller.select_clip(i);
                                                user_interacted = true;
                                            }
                                        }
                                    });
                            }
                        });

                        // Center-aligned: play/pause, stop (square), loop
                        cols[1].with_layout(
                            egui::Layout::left_to_right(egui::Align::Center)
                                .with_main_align(egui::Align::Center),
                            |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                let play_pause_icon = if controller.is_playing() { "⏸" } else { "▶" };
                                let play_pause_tooltip = if controller.is_playing() {
                                    lang.t(TextKey::Pause)
                                } else {
                                    lang.t(TextKey::Play)
                                };
                                if ui.button(play_pause_icon).on_hover_text(play_pause_tooltip).clicked() {
                                    controller.toggle_play_pause();
                                    user_interacted = true;
                                }

                                if ui.button("⏹").on_hover_text(lang.t(TextKey::Stop)).clicked() {
                                    controller.stop();
                                    user_interacted = true;
                                }

                                let loop_text = if controller.is_looping { "⇄" } else { "➡" };
                                let loop_btn = ui.button(loop_text).on_hover_text(lang.t(TextKey::Loop));
                                if loop_btn.clicked() {
                                    controller.set_looping(!controller.is_looping);
                                }
                            },
                        );

                        // Right-aligned: speed (without label)
                        cols[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let speed_text = format!("{:.2}x", controller.speed());
                            egui::ComboBox::from_id_salt("anim_speed_select")
                                .selected_text(speed_text)
                                .width(56.0)
                                .show_ui(ui, |ui| {
                                    for &sp in &[0.25, 0.5, 1.0, 1.5, 2.0] {
                                        if ui.selectable_label((controller.speed() - sp).abs() < 1e-4, format!("{:.2}x", sp)).clicked() {
                                            controller.set_speed(sp);
                                        }
                                    }
                                });
                        });
                    });
                });
        });

    user_interacted
}

