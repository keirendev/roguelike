use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;

mod object;
use object::Object;

pub mod tile;
use tile::Tile;

pub mod game;
use game::Game;
use game::Map;

mod rect;
use rect::Rect;

const WINDOW_WIDTH: i32 = 80;
const WINDOW_HEIGHT: i32 = 50;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

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

fn handle_key_input(tcod: &mut Tcod, player: &mut Object, game: &Game) -> bool {
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

fn make_map(player: &mut Object) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        let rooms_intersect = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));

        if !rooms_intersect {
            create_room(new_room, &mut map);

            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                player.set_location(new_x, new_y);
            }

            else {
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();
            
                if rand::random() {
                    create_horizontal_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_vertical_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_vertical_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_horizontal_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
        }

        rooms.push(new_room);
    }

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

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_horizontal_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_vertical_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn main() {
    let root = Root::initializer()
        .font("res/arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Rust roguelike demo")
        .init();

    let console = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);

    let mut tcod = Tcod { root, console };

    tcod::system::set_fps(LIMIT_FPS);

    let player_char = '@';
    let player_color = WHITE;

    let player = Object::new(
        0,
        0,
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

    let game = Game { map: make_map(&mut objects[0]) };

    while !tcod.root.window_closed() {
        tcod.console.clear();
        render_all(&mut tcod, &game, &objects);
        tcod.root.flush();

        let player = &mut objects[0];
        let exit_game_state = handle_key_input(&mut tcod, player, &game);
        if exit_game_state {
            break;
        }
    }
}
