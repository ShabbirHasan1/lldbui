use egui::{ScrollArea, Ui};

use crate::app::{App, BreakpointsTab};

pub fn add(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut app.breakpoints_tab,
            BreakpointsTab::Breakpoints,
            "breakpoints",
        );
        ui.selectable_value(
            &mut app.breakpoints_tab,
            BreakpointsTab::Watchpoints,
            "watchpoints",
        );
    });
    ScrollArea::both()
        .id_source("breakpoints")
        .show(ui, |ui| match app.breakpoints_tab {
            BreakpointsTab::Breakpoints => egui::Grid::new(ui.next_auto_id())
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    for (id, file, line) in app.debug_session.breakpoint_locations().iter() {
                        ui.label(format!("{}", id));
                        ui.label(file);
                        ui.label(format!("{}", line));
                        ui.end_row();
                    }
                }),
            BreakpointsTab::Watchpoints => egui::Grid::new(ui.next_auto_id())
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    for watchpoint in app.debug_session.target.as_ref().unwrap().watchpoints() {
                        ui.label(format!("{}", watchpoint.id()));
                        ui.label(format!("{:#x}", watchpoint.watch_address()));
                        ui.end_row()
                    }
                }),
        });
}
