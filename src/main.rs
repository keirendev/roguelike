use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};

mod entity;
use entity::Entity;

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

const MAX_ROOM_MONSTERS: i32 = 3;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};
const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

const LIMIT_FPS: i32 = 20;

const PLAYER_ID: usize = 0;

struct Tcod {
    root: Root,
    console: Offscreen,
    fov: FovMap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn handle_key_input(tcod: &mut Tcod, entities: &mut Vec<Entity>, map: &Map) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    use PlayerAction::*;

    let key = tcod.root.wait_for_keypress(true);
    let player_alive = entities[PLAYER_ID].alive;
    match (key, key.text(), player_alive) {
        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
            _,
        ) => {
            let fullscreen_state = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen_state);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _, _) => Exit,
        (Key { code: Up, .. }, _, true) => {
            player_move_or_attack(0, -1, map, entities);
            TookTurn
        }
        (Key { code: Down, .. }, _, true) => {
            player_move_or_attack(0, 1, map, entities);
            TookTurn
        }
        (Key { code: Left, .. }, _, true) => {
            player_move_or_attack(-1, 0, map, entities);
            TookTurn
        }
        (Key { code: Right, .. }, _, true) => {
            player_move_or_attack(1, 0, map, entities);
            TookTurn
        }

        _ => DidntTakeTurn,
    }
}

fn make_map(entities: &mut Vec<Entity>) -> Map {
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
            place_entities(new_room, &map, entities);

            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                entities[PLAYER_ID].set_location(new_x, new_y);
            } else {
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

fn render_all(tcod: &mut Tcod, game: &mut Game, entities: &[Entity], fov_recompute: bool) {
    if fov_recompute {
        let player_location = &entities[PLAYER_ID].get_location();

        tcod.fov.compute_fov(
            player_location.0,
            player_location.1,
            TORCH_RADIUS,
            FOV_LIGHT_WALLS,
            FOV_ALGO,
        );
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let color = match (visible, wall) {
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };

            let explored = &mut game.map[x as usize][y as usize].explored;

            if visible {
                *explored = true;
            }
            if *explored {
                tcod.console
                    .set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }

    let mut to_draw: Vec<_> = entities
        .iter()
        .filter(|o| tcod.fov.is_in_fov(o.x, o.y))
        .collect();

    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));

    for entity in &to_draw {
        entity.draw(&mut tcod.console);
    }

    tcod.root.set_default_foreground(WHITE);
    if let Some(fighter) = entities[PLAYER_ID].fighter {
        tcod.root.print_ex(
            1,
            WINDOW_HEIGHT - 2,
            BackgroundFlag::None,
            TextAlignment::Left,
            format!("HP: {}/{} ", fighter.hp, fighter.max_hp),
        );
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

fn place_entities(room: Rect, map: &Map, entities: &mut Vec<Entity>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !entity::is_blocked(x, y, map, entities) {
            let mut monster = if rand::random::<f32>() < 0.8 {
                let mut orc = Entity::new(x, y, 'o', "orc", DESATURATED_GREEN, true);
                orc.fighter = Some(entity::Fighter {
                    max_hp: 10,
                    hp: 10,
                    defense: 0,
                    power: 3,
                    on_death: entity::DeathCallback::Monster,
                });
                orc.ai = Some(entity::AI::Basic);
                orc
            } else {
                let mut troll = Entity::new(x, y, 'T', "troll", DARKER_GREEN, true);
                troll.fighter = Some(entity::Fighter {
                    max_hp: 16,
                    hp: 16,
                    defense: 1,
                    power: 4,
                    on_death: entity::DeathCallback::Monster,
                });
                troll.ai = Some(entity::AI::Basic);
                troll
            };
            monster.alive = true;
            entities.push(monster);
        }
    }
}

pub fn player_move_or_attack(x_amount: i32, y_amount: i32, map: &Map, entities: &mut [Entity]) {
    let x = entities[PLAYER_ID].x + x_amount;
    let y = entities[PLAYER_ID].y + y_amount;

    let target_id = entities
        .iter()
        .position(|entity| entity.fighter.is_some() && entity.get_location() == (x, y));

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER_ID, target_id, entities);
            player.attack(target);
        }
        None => {
            entity::move_by(PLAYER_ID, x_amount, y_amount, map, entities);
        }
    }
}

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_at_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_at_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &Game, entities: &mut [Entity]) {
    let (monster_x, monster_y) = entities[monster_id].get_location();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if entities[monster_id].distance_to(&entities[PLAYER_ID]) >= 2.0 {
            let (player_x, player_y) = entities[PLAYER_ID].get_location();
            entity::move_towards(monster_id, player_x, player_y, &game.map, entities);
        } else if entities[PLAYER_ID].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, PLAYER_ID, entities);
            monster.attack(player);
        }
    }
}

fn main() {
    let root = Root::initializer()
        .font("res/arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Rust roguelike demo")
        .init();

    let mut tcod = Tcod {
        root,
        console: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    tcod::system::set_fps(LIMIT_FPS);

    let default_x = 0;
    let default_y = 0;
    let mut player = Entity::new(default_x, default_y, '@', "player", WHITE, true);

    player.fighter = Some(entity::Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
        on_death: entity::DeathCallback::Player,
    });

    player.alive = true;

    let mut previous_player_location = player.get_location();

    let mut entities = vec![player];

    let mut game = Game {
        map: make_map(&mut entities),
    };

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            );
        }
    }

    while !tcod.root.window_closed() {
        let player_location = entities[PLAYER_ID].get_location();
        let fov_recompute = previous_player_location != player_location;

        tcod.console.clear();
        render_all(&mut tcod, &mut game, &entities, fov_recompute);
        tcod.root.flush();

        previous_player_location = player_location;
        let player_action = handle_key_input(&mut tcod, &mut entities, &game.map);
        if player_action == PlayerAction::Exit {
            break;
        }

        if entities[PLAYER_ID].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..entities.len() {
                if entities[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &game, &mut entities);
                }
            }
        }
    }
}
