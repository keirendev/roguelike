use crate::tile::Tile;

pub type Map = Vec<Vec<Tile>>;

pub struct Game {
    pub map: Map,
}