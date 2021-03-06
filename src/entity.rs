use tcod::colors::*;
use tcod::console::*;

use crate::game::{Game, Map};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, entity: &mut Entity, game: &mut Game) {
        use DeathCallback::*;
        let callback = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(entity, game);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AI {
    Basic,
    Confused {
        previous_ai: Box<AI>,
        num_turns: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
}

pub enum UseResult {
    UsedUp,
    Cancelled,
}

#[derive(Debug)]
pub struct Entity {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<AI>,
    pub item: Option<Item>,
}

impl Entity {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Entity {
            x: x,
            y: y,
            char: char,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
        }
    }

    pub fn set_location(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn get_location(&self) -> (i32, i32) {
        return (self.x, self.y);
    }

    pub fn draw(&self, console: &mut dyn Console) {
        console.set_default_foreground(self.color);
        console.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn distance_to(&self, other: &Entity) -> f32 {
        let distance_x = other.x - self.x;
        let distance_y = other.y - self.y;
        ((distance_x.pow(2) + distance_y.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
            }
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn attack(&mut self, target: &mut Entity, game: &mut Game) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            game.messages.add(
                format!(
                    "{} attacks {} for {} hit points.",
                    self.name, target.name, damage
                ),
                WHITE,
            );
            target.take_damage(damage, game);
        } else {
            game.messages.add(
                format!(
                    "{} attacks {} but it has no effect!",
                    self.name, target.name
                ),
                WHITE,
            );
        }
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Map, entities: &[Entity]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }

    entities
        .iter()
        .any(|entity| entity.blocks && entity.get_location() == (x, y))
}

pub fn move_by(id: usize, x_amount: i32, y_amount: i32, map: &Map, entities: &mut [Entity]) {
    let move_x = entities[id].x + x_amount;
    let move_y = entities[id].y + y_amount;

    if !is_blocked(move_x, move_y, map, entities) {
        entities[id].set_location(move_x, move_y);
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, entities: &mut [Entity]) {
    let distance_x = target_x - entities[id].x;
    let distance_y = target_y - entities[id].y;
    let distance = ((distance_x.pow(2) + distance_y.pow(2)) as f32).sqrt();

    let distance_x = (distance_x as f32 / distance).round() as i32;
    let distance_y = (distance_y as f32 / distance).round() as i32;
    move_by(id, distance_x, distance_y, map, entities);
}

fn player_death(player: &mut Entity, game: &mut Game) {
    game.messages.add("You died!", RED);

    player.char = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut Entity, game: &mut Game) {
    game.messages
        .add(format!("{} is dead!", monster.name), ORANGE);
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}
