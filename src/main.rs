//! quoquendo — the Loquendo 7 (LTTS7) voices without the original engine:
//! a window (desktop and browser) and, on the desktop, a command line.
//!
//! The app never guesses a language from the text: it reads the loaded voice's
//! own `MotherTongue`, runs that language's front end, and shows which symbols
//! the voice's inventory actually contains.

mod app;
mod audio;
mod catalog;
mod store;

#[cfg(not(target_arch = "wasm32"))]
mod cli;
#[cfg(not(target_arch = "wasm32"))]
mod dev;
#[cfg(not(target_arch = "wasm32"))]
mod install;
#[cfg(not(target_arch = "wasm32"))]
mod mirror;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use eframe::egui;
    let opts = cli::parse_args();
    if let Some(code) = cli::run(&opts) {
        std::process::exit(code);
    }
    let mut native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 860.0]).with_title("Quoquendo"),
        ..Default::default()
    };
    if opts.screenshot.is_some() {
        // A screenshot must not steal focus, and must not stop drawing when it
        // is not focused.
        native.viewport = native.viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            native.viewport = native.viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            native.event_loop_builder = Some(Box::new(|b| {
                b.with_activate_ignoring_other_apps(false);
            }));
        }
        native.run_and_return = false;
        // the picture shows the defaults, not this computer's saved settings
        native.persist_window = false;
    }
    let launch = app::Launch {
        dev: opts.dev,
        screenshot: opts.screenshot.clone(),
    };
    eframe::run_native("quoquendo", native, Box::new(|cc| Ok(Box::new(app::App::new(cc, launch)))))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("qq_canvas"))
            .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("no <canvas id=\"qq_canvas\">");
        let started = eframe::WebRunner::new()
            .start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(app::App::new(cc, app::Launch::default())))))
            .await;
        if let Some(loading) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("qq_loading")) {
            match started {
                Ok(()) => loading.remove(),
                Err(e) => loading.set_inner_html(&format!("Quoquendo could not start: {e:?}")),
            }
        }
    });
}
