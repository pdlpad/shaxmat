use macroquad::prelude::*;
use shaxmat_game::app::App;
use shaxmat_game::layout::{SCREEN_HEIGHT, SCREEN_WIDTH, Viewport};
use shaxmat_game::render::Renderer;

const START_SCALE: i32 = 3;

fn window_conf() -> Conf {
    Conf {
        window_title: "SHAXMAT".to_owned(),
        window_width: SCREEN_WIDTH as i32 * START_SCALE,
        window_height: SCREEN_HEIGHT as i32 * START_SCALE,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let renderer = Renderer::new();
    let mut app = App::new(miniquad::date::now() as u64);

    #[cfg(not(target_arch = "wasm32"))]
    let mut demo = demo_from_args();

    loop {
        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();
            let (x, y) = Viewport::fit(screen_width(), screen_height()).to_buffer(x, y);
            app.click(x, y);
        }
        app.update();
        renderer.draw(&app);
        renderer.present();

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(script) = &mut demo
            && !script.step(&mut app, renderer.buffer())
        {
            return;
        }
        next_frame().await;
    }
}

/// `shaxmat --demo script.txt` runs a script, see `shaxmat_game::demo`.
#[cfg(not(target_arch = "wasm32"))]
fn demo_from_args() -> Option<shaxmat_game::demo::Demo> {
    let args: Vec<String> = std::env::args().collect();
    let [_, flag, path] = args.as_slice() else {
        return None;
    };
    (flag == "--demo")
        .then(|| shaxmat_game::demo::Demo::load(path).expect("the demo script is readable"))
}
