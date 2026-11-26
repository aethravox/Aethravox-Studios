mod api;
mod app;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = rt.handle().clone();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Aethravox Studios")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Aethravox Studios",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::AethravoxApp::new(cc, tx, rx, handle)))),
    )
}
