use ::bevy::prelude::*;
use ::bevy_egui::{egui, EguiContexts};

#[derive(Resource)]
pub(crate) struct Cfg {
    pub debug_vecs: bool,
    pub debug_print: bool,
    pub num_circles: usize,
    pub node_starting_distance: f32,
    pub node_starting_random_offset: f32,
    pub move_speed: f32,
    pub target_centre_length: f32,
    pub target_point_length: f32,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            debug_vecs: false,
            debug_print: false,
            num_circles: 6,
            node_starting_distance: 100.,
            node_starting_random_offset: 20.,
            move_speed: 100.,
            target_centre_length: 50.,
            target_point_length: 50.,
        }
    }
}

pub(crate) fn settings(mut contexts: EguiContexts, mut cfg: ResMut<Cfg>) {
    egui::Window::new("Settings").show(contexts.ctx_mut(), |ui| {
        ui.checkbox(&mut cfg.debug_vecs, "Debug Vectors");
        ui.checkbox(&mut cfg.debug_print, "Debug Print");
        let id = ui.label("Number of Generation Circles").id;
        ui.add(egui::DragValue::new(&mut cfg.num_circles).clamp_range(1..=20))
            .labelled_by(id);
        let id = ui.label("Node Starting Distance").id;
        ui.add(egui::DragValue::new(&mut cfg.node_starting_distance).clamp_range(1.0..=10000.))
            .labelled_by(id);
        let start = cfg.node_starting_distance;
        let id = ui.label("Node Starting Random Offset").id;
        ui.add(
            egui::DragValue::new(&mut cfg.node_starting_random_offset)
                .clamp_range(0.0..=start - 1.0),
        )
        .labelled_by(id);
        let id = ui.label("Move Speed").id;
        ui.add(egui::DragValue::new(&mut cfg.move_speed).clamp_range(0.0..=10000.))
            .labelled_by(id);
        let id = ui.label("Target Length to Points").id;
        ui.add(egui::DragValue::new(&mut cfg.target_point_length).clamp_range(0.0..=10000.))
            .labelled_by(id);
        let id = ui.label("Target Length to Centre").id;
        ui.add(egui::DragValue::new(&mut cfg.target_centre_length).clamp_range(0.0..=10000.))
            .labelled_by(id);
    });
}
