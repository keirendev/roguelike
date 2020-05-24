use tcod::colors::*;
use tcod::console::*;

use crate::game::Game;

#[derive(Debug)]
pub struct Object {
    location_x: i32,
    location_y: i32,
    char: char,
    color: Color,
}

impl Object {
    pub fn new(location_x: i32, location_y: i32, char: char, color: Color) -> Self {
        Object {
            location_x,
            location_y,
            char,
            color,
        }
    }

    pub fn move_by(&mut self, x_amount: i32, y_amount: i32, game: &Game) {
        let move_location_x = self.location_x + x_amount;
        let move_location_y = self.location_y + y_amount;

        if !game.map[move_location_x as usize][move_location_y as usize].blocked {
            self.location_x += x_amount;
            self.location_y += y_amount;
        } 
    }

    pub fn draw(&self, console: &mut dyn Console) {
        console.set_default_foreground(self.color);
        console.put_char(
            self.location_x,
            self.location_y,
            self.char,
            BackgroundFlag::None,
        );
    }
}