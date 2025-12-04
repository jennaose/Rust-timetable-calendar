mod database;

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::env;

use crate::database::{insert_entry, load_entries, delete_entry};

/// Connection string environment variable name
const DATABASE_URL_ENV: &str = "DATABASE_URL";

/// A single timetable entry
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TimetableEntry {
    pub activity: String,
    pub time: String,
    pub day: String,
    pub notes: String,
}

/// The main app state
#[derive(Serialize, Deserialize, Default)]
pub struct TimetableApp {
    pub entries: Vec<TimetableEntry>,

    /// Temporary fields for user input (NOT saved)
    #[serde(skip)]
    pub new_entry: TimetableEntry,

    /// Day filter for viewing entries
    #[serde(skip)]
    pub selected_day_filter: String,

    /// DB connection string (not serialized)
    #[serde(skip)]
    pub db_url: String,
}

impl TimetableApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Allow .env during local dev:
        let _ = dotenvy::dotenv();

        // Read DATABASE_URL from environment
        dotenvy::dotenv().ok();

        // Read DATABASE_URL strictly from the environment
        let db_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in the .env file or environment");


        // Attempt to load entries from DB; fall back to default on error.
        let entries = match load_entries(&db_url) {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Warning: failed to load entries from DB: {:?}", err);
                Vec::new()
            }
        };

        Self {
            entries,
            new_entry: TimetableEntry::default(),
            selected_day_filter: String::new(),
            db_url,
        }
    }

    /// Clears input fields after successfully adding an entry
    fn clear_new_entry_fields(&mut self) {
        self.new_entry = TimetableEntry::default();
    }
}

impl eframe::App for TimetableApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // We don't use eframe local storage now; DB persists data.
        // But implement `save` as a no-op to satisfy trait.
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("Timetable Scheduler (Postgres-backed)");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Add a New Schedule Item");
            ui.separator();

            // GRID FOR INPUT FIELDS
            egui::Grid::new("input_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Activity:");
                    ui.text_edit_singleline(&mut self.new_entry.activity);
                    ui.end_row();

                    ui.label("Time:");
                    ui.text_edit_singleline(&mut self.new_entry.time);
                    ui.end_row();

                    ui.label("Day:");
                    egui::ComboBox::from_label("Select Day")
                        .selected_text(if self.new_entry.day.is_empty() {
                            "Choose..."
                        } else {
                            &self.new_entry.day
                        })
                        .show_ui(ui, |ui| {
                            let days = [
                                "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
                                "Sunday",
                            ];
                            for day in days.iter() {
                                ui.selectable_value(&mut self.new_entry.day, day.to_string(), *day);
                            }
                        });
                    ui.end_row();

                    ui.label("Notes:");
                    ui.text_edit_singleline(&mut self.new_entry.notes);
                    ui.end_row();
                });

            ui.separator();

            // BUTTON TO ADD ENTRY
            if ui.button("Add Schedule Item").clicked() {
                if !self.new_entry.activity.is_empty()
                    && !self.new_entry.time.is_empty()
                    && !self.new_entry.day.is_empty()
                {
                    // Push locally first (so UI updates immediately)
                    self.entries.push(self.new_entry.clone());

                    // Save to DB (log errors but keep UI responsive)
                    if let Err(e) = insert_entry(&self.db_url, &self.entries.last().unwrap()) {
                        eprintln!("DB insert error: {:?}", e);
                    }

                    self.clear_new_entry_fields();
                } else {
                    ui.colored_label(egui::Color32::RED, "❌ Error: Activity, Time, and Day are required.");
                }
            }

            ui.separator();

            // FILTER COMBOBOX
            ui.heading("View Schedule");
            egui::ComboBox::from_label("Filter by Day")
                .selected_text(if self.selected_day_filter.is_empty() {
                    "All Days"
                } else {
                    &self.selected_day_filter
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_day_filter, "".to_string(), "All Days");

                    let days = [
                        "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
                    ];

                    for day in days.iter() {
                        ui.selectable_value(&mut self.selected_day_filter, day.to_string(), *day);
                    }
                });

            ui.separator();

            // SCROLL AREA FOR ENTRIES
            egui::ScrollArea::vertical().show(ui, |ui| {
                let entries_to_display: Vec<(usize, TimetableEntry)> = self
                    .entries
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter(|(_, entry)| {
                        self.selected_day_filter.is_empty() || entry.day == self.selected_day_filter
                    })
                    .collect();

                for (index, entry) in &entries_to_display {
                    egui::CollapsingHeader::new(format!("{} - {}", entry.activity, entry.time))
                        .id_salt(*index)
                        .show(ui, |ui| {
                            ui.label(format!("Day: {}", entry.day));

                            if !entry.notes.is_empty() {
                                ui.label(format!("Notes: {}", entry.notes));
                            }

                            ui.separator();

                            if ui.button("Delete this item").clicked() {
                                // Delete from DB first
                                if let Err(e) =
                                    delete_entry(&self.db_url, &entry.activity, &entry.time, &entry.day)
                                {
                                    eprintln!("DB delete error: {:?}", e);
                                }
                                // Remove from local Vec by index
                                // Note: this index is the original index in the entries Vec (because we used enumerate()+cloned())
                                self.entries.remove(*index);
                            }
                        });

                    ui.separator();
                }

                if entries_to_display.is_empty() {
                    ui.label("No entries match the selected filter.");
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0])
            .with_min_inner_size([480.0, 380.0])
            .with_title("Rust Timetable App (Postgres)"),
        ..Default::default()
    };

    eframe::run_native(
        "Timetable Scheduler (Postgres)",
        options,
        Box::new(|cc| Ok(Box::new(TimetableApp::new(cc)))),
    )
}
