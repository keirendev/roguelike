use crate::tile::Tile;
use crate::messages::Messages;

pub type Map = Vec<Vec<Tile>>;

pub struct Game {
    pub map: Map,
    pub messages: Messages,

}
