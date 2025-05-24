use eframe::{egui, App, CreationContext, NativeOptions};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod stats_sender;
use stats_sender::StatsSender;

#[derive(Default)]
struct Stats {
    pub node_id: String,
    pub cpu: f32,
    pub mem: f32,
    pub funds: f32,
}

#[derive(Default)]
struct NodeDashboard {
    stats: Arc<Mutex<Stats>>,
}

impl App for NodeDashboard {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new("🖥️ Lumaris Node Dashboard").size(32.0).strong());
                ui.add_space(20.0);

                if let Ok(stats) = self.stats.try_lock() {
                    let stats = stats.clone();

                    ui.group(|ui| {
                        ui.style_mut().spacing.item_spacing = egui::vec2(10.0, 15.0);

                        ui.label(
                            egui::RichText::new(format!("🧠 CPU Usage:   {:.1}%", stats.cpu))
                                .size(22.0)
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("💾 Memory Usage: {:.1}%", stats.mem))
                                .size(22.0)
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(format!("💰 Earnings:     ${:.2}", stats.funds))
                                .size(22.0)
                                .monospace(),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(format!("🔗 Node ID: {}", stats.node_id))
                                .size(18.0)
                                .italics(),
                        );
                    });
                } else {
                    ui.label(
                        egui::RichText::new("⏳ Waiting for updated stats...")
                            .size(20.0)
                            .italics()
                            .color(egui::Color32::LIGHT_YELLOW),
                    );
                }
            });
        });

        ctx.request_repaint_after(Duration::from_millis(1000));
    }
}

fn main() -> Result<(), eframe::Error> {
    let stats = Arc::new(Mutex::new(Stats::default()));
    
    // Start the stats sender
    let stats_sender = StatsSender::new("ws://127.0.0.1:9001", "local-node", 5000);
    stats_sender.start();

    let options = NativeOptions {
        drag_and_drop_support: false,
        initial_window_size: Some(egui::vec2(500.0, 300.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Lumaris Node Dashboard",
        options,
        Box::new(|_cc: &CreationContext| Box::new(NodeDashboard { stats })),
    )
}
