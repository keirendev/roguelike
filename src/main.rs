use tcod::colors::*;
use tcod::console::*;

// actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

const LIMIT_FPS: i32 = 20; // 20 frames-per-second maximum

struct Tcod {
    root: Root,
}

fn handle_key_input(
    tcod: &mut Tcod,
    player_location_x: &mut i32,
    player_location_y: &mut i32,
) -> bool {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = tcod.root.wait_for_keypress(true);
    match key {
        Key {
            code: Enter,
            alt: true,
            ..
        } => {
            let fullscreen_state = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen_state);
        }
        Key { code: Escape, .. } => {
            let exit_game_status = true;
            return exit_game_status;
        },
        Key { code: Up, .. } => *player_location_y -= 1,
        Key { code: Down, .. } => *player_location_y += 1,
        Key { code: Left, .. } => *player_location_x -= 1,
        Key { code: Right, .. } => *player_location_x += 1,

        _ => {}
    }

    let exit_game_status = false;
    return exit_game_status;
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust roguelike demo")
        .init();

    let mut tcod = Tcod { root };

    tcod::system::set_fps(LIMIT_FPS);

    let default_player_location_width = SCREEN_WIDTH / 2;
    let default_player_location_height = SCREEN_HEIGHT / 2;

    let mut player_location_x = default_player_location_width;
    let mut player_location_y = default_player_location_height;

    while !tcod.root.window_closed() {
        tcod.root.set_default_foreground(WHITE);
        tcod.root.clear();
        tcod.root.put_char(player_location_x, player_location_y, '@', BackgroundFlag::None);
        tcod.root.flush();
        
        let exit_game_status = handle_key_input(&mut tcod, &mut player_location_x, &mut player_location_y);
        if exit_game_status {
            break;
        }
    }
}
