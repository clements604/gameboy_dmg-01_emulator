
const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

/*
*   Get the tile id from the address of the tile in video ram
*/
fn get_tile_id(address: u16) -> u16 {
    (address / 16) % 256
}

/*
*   Get the x position of the tile in the tile map
*/
fn get_tile_x(address: u16) -> u16 {
    address % 32
}

/*
*   Get the y position of the tile in the tile map
*/
fn get_tile_y(address: u16) -> u16 {
    (address / 32) % 32
}
