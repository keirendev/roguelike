use tcod::colors::*;
use tcod::console::*;

mod object;
use object::Object;

pub mod tile;
use tile::Tile;

pub mod game;
use game::Game;
use game::Map;

const WINDOW_WIDTH: i32 = 80;
const WINDOW_HEIGHT: i32 = 50;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};

const LIMIT_FPS: i32 = 20;

struct Tcod {
    root: Root,
    console: Offscreen,
}

fn handle_key_input(
    tcod: &mut Tcod,
    player: &mut Object,
    game: &Game
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
            let exit_game_state = true;
            return exit_game_state;
        }
        Key { code: Up, .. } => player.move_by(0, -1, game),
        Key { code: Down, .. } => player.move_by(0, 1, game),
        Key { code: Left, .. } => player.move_by(-1, 0, game),
        Key { code: Right, .. } => player.move_by(1, 0, game),

        _ => {}
    }

    let exit_game_state = false;
    exit_game_state
}

fn make_map() -> Map {
    let mut map = vec![vec![Tile::empty(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    map[30][22] = Tile::wall();
    map[50][22] = Tile::wall();

    map
}

fn render_all(tcod: &mut Tcod, game: &Game, objects: &[Object]) {
    for object in objects {
        object.draw(&mut tcod.console);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let wall = game.map[x as usize][y as usize].block_sight;
            if wall {
                tcod.console
                    .set_char_background(x, y, COLOR_DARK_WALL, BackgroundFlag::Set);
            } else {
                tcod.console
                    .set_char_background(x, y, COLOR_DARK_GROUND, BackgroundFlag::Set);
            }
        }
    }

    blit(
        &tcod.console,
        (0, 0),
        (WINDOW_WIDTH, WINDOW_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Rust roguelike demo")
        .init();

    let console = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);
    let game = Game {
        map: make_map(),
    };

    let mut tcod = Tcod { root, console };

    tcod::system::set_fps(LIMIT_FPS);

    let default_player_location_width = WINDOW_WIDTH / 2;
    let default_player_location_height = WINDOW_HEIGHT / 2;
    let player_char = '@';
    let player_color = WHITE;

    let player = Object::new(
        default_player_location_width,
        default_player_location_height,
        player_char,
        player_color,
    );

    let default_npc_location_width = (WINDOW_WIDTH / 2) - 5;
    let default_npc_location_height = WINDOW_HEIGHT / 2;
    let npc_char = '@';
    let npc_color = YELLOW;

    let npc = Object::new(
        default_npc_location_width,
        default_npc_location_height,
        npc_char,
        npc_color,
    );

    let mut objects = [player, npc];

    while !tcod.root.window_closed() {
        tcod.console.clear();
        render_all(&mut tcod, &game, &objects);
        tcod.root.flush();

        let player = &mut objects[0];
        let exit_game_state =
            handle_key_input(&mut tcod, player, &game);
        if exit_game_state {
            break;
        }
    }
}
