use tcod::colors::*;
use tcod::console::*;

use crate::game::Map;

#[derive(Debug)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    char: char,
    color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x: x,
            y: y,
            char: char,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
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
        console.put_char(
            self.x,
            self.y,
            self.char,
            BackgroundFlag::None,
        );
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }

    objects
        .iter()
        .any(|object| object.blocks && object.get_location() == (x, y))
}

pub fn move_by(id: usize, x_amount: i32, y_amount: i32, map: &Map, objects: &mut [Object]) {
    let move_x = objects[id].x + x_amount;
    let move_y = objects[id].y + y_amount;

    if !is_blocked(move_x, move_y, map, objects) {
        objects[id].set_location(move_x, move_y);
    }
}