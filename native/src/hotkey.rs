use crate::store::{save_settings, settings};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

pub fn toggled(on: bool) -> &'static str {
    if on { "Overlays affichés (Ctrl+Maj+O pour les cacher)" } else { "Overlays cachés (Ctrl+Maj+O pour les remettre)" }
}

fn toggle() {
    let mut s = settings();
    s.ov_enabled = !s.ov_enabled;
    save_settings(&s);
    let on = s.ov_enabled;
    crate::with_app(|a| {
        a.toast(toggled(on));
        crate::page_misc::render_overlays(a);
    });
}

pub fn start() {
    std::thread::spawn(|| unsafe {
        if RegisterHotKey(std::ptr::null_mut(), 1, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, u32::from(b'O')) == 0 {
            return;
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.message == WM_HOTKEY {
                let _ = slint::invoke_from_event_loop(toggle);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_toast_says_how_to_undo() {
        assert!(toggled(true).contains("cacher") && toggled(false).contains("remettre"));
    }
}
