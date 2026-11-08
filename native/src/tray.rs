use crate::with_app;
use slint::ComponentHandle;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

fn show_window() {
    let _ = slint::invoke_from_event_loop(|| {
        with_app(|a| {
            a.ui.window().set_minimized(false);
            let _ = a.ui.show();
        });
    });
}

pub fn start() -> Option<TrayIcon> {
    let im = image::load_from_memory(include_bytes!("../assets/logo-32.png")).ok()?.to_rgba8();
    let (w, h) = im.dimensions();
    let icon = Icon::from_rgba(im.into_raw(), w, h).ok()?;
    let open = MenuItem::new("Ouvrir Kairo", true, None);
    let quit = MenuItem::new("Quitter", true, None);
    let menu = Menu::new();
    menu.append_items(&[&open, &quit]).ok()?;
    let (open_id, quit_id) = (open.id().clone(), quit.id().clone());
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        if e.id == open_id {
            show_window();
        } else if e.id == quit_id {
            let _ = slint::invoke_from_event_loop(|| {
                let _ = slint::quit_event_loop();
            });
        }
    }));
    TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
            show_window();
        }
    }));
    TrayIconBuilder::new().with_tooltip("Kairo").with_icon(icon).with_menu(Box::new(menu)).build().ok()
}
