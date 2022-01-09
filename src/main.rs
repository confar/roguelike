use tcod::colors::*;
use tcod::console::*;
use std::cmp;
use rand::{Rng, random};
use tcod::map::{FovAlgorithm, Map as FovMap};

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const MAX_ROOM_MONSTERS: i32 = 3;
const PLAYER: usize = 0;

const COLOR_DARK_WALL: Color = Color {r: 0, g: 0, b: 100};
const COLOR_DARK_GROUND: Color = Color {r: 50, g: 50, b: 150};
const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};
const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

const LIMIT_FPS: i32 = 20; // 20 frames-per-second maximum

#[derive(Clone, Debug, PartialEq)]
enum Ai {
    Basic,

}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Fighter {
    hp: i32,
    max_hp: i32,
    defense: i32,
    power: i32,

}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

fn create_room(room: Rect, map: &mut Map) {
    // go through the tiles in the rectangle and make them passable
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false, 
            block_sight: false,
            explored: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            explored: false,
        }
    }
}

struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
}

type Map = Vec<Vec<Tile>>;

struct Game {
    map: Map,
}

fn make_map(objects: &mut Vec<Object>) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);
        let new_room = Rect::new(x, y, w, h);
        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);
            let (new_x, new_y) = new_room.center();
            if rooms.is_empty() {
                objects[PLAYER].set_position(new_x, new_y)
            } else {
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();
                if rand::random() {
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
            rooms.push(new_room);
        }
    }
    map
}

fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    let player = &objects[PLAYER];
    if fov_recompute{
        tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }
    for object in objects {
        if tcod.fov.is_in_fov(object.x, object.y) {
            object.draw(&mut tcod.con);
        }
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
                *explored = true
            }
            if *explored {
                tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }

    blit(&tcod.con,
         (0,0 ),
         (MAP_WIDTH, MAP_HEIGHT),
         &mut tcod.root,
         (0,0),
         1.0,
         1.0
    );
}

#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, color: Color, name: &str, blocks: bool, alive: bool) -> Self {
        Object {x: x, 
            y: y, 
            char: char, 
            color: color, 
            name: name.into(), 
            blocks: blocks, 
            alive: alive,
            fighter: None,
            ai: None,
        }
    }
    
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
    
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, tcod::console::BackgroundFlag::None);
    }
    
    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }
    
    pub fn take_damage(&mut self, damage: i32) {
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
    }
    
    pub fn attack(&mut self, target: &mut Object) {
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            println!("{} attacks {} for {} hit points", self.name, target.name, damage);
            target.take_damage(damage);
        }  else {
            println!("{} attacks {} but it has no effect", self.name, target.name);
        }
    }
}

fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &Game, objects: &mut [Object]) {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, game, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let monster = &mut objects[monster_id];
            monster.attack(&mut objects[PLAYER]);
            
        }
    }
}

fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust libtcod tutorial")
        .init();
    tcod::system::set_fps(LIMIT_FPS);
    let mut tcod = Tcod {
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT), 
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    let mut player = Object::new(0, 0, '@', WHITE,"hero", true, true);
    player.fighter = Some(Fighter {hp: 30, max_hp: 30, defense: 2, power: 5});
    let mut objects = vec![player];


    let mut game = Game{
        map: make_map(&mut objects),
    };
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x, 
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            )
        }
    }
    let mut previous_player_position = (-1, -1);
    while !tcod.root.window_closed() {
        tcod.con.clear();
        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        render_all(&mut tcod, &mut game, &objects, fov_recompute);
        tcod.root.flush();
        
        let player = &mut objects[PLAYER];
        previous_player_position = (player.x, player.y);
        let player_action = handle_keys(&mut tcod, &game, &mut objects);

        if player_action == PlayerAction::Exit {
            break;
        }
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &game, &mut objects);
                }
            }
        }
    }
}

fn handle_keys(tcod: &mut Tcod, game: &Game, objects: &mut [Object]) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;
    let key = tcod.root.wait_for_keypress(true);
    let player_alive = objects[PLAYER].alive;
    match (key, key.text(), player_alive) {
        (Key {
            code: Enter,
            alt: true,
            ..
        }, _, _) => {
            // Alt+Enter: toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _, _) => Exit,
        (Key { code: Up, .. }, _, true) => {
            player_move_or_attack(PLAYER, 0, -1, game, objects);
            TookTurn
        }, 
        (Key { code: Down, .. }, _, true) => {
            player_move_or_attack(PLAYER, 0, 1, game, objects);
            TookTurn
        },
        (Key { code: Left, .. }, _, true) => {
            player_move_or_attack(PLAYER, -1, 0, game, objects);
            TookTurn
        },
        (Key { code: Right, .. }, _, true) => {
            player_move_or_attack(PLAYER, 1, 0, game, objects);
            TookTurn
        },
        _ => DidntTakeTurn,
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }
    objects.iter().any(|object| object.blocks && object.pos() == (x, y))
}

fn player_move_or_attack(id: usize, dx: i32, dy: i32, game: &Game, objects: &mut[Object]) {
    // the coordinates of player moving/attacking
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;
    let target_id = objects.iter().position(|object| object.pos() == (x, y));
    match target_id {
        Some(target_id) => println!("The {} laughs at your clumsy efforts to attack him", 
                                    objects[target_id].name),
        None => {
            move_by(PLAYER, dx, dy, &game, objects)
        }
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, game: &Game, objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, game, objects)
}

fn move_by(id: usize, dx: i32, dy: i32, game: &Game, objects: &mut[Object]) {
    let (x, y)= objects[id].pos();
    if !is_blocked(x + dx, y + dy, &game.map, objects) {
        objects[id].set_position(x+dx, y+dy);
    }
}

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);
    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
        if !is_blocked(x, y, map, objects) {
            let monster = if rand::random::<f32>() < 0.8 {
                let mut orc = Object::new(x, y, 'o', DESATURATED_GREEN, "orc", true, true);
                orc.fighter = Some(Fighter { hp: 10, max_hp: 10, defense: 0, power: 2 });
                orc.ai = Some(Ai::Basic);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', DARK_GREEN, "troll", true, true);
                troll.fighter = Some(Fighter { hp: 16, max_hp: 16, defense: 1, power: 4 });
                troll.ai = Some(Ai::Basic);
                troll
            };
            objects.push(monster);
        }
    }
}
