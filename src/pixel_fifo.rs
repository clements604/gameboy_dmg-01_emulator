
struct PixelFifo {
    pixel_fifo: Vec<Pixel>,
    fetch_state: FetchState,
    line_x: u8,
    pushed_x: u8,
    bgw_fetch_data: [u8; 3],
    fetch_entry_data: [u8; 6], // oam data (temporarily stored)
    map_x: u8,
    map_y: u8,
    tile_y: u8,
    fifo_x: u8,
}

/**
The FIFO and Pixel Fetcher work together to ensure that the FIFO always contains at least 8 pixels at any given time,
as 8 pixels are required for the Pixel Rendering operation to take place.

Each FIFO is manipulated only during mode 3 (Pixel transfer).
**/

enum FetchState {
    Tile,
    TileData0,
    TileData1,
    Idle,
    Push
}

struct Pixel {
    colour: u8,
    palette: u8,
    priority: u8,
    background_priority: u8,
}

impl PixelFifo {
    pub fn new() -> PixelFifo {
        PixelFifo {
            pixel_fifo: Vec::new(),
            fetch_state: FetchState::Idle,
            line_x: 0,
            pushed_x: 0,
            bgw_fetch_data: [0; 3],
            fetch_entry_data: [0; 6], // oam data (temporarily stored)
            map_x: 0,
            map_y: 0,
            tile_y: 0,
            fifo_x: 0,
        }
    }

}

impl Pixel {
    pub fn new() -> Pixel {
        Pixel {
            colour: 0,
            palette: 0,
            priority: 0,
            background_priority: 0,
        }
    }
}
