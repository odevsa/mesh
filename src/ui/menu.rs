use crate::config::{Config, ModelPosition};
use egui::{Context, Pos2};

#[derive(Default)]
pub struct UiResponse {
    pub open_file_dialog: bool,
    pub unload_model_requested: bool,
    pub reset_camera_requested: bool,
    pub reset_defaults_requested: bool,
    pub close_app_requested: bool,
    pub config_changed: bool,
    pub bg_changed: bool,
    pub obj_color_changed: bool,
    pub scale_changed: bool,
    pub position_changed: bool,
    pub grid_rebuild: bool,
    pub axes_rebuild: bool,
    pub controls_changed: bool,
    pub dummy_box_changed: bool,
    pub close_menu_requested: bool,
}

fn menu_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(text)
            .frame_when_inactive(false),
    )
}

fn menu_button_enabled(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .frame_when_inactive(false),
    )
}

pub fn render_context_menu(
    ctx: &Context,
    menu_pos: Pos2,
    cfg: &mut Config,
    has_model: bool,
) -> UiResponse {
    ctx.global_style_mut(|s| s.animation_time = 0.0);

    let wide_mode_id = egui::Id::new("menu_wide_mode");
    let is_wide = ctx.data(|d| d.get_temp::<bool>(wide_mode_id).unwrap_or(false));
    let menu_width = if is_wide { 230.0 } else { 170.0 };

    let mut resp = UiResponse::default();

    let area_resp = egui::Area::new(egui::Id::new("context_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(menu_pos)
        .constrain(true)
        .show(ctx, |ui| {
            ui.set_width(menu_width);
            egui::Frame::menu(&ctx.global_style())
                .shadow(egui::Shadow::NONE)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        ui.spacing_mut().slider_width = 80.0;
                        ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
                        ui.visuals_mut().widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        ui.visuals_mut().widgets.active.bg_stroke = egui::Stroke::NONE;

                        if menu_button(ui, "Load 3D Model").clicked() {
                            resp.open_file_dialog = true;
                            resp.close_menu_requested = true;
                        }

                        if menu_button_enabled(ui, has_model, "Unload 3D Model").clicked() {
                            resp.unload_model_requested = true;
                            resp.close_menu_requested = true;
                        }

                        ui.separator();

                        let r_grid = ui.collapsing("Grid", |ui| {
                            if ui.checkbox(&mut cfg.show_grid, "Show Grid").changed() {
                                resp.config_changed = true;
                                resp.grid_rebuild = true;
                            }
                            if cfg.show_grid {
                                if ui.add(egui::Slider::new(&mut cfg.grid_size, 0.5..=20.0).text("Size")).changed() {
                                    resp.config_changed = true;
                                    resp.grid_rebuild = true;
                                }
                                if ui.add(egui::Slider::new(&mut cfg.grid_divisions, 2..=50).text("Divisions")).changed() {
                                    resp.config_changed = true;
                                    resp.grid_rebuild = true;
                                }
                                ui.horizontal(|ui| {
                                    ui.label("Grid Color:");
                                    if ui.color_edit_button_srgb(&mut cfg.grid_color).changed() {
                                        resp.config_changed = true;
                                        resp.grid_rebuild = true;
                                    }
                                });
                            }

                            ui.horizontal(|ui| {
                                ui.label("Model Position:");
                                egui::ComboBox::from_id_salt("model_position")
                                    .selected_text(match cfg.model_position {
                                        ModelPosition::Above => "Above",
                                        ModelPosition::Center => "Center",
                                        ModelPosition::Below => "Below",
                                    })
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_value(&mut cfg.model_position, ModelPosition::Above, "Above").changed() {
                                            resp.config_changed = true;
                                            resp.position_changed = true;
                                        }
                                        if ui.selectable_value(&mut cfg.model_position, ModelPosition::Center, "Center").changed() {
                                            resp.config_changed = true;
                                            resp.position_changed = true;
                                        }
                                        if ui.selectable_value(&mut cfg.model_position, ModelPosition::Below, "Below").changed() {
                                            resp.config_changed = true;
                                            resp.position_changed = true;
                                        }
                                    });
                            });
                        });

                        let r_axes = ui.collapsing("Coordinate Axes", |ui| {
                            ui.label("Axes Visibility:");
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut cfg.show_axes[0], "X").changed() {
                                    resp.config_changed = true;
                                    resp.axes_rebuild = true;
                                }
                                if ui.checkbox(&mut cfg.show_axes[1], "Y").changed() {
                                    resp.config_changed = true;
                                    resp.axes_rebuild = true;
                                }
                                if ui.checkbox(&mut cfg.show_axes[2], "Z").changed() {
                                    resp.config_changed = true;
                                    resp.axes_rebuild = true;
                                }
                            });
                            if ui.checkbox(&mut cfg.show_axis_direction, "Axis Direction Arrows").changed() {
                                resp.config_changed = true;
                                resp.axes_rebuild = true;
                            }
                        });

                        let r_colors = ui.collapsing("Colors", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Background:");
                                if ui.color_edit_button_srgb(&mut cfg.background).changed() {
                                    resp.config_changed = true;
                                    resp.bg_changed = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Object:");
                                if ui.color_edit_button_srgb(&mut cfg.object_color).changed() {
                                    resp.config_changed = true;
                                    resp.obj_color_changed = true;
                                }
                            });
                        });

                        let r_controls = ui.collapsing("Controls & Scale", |ui| {
                            if ui.checkbox(&mut cfg.show_dummy_box, "Show Dummy Box").changed() {
                                resp.config_changed = true;
                                resp.dummy_box_changed = true;
                            }
                            if ui.add(egui::Slider::new(&mut cfg.object_scale, 0.1..=10.0).text("Scale")).changed() {
                                resp.config_changed = true;
                                resp.scale_changed = true;
                            }
                            if ui.checkbox(&mut cfg.smooth_orbit, "Smooth Orbit").changed() {
                                resp.config_changed = true;
                                resp.controls_changed = true;
                            }
                            if ui.checkbox(&mut cfg.invert_scroll, "Invert Zoom").changed() {
                                resp.config_changed = true;
                                resp.controls_changed = true;
                            }
                            if ui.add(egui::Slider::new(&mut cfg.scroll_speed, 0.001..=0.05).text("Zoom Speed")).changed() {
                                resp.config_changed = true;
                                resp.controls_changed = true;
                            }
                        });

                        let should_be_wide = r_grid.body_response.is_some()
                            || r_axes.body_response.is_some()
                            || r_colors.body_response.is_some()
                            || r_controls.body_response.is_some();

                        if should_be_wide != is_wide {
                            ctx.data_mut(|d| d.insert_temp(wide_mode_id, should_be_wide));
                            ctx.request_repaint();
                        }

                        ui.separator();

                        if menu_button(ui, "Reset Camera").clicked() {
                            resp.reset_camera_requested = true;
                            resp.close_menu_requested = true;
                        }

                        if menu_button(ui, "Reset Defaults").clicked() {
                            resp.reset_defaults_requested = true;
                        }

                        ui.separator();

                        if menu_button(ui, "Exit").clicked() {
                            resp.close_app_requested = true;
                        }
                    });
                });
        });

    if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
        && let Some(pos) = ctx.input(|i| i.pointer.interact_pos())
        && !area_resp.response.rect.contains(pos)
        && !ctx.is_pointer_over_egui()
    {
        resp.close_menu_requested = true;
    }

    resp
}
