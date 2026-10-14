#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
pub use other::*;

#[cfg(windows)]
mod windows {
    use slint::winit_030::winit::{
        platform::windows::WindowExtWindows,
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
        window::Window,
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, SendInput, VK_LWIN,
                VK_RWIN,
            },
            Shell::{DefSubclassProc, SetWindowSubclass},
            WindowsAndMessaging::{
                GWL_EXSTYLE, GetWindowLongPtrW, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE,
                SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WINDOWPOS,
                WM_WINDOWPOSCHANGING, WS_EX_NOACTIVATE,
            },
        },
    };

    pub fn make_overlay(window: &Window) {
        window.set_skip_taskbar(true);
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        let hwnd = handle.hwnd.get() as HWND;
        unsafe {
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_NOACTIVATE as isize);
            SetWindowSubclass(hwnd, Some(keep_at_bottom), 1, 0);
            SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    unsafe extern "system" fn keep_at_bottom(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if msg == WM_WINDOWPOSCHANGING {
            let pos = unsafe { &mut *(lparam as *mut WINDOWPOS) };
            if pos.flags & SWP_NOZORDER == 0 {
                pos.hwndInsertAfter = HWND_BOTTOM;
            }
        }
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }

    pub fn super_down() -> bool {
        unsafe { GetAsyncKeyState(VK_LWIN.into()) < 0 || GetAsyncKeyState(VK_RWIN.into()) < 0 }
    }

    pub fn fuck_off_start_menu() {
        let mut inputs: [INPUT; 2] = unsafe { std::mem::zeroed() };
        for (input, flags) in inputs.iter_mut().zip([0, KEYEVENTF_KEYUP]) {
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki.wVk = 0xE8;
            input.Anonymous.ki.dwFlags = flags;
        }
        unsafe { SendInput(2, inputs.as_ptr(), size_of::<INPUT>() as i32) };
    }
}

#[cfg(not(windows))]
mod other {
    use slint::winit_030::winit::window::Window;

    pub fn make_overlay(_window: &Window) {}

    pub fn super_down() -> bool {
        false
    }

    pub fn fuck_off_start_menu() {}
}
