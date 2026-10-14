use std::{cell::RefCell, rc::Rc};

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};

mod overlay;
mod platform;

slint::include_modules!();

fn is_gif(p: &std::path::Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif"))
}

fn main() {
    let ui = AppWindow::new().unwrap();
    let overlays = Rc::new(RefCell::new(overlay::Overlays::default()));

    let gif_chosen = {
        let overlays = overlays.clone();
        move |p: &std::path::Path| {
            if let Err(e) = overlay::open(&overlays, p) {
                rfd::MessageDialog::new()
                    .set_title("Couldn't open GIF")
                    .set_description(format!("{}: {e}", p.display()))
                    .show();
            }
        }
    };

    ui.on_add_clicked({
        let gif_chosen = gif_chosen.clone();
        move || {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("GIF", &["gif"])
                .pick_file()
            {
                gif_chosen(&p);
            }
        }
    });

    let weak = ui.as_weak();
    ui.window().on_winit_window_event(move |_, event| {
        let ui = weak.unwrap();
        match event {
            WindowEvent::HoveredFile(p) if is_gif(p) => ui.set_hovering(true),
            WindowEvent::HoveredFileCancelled => ui.set_hovering(false),
            WindowEvent::DroppedFile(p) => {
                ui.set_hovering(false);
                if is_gif(p) {
                    gif_chosen(p);
                }
            }
            _ => {}
        }
        EventResult::Propagate
    });

    ui.window().on_close_requested(|| {
        slint::quit_event_loop().unwrap();
        slint::CloseRequestResponse::HideWindow
    });

    ui.run().unwrap();
}
