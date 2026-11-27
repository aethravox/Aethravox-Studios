pub mod api;
pub mod app;
pub mod theme;
pub mod ui;

use aethravox_core::DeviceProfile;

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

fn base_native_options(profile: DeviceProfile) -> eframe::NativeOptions {
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

pub fn run_desktop() -> eframe::Result<()> {
    let profile = detect_initial_device_profile();
    let options = base_native_options(profile);
    run_with_options(options, profile)
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
