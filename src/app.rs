use egui::{
    widgets, Align, CentralPanel, CollapsingHeader, Layout, RichText, ScrollArea, SidePanel,
    TopBottomPanel, Ui,
};
use egui_extras::syntax_highlighting::{highlight, CodeTheme};
use lldb::{SBDebugger, SBEvent, SBTarget, SBValue};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum ConsoleTab {
    Console,
    Stdout,
    Stderr,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum VariableTab {
    Locals,
    Statics,
    Arguments,
    Registers,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(default)]
pub struct App {
    console_tab: ConsoleTab,
    variables_tab: VariableTab,
    #[serde(skip)]
    target: Option<SBTarget>,
    #[serde(skip)]
    selected_thread_id: u64,
    #[serde(skip)]
    selected_frame_id: u32,
    #[serde(skip)]
    sources: HashMap<String, String>,
    #[serde(skip)]
    selected_source: Option<String>,
    #[serde(skip)]
    source_language: HashMap<String, String>,
    #[serde(skip)]
    selected_line: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            console_tab: ConsoleTab::Console,
            variables_tab: VariableTab::Locals,
            target: None,
            selected_thread_id: 0,
            selected_frame_id: 0,
            selected_source: None,
            sources: HashMap::new(),
            source_language: HashMap::new(),
            selected_line: 0,
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, target: SBTarget) -> Self {
        let mut app = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            App::default()
        };

        app.target = Some(target);
        app
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.target.is_none() {
            return;
        }
        let debugger = self
            .target
            .as_ref()
            .expect("target should be set")
            .debugger();
        let process = self
            .target
            .as_ref()
            .expect("target should be set")
            .process();
        let executable = self
            .target
            .as_ref()
            .expect("target should be set")
            .executable();

        let thread = process.selected_thread();
        let frame = thread.selected_frame();
        self.selected_thread_id = thread.thread_id();
        self.selected_frame_id = thread.selected_frame().frame_id();

        // without polling the events process.state() never changes???
        let event = SBEvent::new();
        while debugger.listener().get_next_event(&event) {
            // println!("{:?}", event);
        }

        TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    widgets::global_dark_light_mode_switch(ui);
                    ui.separator();
                    ui.label(RichText::new(SBDebugger::version()).small());
                    ui.separator();
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        egui::warn_if_debug_build(ui);
                    });
                });
            });

        SidePanel::left("left_panel")
            .resizable(true)
            .show(ctx, |ui| {
                egui::Grid::new("target")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        if let Some(executable) = executable {
                            ui.label("Target:");
                            ui.label(executable.filename().to_string());
                            ui.end_row();
                        }

                        ui.label("Process state:");
                        ui.label(format!("{:?}", process.state()));
                        ui.end_row();

                        ui.label("PID:");
                        ui.label(format!("{:?}", process.process_id()));
                        ui.end_row();
                    });
                if process.is_running() {
                    if ui.button("Stop").clicked() {
                        process.stop().unwrap();
                    }
                } else if process.is_stopped() && ui.button("Run").clicked() {
                    process.continue_execution().unwrap();
                }
            });
        SidePanel::right("right_panel")
            .resizable(true)
            .min_width(250.0)
            .show(ctx, |ui| {
                TopBottomPanel::top("threads")
                    .resizable(true)
                    .min_height(150.0)
                    .show_inside(ui, |ui| {
                        ui.label("threads");
                        ScrollArea::vertical()
                            .id_source("threads")
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                egui::Grid::new("threads")
                                    .num_columns(1)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        for thread in process.threads() {
                                            if !thread.is_valid() {
                                                continue;
                                            }
                                            let mut label = format!(
                                                "{} {}",
                                                thread.thread_id(),
                                                thread.name().unwrap_or("")
                                            );
                                            if let Some(queue) = thread.queue() {
                                                label.push_str(&format!(" queue={}", queue.name()));
                                            }
                                            if thread.is_stopped() {
                                                label.push_str(&format!(
                                                    " stop_reason={:?}",
                                                    thread.stop_reason()
                                                ));
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut self.selected_thread_id,
                                                    thread.thread_id(),
                                                    label,
                                                )
                                                .clicked()
                                            {
                                                process.set_selected_thread(&thread);
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                TopBottomPanel::top("frames")
                    .resizable(true)
                    .min_height(150.0)
                    .show_inside(ui, |ui| {
                        ui.label("frames");
                        ScrollArea::vertical()
                            .id_source("frames")
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                ScrollArea::horizontal().auto_shrink(false).show(ui, |ui| {
                                    egui::Grid::new("frames").num_columns(2).striped(true).show(
                                        ui,
                                        |ui| {
                                            for frame in thread.frames() {
                                                if !frame.is_valid() {
                                                    continue;
                                                }
                                                let function = frame.function();
                                                let compile_unit = frame.compile_unit();

                                                if function.is_valid() {
                                                    if ui
                                                        .selectable_value(
                                                            &mut self.selected_frame_id,
                                                            frame.frame_id(),
                                                            function.display_name(),
                                                        )
                                                        .clicked()
                                                    {
                                                        thread.set_selected_frame(frame.frame_id());

                                                        if let Some(line_entry) = frame.line_entry()
                                                        {
                                                            // TODO: use proper mapping
                                                            let mut language = format!(
                                                                "{:?}",
                                                                compile_unit.language()
                                                            );
                                                            if language == "C99"
                                                                || language == "C11"
                                                            {
                                                                language = "C".to_string();
                                                            }

                                                            let path: PathBuf = [
                                                                line_entry.filespec().directory(),
                                                                line_entry.filespec().filename(),
                                                            ]
                                                            .iter()
                                                            .collect();

                                                            let fs = line_entry.filespec();

                                                            if fs.exists() {
                                                                let key = path
                                                                    .clone()
                                                                    .into_os_string()
                                                                    .into_string()
                                                                    .unwrap();

                                                                self.sources.insert(
                                                                    key.clone(),
                                                                    read_to_string(&path).unwrap(),
                                                                );
                                                                self.source_language
                                                                    .insert(key.clone(), language);
                                                                self.selected_source = Some(key);
                                                                self.selected_line =
                                                                    line_entry.line();
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    ui.label(
                                                        frame.display_function_name().unwrap_or(""),
                                                    );
                                                }
                                                if let Some(line_entry) = frame.line_entry() {
                                                    let path: PathBuf = [
                                                        line_entry.filespec().directory(),
                                                        line_entry.filespec().filename(),
                                                    ]
                                                    .iter()
                                                    .collect();
                                                    ui.label(format!(
                                                        "{}:{}",
                                                        path.display(),
                                                        line_entry.line(),
                                                    ));
                                                } else {
                                                    ui.label("");
                                                }
                                                ui.end_row();
                                            }
                                        },
                                    );
                                });
                            });
                    });
                CentralPanel::default().show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.variables_tab, VariableTab::Locals, "locals");
                        ui.selectable_value(
                            &mut self.variables_tab,
                            VariableTab::Statics,
                            "statics",
                        );
                        ui.selectable_value(
                            &mut self.variables_tab,
                            VariableTab::Arguments,
                            "arguments",
                        );
                        ui.selectable_value(
                            &mut self.variables_tab,
                            VariableTab::Registers,
                            "registers",
                        );
                    });
                    ScrollArea::vertical()
                        .id_source("variables")
                        .auto_shrink(false)
                        .show(ui, |ui| match self.variables_tab {
                            VariableTab::Locals => {
                                render_values(ui, frame.locals().iter());
                            }
                            VariableTab::Statics => {
                                render_values(ui, frame.statics().iter());
                            }
                            VariableTab::Arguments => {
                                render_values(ui, frame.arguments().iter());
                            }
                            VariableTab::Registers => {
                                render_values(ui, frame.registers().iter());
                            }
                        });
                });
            });
        TopBottomPanel::bottom("console_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.console_tab, ConsoleTab::Console, "console");
                    ui.selectable_value(&mut self.console_tab, ConsoleTab::Stdout, "stdout");
                    ui.selectable_value(&mut self.console_tab, ConsoleTab::Stderr, "stderr");
                });
                ui.label("foo");
            });
        CentralPanel::default().show(ctx, |ui| {
            let theme = &CodeTheme::from_style(ui.style());
            ui.horizontal(|ui| {
                for path in self.sources.keys() {
                    if ui
                        .selectable_value(&mut self.selected_source, Some(path.to_string()), path)
                        .clicked()
                    {
                        self.selected_line = 0;
                    };
                }
            });
            ui.separator();

            if let Some(path) = &self.selected_source {
                let language = self.source_language.get(path).unwrap();
                let code = self.sources.get(path).unwrap();
                ScrollArea::horizontal()
                    .id_source(path)
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_source(path)
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                let mut i = 0;
                                egui::Grid::new("source").num_columns(2).show(ui, |ui| {
                                    for line in code.lines() {
                                        i += 1;
                                        ui.label(format!("{}", i));
                                        let layout_job =
                                            highlight(ui.ctx(), theme, &line, &language);
                                        let response =
                                            ui.add(egui::Label::new(layout_job).selectable(true));
                                        if i == self.selected_line {
                                            response.scroll_to_me(Some(egui::Align::Center));
                                        }
                                        ui.end_row();
                                    }
                                });
                            });
                    });
            }
        });
    }
}

fn render_values(ui: &mut Ui, values: impl Iterator<Item = SBValue>) {
    egui::Grid::new(ui.next_auto_id())
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            for v in values {
                if v.children().count() > 0 {
                    CollapsingHeader::new(v.name().expect("name should be present"))
                        .id_source(ui.next_auto_id())
                        .show(ui, |ui| {
                            render_values(ui, v.children());
                        });
                } else {
                    render_value(ui, &v);
                }
                ui.end_row();
            }
        });
}

fn render_value(ui: &mut Ui, value: &SBValue) {
    if let Some(name) = value.name() {
        ui.label(name);
    }
    if let Some(type_name) = value.display_type_name() {
        ui.label(type_name);
    }
    if let Some(value) = value.value() {
        ui.label(value);
    }
}
