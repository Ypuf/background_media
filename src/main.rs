#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::RefCell, rc::Rc};

use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};

mod overlay;
mod platform;

slint::include_modules!();

fn is_gif(p: &std::path::Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif"))
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        use slint::winit_030::winit::{
            event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11,
        };
        let mut event_loop = EventLoop::with_user_event();
        event_loop.with_x11();
        slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .with_winit_event_loop_builder(event_loop)
            .select()
            .unwrap();
    }

    let ui = AppWindow::new().unwrap();
    let overlays = Rc::new(RefCell::new(overlay::Overlays::default()));

    let gif_chosen = {
        let overlays = overlays.clone();
        let asweak = ui.as_weak();
        move |p: &std::path::Path| {
            let on_top = asweak.upgrade().is_some_and(|ui| ui.get_on_top());
            if let Err(e) = overlay::open(&overlays, p, on_top) {
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

    let weak = ui.as_weak();
    slint::spawn_local(async move {
        let Some(ui) = weak.upgrade() else { return };
        let Ok(w) = ui.window().winit_window().await else {
            return;
        };
        if let Some(monitor) = w.current_monitor() {
            let size = monitor.size().to_logical::<f32>(monitor.scale_factor());
            ui.set_scale((size.width / 1920.0).min(size.height / 1080.0));
        }
    })
    .unwrap();

    let tray = Tray::new().unwrap();
    tray.on_quit(|| slint::quit_event_loop().unwrap());
    tray.on_clickthrough_toggled({
        let overlays = overlays.clone();
        move |on| overlays.borrow().clickthrough.set(on)
    });
    tray.on_open({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.show().unwrap();
            }
        }
    });

    ui.window()
        .on_close_requested(|| slint::CloseRequestResponse::HideWindow);

    ui.show().unwrap();
    slint::run_event_loop_until_quit().unwrap();
}
