use crate::tile::Tile;
use crate::messages::Messages;
use crate::entity::Entity;

pub type Map = Vec<Vec<Tile>>;

pub struct Game {
    pub map: Map,
    pub messages: Messages,
    pub inventory: Vec<Entity>,
}
