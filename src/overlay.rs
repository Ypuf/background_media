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
            dpi::PhysicalPosition,
            event::{ElementState, MouseButton, WindowEvent},
            keyboard::{KeyCode, PhysicalKey},
            window::{CursorIcon, ResizeDirection, Window, WindowLevel},
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
    let ralt_held = Cell::new(false);
    let cursor = Cell::new(PhysicalPosition::new(0.0, 0.0));

    window.window().on_winit_window_event(move |w, event| {
        let held = || ralt_held.get() || platform::ralt_down();
        match event {
            WindowEvent::KeyboardInput { event, .. }
                if event.physical_key == PhysicalKey::Code(KeyCode::AltRight) =>
            {
                ralt_held.set(event.state.is_pressed())
            }
            WindowEvent::CursorMoved { position, .. } => {
                cursor.set(*position);
                w.with_winit_window(|w| {
                    let edge = held().then(|| resize_edge(w, *position)).flatten();
                    w.set_cursor(edge.map_or(CursorIcon::Default, CursorIcon::from));
                });
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } if held() => {
                match button {
                    MouseButton::Left => {
                        w.with_winit_window(|w| match resize_edge(w, cursor.get()) {
                            Some(edge) => w.drag_resize_window(edge).ok(),
                            None => w.drag_window().ok(),
                        });
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

fn resize_edge(w: &Window, pos: PhysicalPosition<f64>) -> Option<ResizeDirection> {
    let border = 8.0 * w.scale_factor();
    let size = w.inner_size();
    let left = pos.x < border;
    let right = pos.x > size.width as f64 - border;
    let top = pos.y < border;
    let bottom = pos.y > size.height as f64 - border;
    Some(match (left, right, top, bottom) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (_, true, true, _) => ResizeDirection::NorthEast,
        (true, _, _, true) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, ..) => ResizeDirection::West,
        (_, true, ..) => ResizeDirection::East,
        (_, _, true, _) => ResizeDirection::North,
        (.., true) => ResizeDirection::South,
        _ => return None,
    })
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
