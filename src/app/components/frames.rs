use std::sync::atomic::Ordering;

use egui::{ScrollArea, Ui};

use crate::app::App;

pub fn add(app: &mut App, ui: &mut Ui) {
    let state = app.debug_session.state.lock().unwrap();

    ScrollArea::both()
        .id_source("frames")
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Grid::new("frames")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    let mut selected_frame_id = state.selected_frame_id;
                    for frame in state.frames {
                        if let Some(line_entry) = frame.line_entry() {
                            ui.label(format!(
                                "{}:{}",
                                line_entry.filespec().filename(),
                                line_entry.line(),
                            ));
                        } else {
                            ui.label("");
                        }

                        if ui
                            .selectable_value(
                                &mut selected_frame_id,
                                frame.frame_id(),
                                frame.display_function_name().unwrap(),
                            )
                            .clicked()
                        {
                            app.debug_session.select_frame(frame.frame_id());
                            // (ds): lldb does not publish frame changed events
                            //       when the frame is changed via the API.
                            //       So we need to manually trigger a redraw.
                            //       https://stackoverflow.com/questions/41798498/how-to-use-lldb-trace-thread-and-python-sbthread-ebroadcastbitselectedframechang/41815283#41815283
                            app.source_view_changed.store(true, Ordering::Relaxed)
                        }
                        ui.end_row();
                    }
                });
        });
}
