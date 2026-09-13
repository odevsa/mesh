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
