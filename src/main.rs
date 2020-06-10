use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use tcod::input::{self, Event, Key, Mouse};
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

mod messages;
use messages::Messages;

const WINDOW_WIDTH: i32 = 80;
const WINDOW_HEIGHT: i32 = 50;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;

const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = WINDOW_HEIGHT - PANEL_HEIGHT;

const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = WINDOW_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 2;

const INVENTORY_WIDTH: i32 = 50;

const HEAL_AMOUNT: i32 = 4;

const LIGHTNING_DAMAGE: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;

const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;

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
    panel: Offscreen,
    fov: FovMap,
    key: Key,
    mouse: Mouse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn handle_key_input(tcod: &mut Tcod, entities: &mut Vec<Entity>, game: &mut Game) -> PlayerAction {
    use tcod::input::KeyCode::*;

    use PlayerAction::*;

    let player_alive = entities[PLAYER_ID].alive;
    match (tcod.key, tcod.key.text(), player_alive) {
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
            player_move_or_attack(0, -1, game, entities);
            TookTurn
        }
        (Key { code: Down, .. }, _, true) => {
            player_move_or_attack(0, 1, game, entities);
            TookTurn
        }
        (Key { code: Left, .. }, _, true) => {
            player_move_or_attack(-1, 0, game, entities);
            TookTurn
        }
        (Key { code: Right, .. }, _, true) => {
            player_move_or_attack(1, 0, game, entities);
            TookTurn
        }
        (Key { code: Text, .. }, "g", true) => {
            let item_id = entities.iter().position(|entity| {
                entity.get_location() == entities[PLAYER_ID].get_location() && entity.item.is_some()
            });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, entities);
            }
            DidntTakeTurn
        }
        (Key { code: Text, .. }, "i", true) => {
            let original_inventory_length = &game.inventory.len();
            let inventory_index = inventory_menu(
                &game.inventory,
                "Press the key next to an item to use it, or any other to cancel.\n",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, tcod, game, entities);
            }
            if original_inventory_length > &game.inventory.len() {
                TookTurn
            } else {
                DidntTakeTurn
            }
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

    blit(
        &tcod.console,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );

    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    let hp = entities[PLAYER_ID].fighter.map_or(0, |f| f.hp);
    let max_hp = entities[PLAYER_ID].fighter.map_or(0, |f| f.max_hp);
    render_bar(
        &mut tcod.panel,
        1,
        1,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        LIGHT_RED,
        DARKER_RED,
    );

    tcod.panel.set_default_foreground(LIGHT_GREY);
    tcod.panel.print_ex(
        1,
        0,
        BackgroundFlag::None,
        TextAlignment::Left,
        get_names_under_mouse(tcod.mouse, entities, &tcod.fov),
    );

    blit(
        &tcod.panel,
        (0, 0),
        (WINDOW_WIDTH, PANEL_HEIGHT),
        &mut tcod.root,
        (0, PANEL_Y),
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

    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !entity::is_blocked(x, y, map, entities) {
            let dice = rand::random::<f32>();
            let item = if dice < 0.7 {
                let mut entity = Entity::new(x, y, '!', "healing potion", VIOLET, false);
                entity.item = Some(entity::Item::Heal);
                entity
            } else if dice < 0.7 + 0.1 {
                let mut object =
                    Entity::new(x, y, '#', "scroll of lightning bolt", LIGHT_YELLOW, false);
                object.item = Some(entity::Item::Lightning);
                object
            } else {
                let mut object = Entity::new(x, y, '#', "scroll of confusion", LIGHT_YELLOW, false);
                object.item = Some(entity::Item::Confuse);
                object
            };
            entities.push(item);
        }
    }
}

pub fn player_move_or_attack(
    x_amount: i32,
    y_amount: i32,
    game: &mut Game,
    entities: &mut [Entity],
) {
    let x = entities[PLAYER_ID].x + x_amount;
    let y = entities[PLAYER_ID].y + y_amount;

    let target_id = entities
        .iter()
        .position(|entity| entity.fighter.is_some() && entity.get_location() == (x, y));

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER_ID, target_id, entities);
            player.attack(target, game);
        }
        None => {
            entity::move_by(PLAYER_ID, x_amount, y_amount, &game.map, entities);
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

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut Game, entities: &mut [Entity]) {
    use entity::AI::*;
    if let Some(ai) = entities[monster_id].ai.take() {
        let new_ai = match ai {
            Basic => ai_basic(monster_id, tcod, game, entities),
            Confused {
                previous_ai,
                num_turns,
            } => ai_confused(monster_id, tcod, game, entities, previous_ai, num_turns),
        };
        entities[monster_id].ai = Some(new_ai);
    }
}

fn ai_basic(monster_id: usize, tcod: &Tcod, game: &mut Game, entities: &mut [Entity]) -> entity::AI {
    let (monster_x, monster_y) = entities[monster_id].get_location();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if entities[monster_id].distance_to(&entities[PLAYER_ID]) >= 2.0 {
            let (player_x, player_y) = entities[PLAYER_ID].get_location();
            entity::move_towards(monster_id, player_x, player_y, &game.map, entities);
        } else if entities[PLAYER_ID].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, PLAYER_ID, entities);
            monster.attack(player, game);
        }
    }
    entity::AI::Basic
}

fn ai_confused(
    monster_id: usize,
    _tcod: &Tcod,
    game: &mut Game,
    entities: &mut [Entity],
    previous_ai: Box<entity::AI>,
    num_turns: i32,
) -> entity::AI {
    if num_turns >= 0 {
        entity::move_by(
            monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            &game.map,
            entities,
        );
        entity::AI::Confused {
            previous_ai: previous_ai,
            num_turns: num_turns - 1,
        }
    } else {
        game.messages.add(
            format!("The {} is no longer confused!", entities[monster_id].name),
            RED,
        );
        *previous_ai
    }
}

fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    back_color: Color,
) {
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    panel.set_default_foreground(WHITE);
    panel.print_ex(
        x + total_width / 2,
        y,
        BackgroundFlag::None,
        TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
    );
}

fn get_names_under_mouse(mouse: Mouse, entities: &[Entity], fov_map: &FovMap) -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = entities
        .iter()
        .filter(|obj| obj.get_location() == (x, y) && fov_map.is_in_fov(obj.x, obj.y))
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}

fn pick_item_up(object_id: usize, game: &mut Game, entities: &mut Vec<Entity>) {
    if game.inventory.len() >= 26 {
        game.messages.add(
            format!(
                "Your inventory is full, cannot pick up {}.",
                entities[object_id].name
            ),
            RED,
        );
    } else {
        let item = entities.swap_remove(object_id);
        game.messages
            .add(format!("You picked up a {}!", item.name), GREEN);
        game.inventory.push(item);
    }
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    assert!(
        options.len() <= 26,
        "Cannot have a menu with more than 26 options."
    );

    let header_height = root.get_height_rect(0, 0, width, WINDOW_HEIGHT, header);
    let height = options.len() as i32 + header_height;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(WHITE);
    window.print_rect_ex(
        0,
        0,
        width,
        height,
        BackgroundFlag::None,
        TextAlignment::Left,
        header,
    );

    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(
            0,
            header_height + index as i32,
            BackgroundFlag::None,
            TextAlignment::Left,
            text,
        );
    }

    let x = WINDOW_WIDTH / 2 - width / 2;
    let y = WINDOW_HEIGHT / 2 - height / 2;
    blit(&window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);

    root.flush();
    let key = root.wait_for_keypress(true);

    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu(inventory: &[Entity], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, entities: &mut [Entity]) {
    use entity::Item::*;
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            Lightning => cast_lightning,
            Confuse => cast_confuse,
        };
        match on_use(inventory_id, tcod, game, entities) {
            entity::UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            entity::UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(
            format!("The {} cannot be used.", game.inventory[inventory_id].name),
            WHITE,
        );
    }
}

fn cast_heal(
    _inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut Game,
    entities: &mut [Entity],
) -> entity::UseResult {
    if let Some(fighter) = entities[PLAYER_ID].fighter {
        if fighter.hp == fighter.max_hp {
            game.messages.add("You are already at full health.", RED);
            return entity::UseResult::Cancelled;
        }
        game.messages
            .add("Your wounds start to feel better!", LIGHT_VIOLET);
        entities[PLAYER_ID].heal(HEAL_AMOUNT);
        return entity::UseResult::UsedUp;
    }
    entity::UseResult::Cancelled
}

fn cast_lightning(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    entities: &mut [Entity],
) -> entity::UseResult {
    let monster_id = closest_monster(tcod, entities, LIGHTNING_RANGE);
    if let Some(monster_id) = monster_id {
        game.messages.add(
            format!(
                "A lightning bolt strikes the {} with a loud thunder! \
                 The damage is {} hit points.",
                 entities[monster_id].name, LIGHTNING_DAMAGE
            ),
            LIGHT_BLUE,
        );
        entities[monster_id].take_damage(LIGHTNING_DAMAGE, game);
        entity::UseResult::UsedUp
    } else {
        game.messages
            .add("No enemy is close enough to strike.", RED);
            entity::UseResult::Cancelled
    }
}

fn closest_monster(tcod: &Tcod, entities: &[Entity], max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32;

    for (id, object) in entities.iter().enumerate() {
        if (id != PLAYER_ID)
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y)
        {
            let dist = entities[PLAYER_ID].distance_to(object);
            if dist < closest_dist {
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

fn cast_confuse(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    entities: &mut [Entity],
) -> entity::UseResult {
    let monster_id = closest_monster(tcod, entities, CONFUSE_RANGE);
    if let Some(monster_id) = monster_id {
        let old_ai = entities[monster_id].ai.take().unwrap_or(entity::AI::Basic);
        entities[monster_id].ai = Some(entity::AI::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        game.messages.add(
            format!(
                "The eyes of {} look vacant, as he starts to stumble around!",
                entities[monster_id].name
            ),
            LIGHT_GREEN,
        );
        entity::UseResult::UsedUp
    } else {
        game.messages
            .add("No enemy is close enough to strike.", RED);
            entity::UseResult::Cancelled
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
        panel: Offscreen::new(WINDOW_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
        key: Default::default(),
        mouse: Default::default(),
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
        messages: Messages::new(),
        inventory: vec![],
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

    game.messages.add(
        "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
        RED,
    );

    while !tcod.root.window_closed() {
        let player_location = entities[PLAYER_ID].get_location();
        let fov_recompute = previous_player_location != player_location;

        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => tcod.key = k,
            _ => tcod.key = Default::default(),
        }

        tcod.console.clear();
        render_all(&mut tcod, &mut game, &entities, fov_recompute);
        tcod.root.flush();

        previous_player_location = player_location;
        let player_action = handle_key_input(&mut tcod, &mut entities, &mut game);
        if player_action == PlayerAction::Exit {
            break;
        }

        if entities[PLAYER_ID].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..entities.len() {
                if entities[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &mut game, &mut entities);
                }
            }
        }
    }
}
