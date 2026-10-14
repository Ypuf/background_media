use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};

slint::include_modules!();

fn is_gif(p: &std::path::Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif"))
}

fn gif_chosen(p: &std::path::Path) {
    println!("chosen: {}", p.display());
}

fn main() {
    let ui = AppWindow::new().unwrap();

    ui.on_add_clicked(|| {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("GIF", &["gif"])
            .pick_file()
        {
            gif_chosen(&p);
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

    ui.run().unwrap();
}
