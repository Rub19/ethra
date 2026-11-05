slint::include_modules!();
use slint::winit_030::{winit, WinitWindowAccessor};
fn main() {
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|a| a)
        .select().unwrap();
    let ui = AppWindow::new().unwrap();
    ui.global::<Api>().on_img(|_,_| slint::Image::default());
    let ov = Overlay::new().unwrap();
    ov.global::<Api>().on_img(|_,_| slint::Image::default());
    let svg = slint::Image::load_from_svg_data(b"<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'><rect width='10' height='10' fill='red'/></svg>");
    println!("svg {:?}", svg.is_ok());
    ov.show().unwrap();
    ui.show().unwrap();
    ov.window().with_winit_window(|w: &winit::window::Window| {
        println!("hittest {:?}", w.set_cursor_hittest(false));
    });
    let t = slint::Timer::default();
    let (a, b) = (ui.as_weak(), ov.as_weak());
    t.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(1500), move || {
        let s = a.unwrap().window().take_snapshot().unwrap();
        println!("main {}x{}", s.width(), s.height());
        let s = b.unwrap().window().take_snapshot().unwrap();
        let p = s.as_bytes();
        println!("ov {}x{} corner rgba {:?} center {:?}", s.width(), s.height(), &p[0..4], &p[((60*s.width()+300)*4) as usize..((60*s.width()+300)*4+4) as usize]);
        slint::quit_event_loop().unwrap();
    });
    slint::run_event_loop_until_quit().unwrap();
}
