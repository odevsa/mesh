use egui::{Align2, Context, Margin, Vec2};

pub fn render_loading_overlay(ctx: &Context) {
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
                        ui.label("Loading model...");
                    });
                });
        });
}

pub fn render_empty_overlay(ctx: &Context) -> bool {
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
                        if ui.button("Load 3D Model").clicked() {
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

pub fn render_about_dialog(ctx: &Context, show_about: &mut bool) {
    if !*show_about {
        return;
    }

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *show_about = false;
        return;
    }

    let mut close_clicked = false;
    let icon_bytes = include_bytes!("../../assets/icons/128x128/apps/mesh.png");
    let icon_image = image::load_from_memory(icon_bytes).ok().map(|img| {
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_flat_samples().as_slice())
    });

    egui::Window::new("About")
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
                ui.label("A simple and lightweight 3D mesh viewer written in Rust.");
                ui.add_space(6.0);
                let link_resp = ui.hyperlink_to("github.com/odevsa/mesh", "https://github.com/odevsa/mesh");
                if link_resp.clicked() {
                    open_url("https://github.com/odevsa/mesh");
                }
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(egui::RichText::new("Supported Formats").strong());
                ui.add_space(4.0);
                ui.label("• STL (.stl)");
                ui.label("• 3MF (.3mf)");
                ui.label("• Wavefront OBJ (.obj)");
                ui.label("• glTF / GLB (.gltf, .glb)");

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                if ui.button("  Close  ").clicked() {
                    close_clicked = true;
                }
                ui.add_space(4.0);
            });
        });

    if close_clicked {
        *show_about = false;
    }
}

