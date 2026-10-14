use std::{
    cell::{Cell, RefCell},
    error::Error,
    fs::File,
    io::BufReader,
    path::Path,
    rc::{Rc, Weak},
    time::Duration,
};

use image::{AnimationDecoder, codecs::gif::GifDecoder};
use slint::{
    ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode,
    winit_030::{
        EventResult, WinitWindowAccessor,
        winit::{
            event::{ElementState, MouseButton, WindowEvent},
            window::WindowLevel,
        },
    },
};

use crate::{GifWindow, platform};

struct Overlay {
    id: u64,
    _window: GifWindow,
    _timer: Rc<Timer>,
}

#[derive(Default)]
pub struct Overlays {
    next_id: u64,
    list: Vec<Overlay>,
}

pub fn open(overlays: &Rc<RefCell<Overlays>>, path: &Path) -> Result<(), Box<dyn Error>> {
    let frames = load_frames(path)?;
    let window = GifWindow::new()?;
    window.set_frame(frames[0].0.clone());
    window.show()?;

    window.window().with_winit_window(|w| {
        w.set_window_level(WindowLevel::AlwaysOnBottom);
        platform::make_overlay(w);
    });

    let timer = Rc::new(Timer::default());
    if frames.len() > 1 {
        animate(&timer, &window, frames);
    }

    let mut o = overlays.borrow_mut();
    let id = o.next_id;
    o.next_id += 1;
    giffing_input(&window, Rc::downgrade(overlays), id);
    o.list.push(Overlay {
        id,
        _window: window,
        _timer: timer,
    });
    Ok(())
}

fn giffing_input(window: &GifWindow, overlays: Weak<RefCell<Overlays>>, id: u64) {
    let super_held = Cell::new(false);

    window.window().on_winit_window_event(move |w, event| {
        match event {
            WindowEvent::ModifiersChanged(m) => super_held.set(m.state().super_key()),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } if super_held.get() || platform::super_down() => {
                platform::fuck_off_start_menu();
                match button {
                    MouseButton::Left => {
                        w.with_winit_window(|w| w.drag_window().ok());
                    }
                    MouseButton::Right => {
                        let overlays = overlays.clone();
                        Timer::single_shot(Duration::ZERO, move || {
                            if let Some(overlays) = overlays.upgrade() {
                                overlays.borrow_mut().list.retain(|o| o.id != id);
                            }
                        });
                    }
                    _ => {}
                }
                return EventResult::PreventDefault;
            }
            _ => {}
        }
        EventResult::Propagate
    });
}

fn animate(timer: &Rc<Timer>, window: &GifWindow, frames: Vec<(Image, Duration)>) {
    let weak_window = window.as_weak();
    let weak_timer = Rc::downgrade(timer);
    let mut index = 0;
    timer.start(TimerMode::Repeated, frames[0].1, move || {
        index = (index + 1) % frames.len();
        let (frame, delay) = &frames[index];
        if let Some(window) = weak_window.upgrade() {
            window.set_frame(frame.clone());
        }
        if let Some(timer) = weak_timer.upgrade() {
            timer.set_interval(*delay);
        }
    });
}

fn load_frames(path: &Path) -> Result<Vec<(Image, Duration)>, Box<dyn Error>> {
    let decoder = GifDecoder::new(BufReader::new(File::open(path)?))?;
    let mut frames = Vec::new();
    for frame in decoder.into_frames() {
        let frame = frame?;
        let (num, den) = frame.delay().numer_denom_ms();
        let ms = num / den.max(1);
        let delay = Duration::from_millis(if ms < 20 { 100 } else { ms.into() });
        let buffer = frame.into_buffer();
        let pixels = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
            buffer.as_raw(),
            buffer.width(),
            buffer.height(),
        );
        frames.push((Image::from_rgba8(pixels), delay));
    }
    if frames.is_empty() {
        return Err("the GIF has no frames".into());
    }
    Ok(frames)
}
