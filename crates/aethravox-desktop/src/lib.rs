pub mod api;
pub mod app;
pub mod theme;
pub mod ui;

use aethravox_core::DeviceProfile;

#[cfg(not(target_arch = "wasm32"))]
fn detect_initial_device_profile() -> DeviceProfile {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--tv" => return DeviceProfile::Tv,
            "--pdi" | "--whiteboard" => return DeviceProfile::Pdi,
            "--mobile" | "--phone" => return DeviceProfile::Mobile,
            "--desktop" => return DeviceProfile::Desktop,
            _ => {}
        }
    }
    if let Ok(val) = std::env::var("AETHRAVOX_DEVICE") {
        let parsed = DeviceProfile::from_str_opt(&val);
        if parsed != DeviceProfile::Auto {
            return parsed;
        }
    }
    DeviceProfile::Auto
}

#[cfg(not(target_arch = "wasm32"))]
fn base_native_options(profile: DeviceProfile) -> eframe::NativeOptions {
    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default().with_title("Aethravox Studios");

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        viewport = viewport
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([360.0, 520.0]);
        if matches!(profile, DeviceProfile::Tv | DeviceProfile::Pdi) {
            viewport = viewport.with_fullscreen(true);
        }
    }

    let _ = profile;
    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_with_options(
    options: eframe::NativeOptions,
    initial_profile: DeviceProfile,
) -> eframe::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = rt.handle().clone();

    eframe::run_native(
        "Aethravox Studios",
        options,
        Box::new(move |cc| {
            Ok(Box::new(app::AethravoxApp::new_with_profile(
                cc,
                tx,
                rx,
                handle,
                initial_profile,
            )))
        }),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_desktop() -> eframe::Result<()> {
    let profile = detect_initial_device_profile();
    let options = base_native_options(profile);
    run_with_options(options, profile)
}

#[cfg(target_arch = "wasm32")]
pub fn run_web() {
    use wasm_bindgen::JsCast;

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .expect("No se encontró el DOM del navegador");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("No se encontró el canvas #the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("#the_canvas_id no es un HtmlCanvasElement");

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let handle = api::AsyncRt::default();

        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    Ok(Box::new(app::AethravoxApp::new_with_profile(
                        cc,
                        tx,
                        rx,
                        handle,
                        DeviceProfile::Auto,
                    )))
                }),
            )
            .await
            .expect("No se pudo iniciar eframe en WebAssembly");
    });
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let profile = detect_initial_device_profile();
    let mut options = base_native_options(profile);
    options.event_loop_builder = Some(Box::new(move |builder| {
        builder.with_android_app(android_app);
    }));
    let _ = run_with_options(options, profile);
}

#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn aethravox_ios_main() {
    let profile = DeviceProfile::Mobile;
    let options = base_native_options(profile);
    let _ = run_with_options(options, profile);
}

