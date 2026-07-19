use crate::player::Player;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead, Read, Write};
mod db;
use db::{fetch_stats_by_user, init_db, insert_game_stats, login_user, register_user};

use serde_json::json;
use tokio_tungstenite::accept_async;

use std::net::{TcpListener, TcpStream};
use std::thread;

use futures_util::{SinkExt, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc;
use warp::ws::{Message, WebSocket};
use warp::Filter;

mod card;
mod dealer;
mod hand_evaluator;
mod player;

mod game_variants {
    pub mod razz;
    pub mod seven_card_stud;
    pub mod standard_five;
    pub mod texas_hold_em;
}

use std::{
    io::ErrorKind,
    //net::TcpListener,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use dealer::Dealer;
use game_variants::razz::Razz;
use game_variants::seven_card_stud::SevenCardStud;
use game_variants::standard_five::FiveCardDraw;
use game_variants::texas_hold_em::TexasHoldEm;
use serde::{Deserialize, Serialize};

const LOCAL: &str = "127.0.0.1:6000";
const MSG_SIZE: usize = 32;

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}
const PLAYER_STATS_FILE: &str = "player_stats.json";

struct GameRooms {
    five_card: Arc<Mutex<GameSession>>,
    seven_card: Arc<Mutex<GameSession>>,
    texas_holdem: Arc<Mutex<GameSession>>,
}

impl GameRooms {
    fn new() -> Self {
        GameRooms {
            five_card: Arc::new(Mutex::new(GameSession::new())),
            seven_card: Arc::new(Mutex::new(GameSession::new())),
            texas_holdem: Arc::new(Mutex::new(GameSession::new())),
        }
    }
    fn get_room(&self, variant: &str) -> Arc<Mutex<GameSession>> {
        match variant {
            "five_card_draw" => self.five_card.clone(),
            "seven_card_stud" => self.seven_card.clone(),
            "texas_holdem" => self.texas_holdem.clone(),
            _ => self.texas_holdem.clone(), // default to texas holdem
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub suit: String,  // hearts, diamonds, clubs, spades
    pub value: String, // 2, 3, 4, 5, 6, 7, 8, 9, 10, jack, queen, king, ace
}

impl Card {
    pub fn to_string(&self) -> String {
        format!("{}_{}", self.value, self.suit)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    game_started: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct PlayerState {
    name: String,
    cards: Vec<String>,
    chips: i32,
    current_bet: i32,
    folded: bool,
    is_active: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GameStateUpdate {
    players: Vec<PlayerState>,
    community_cards: Vec<String>,
    current_player: String,
    pot: i32,
    current_bet: i32,
    phase: String,
    winner: Option<String>,
}

/// Struct for managing the game session (FR 8,9,10)
#[derive(Serialize, Deserialize)]
pub struct GameSession {
    pub game_number: u32,
    pub active_players: HashSet<String>,
    pub all_players: HashMap<String, Player>,
    pub game_style: Option<String>,
    pub game_state: GameState,

    #[serde(skip)]
    pub connections: Vec<Arc<Mutex<TcpStream>>>,
}

impl GameSession {
    pub fn new() -> Self {
        let session = Self::load_from_file().unwrap_or_else(|_| {
            println!("No previous data found. Creating a new game session.");
            Self {
                game_number: 0,
                active_players: HashSet::new(),
                all_players: HashMap::new(),
                game_style: None,
                game_state: GameState {
                    game_started: false,
                },
                connections: Vec::new(),
            }
        });
        // println!(
        //     "DEBUG: Loaded players at startup: {:?}",
        //     session.all_players.keys()
        // );

        session
    }
    pub fn save_to_file(&self) {
        match serde_json::to_string_pretty(&self.all_players) {
            Ok(json) => {
                if let Err(e) = std::fs::write(PLAYER_STATS_FILE, json) {
                    println!("Failed to save player data: {}", e);
                }
            }
            Err(e) => {
                println!("Failed to serialize player data: {}", e);
            }
        }
    }

    pub fn load_from_file() -> Result<Self, Box<dyn std::error::Error>> {
        // println!("DEBUG: load_from_file called for {}", PLAYER_STATS_FILE);

        // Log if we fail to read the file at all
        let data = std::fs::read_to_string(PLAYER_STATS_FILE).map_err(|err| {
            // eprintln!("DEBUG: Failed to read '{}': {}", PLAYER_STATS_FILE, err);
            err
        })?;

        // println!("DEBUG: Raw JSON Data -> {}", data);

        // Log if JSON is invalid
        let all_players: HashMap<String, Player> = serde_json::from_str(&data).map_err(|err| {
            eprintln!("DEBUG: JSON parse error: {}", err);
            err
        })?;

        // println!(
        //     "DEBUG: Successfully parsed player data. Loaded keys: {:?}",
        //     all_players.keys()
        // );

        // Build your session
        Ok(Self {
            game_number: 0,
            active_players: HashSet::new(),
            all_players,
            game_style: None,
            game_state: GameState {
                game_started: false,
            },
            connections: Vec::new(),
            
        })
    }

    pub fn register_or_login(&mut self) -> String {
        println!("Enter your unique ID or type 'new' to create an account:");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        let input = input.trim().to_string();

        if input == "new" {
            println!("Enter a username:");
            let mut name = String::new();
            io::stdin()
                .read_line(&mut name)
                .expect("Failed to read input");
            let name = name.trim().to_string();

            if self.all_players.contains_key(&name) {
                println!("This name is already taken. Try logging in.");
                return self.register_or_login();
            }

            let player = Player::new(&name);
            self.all_players.insert(name.clone(), player);
            self.active_players.insert(name.clone());

            println!("Registered successfully! Welcome, {}", name);
            return name;
        } else if self.all_players.contains_key(&input) {
            println!("Welcome back, {}!", input);
            self.active_players.insert(input.clone());
            return input;
        } else {
            println!("Invalid ID. Please try again.");
            return self.register_or_login();
        }
    }

    pub fn set_game_style(&mut self, style: String) {
        self.game_style = Some(style);
    }

    pub fn next_game(&mut self) {
        self.game_number += 1;
    }

    pub fn reset(&mut self) {
        self.game_number = 0;
        self.active_players.clear();
        self.save_to_file();
        println!("Game has been reset. Player data retained.");
    }
}

// Update the handle_client function
fn handle_client(mut stream: TcpStream, game_rooms: Arc<GameRooms>) {
    let mut buffer = [0; 1024];
    let conn = init_db().expect("Failed to initialize DB");

    // Set non-blocking or add timeout if desired
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed
                println!("Client disconnected");
                break;
            }
            Ok(bytes_read) => {
                let msg = String::from_utf8_lossy(&buffer[..bytes_read]);
                if let Ok(json_msg) = serde_json::from_str::<serde_json::Value>(&msg) {
                    let action = json_msg.get("action").and_then(|a| a.as_str());
                    println!("Received action: {:?}", action);
                    let username = json_msg
                        .get("username")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    let password = json_msg
                        .get("password")
                        .and_then(|p| p.as_str())
                        .unwrap_or("");
                    let variant = json_msg
                        .get("variant")
                        .and_then(|v| v.as_str())
                        .unwrap_or("texas_holdem");

                    let response = match action {
                        Some("register") => {
                            if register_user(&conn, username, password).unwrap_or(false) {
                                if let Err(e) = insert_game_stats(&conn, username, 0, 0, 0) {
                                    eprintln!("Failed to insert initial stats: {}", e);
                                }
                                json!({ "status": "success", "message": "User registered" })
                            } else {
                                json!({ "status": "error", "message": "Username already taken" })
                            }
                        }
                        Some("login") => {
                            println!("lets login");
                            if login_user(&conn, username, password).unwrap_or(false) {
                                println!("logged in");
                                json!({ "status": "success", "message": "Login successful" })
                            } else {
                                println!("not logged in :(");
                                json!({ "status": "error", "message": "Invalid username or password" })
                            }
                        }
                        Some("stats") => {
                            match fetch_stats_by_user(&conn, username) {
                                Ok(rows) => {
                                    //debug 
                                    println!("Fetched {} stat rows for user {}", rows.len(), username);
                                    for (game, bets, wins) in &rows {
                                        println!(" - Game: {}, Bets: {}, Winnings: {}", game, bets, wins);
                                    }
                                    let data: Vec<_> = rows
                                        .into_iter()
                                        .map(|(game, bets, wins)| {
                                            json!({
                                                "game_number": game,
                                                "total_bets": bets,
                                                "total_winnings": wins
                                            })
                                        })
                                        .collect();
                                    json!({ "status": "success", "stats": data })
                                }
                                Err(e) => {
                                    //debug 
                                    eprintln!(" fetch_stats_by_user failed for {}: {}", username, e);
                                    json!({ "status": "error", "message": "Failed to fetch stats" })
                                }
                            }
                        }
                        
                        
                        Some("join") => {
                            println!("{} joined the {} room", username, variant);
                            let game_session = game_rooms.get_room(variant);
                            let mut session = game_session.lock().unwrap();

                            let shared_stream = Arc::new(Mutex::new(stream.try_clone().unwrap()));
                            session.connections.push(shared_stream.clone());

                            if !session.all_players.contains_key(username) {
                                let player = Player::new(username);
                                session.all_players.insert(username.to_string(), player);
                                session.active_players.insert(username.to_string());
                            }

                            let player_list: Vec<String> =
                                session.active_players.iter().cloned().collect();

                            println!("Current players in {} room: {:?}", variant, player_list);

                            json!({
                                "status": "success",
                                "message": format!("Joined {} room.", variant),
                                "players": player_list,
                                "variant": variant,
                                "game_started": session.game_state.game_started
                            })
                        }
                        Some("players") => {
                            // println!("{} requested player list for {}", username, variant);
                            let game_session = game_rooms.get_room(variant);
                            let session = game_session.lock().unwrap();
                            let player_list: Vec<String> =
                                session.active_players.iter().cloned().collect();

                            // println!("Current players in {} room: {:?}", variant, player_list);

                            json!({
                                "status": "success",
                                "action": "players",
                                "players": player_list,
                                "game_started": session.game_state.game_started
                            })
                        }
                        Some("start") => {
                            println!("{} requested to start the game in {}", username, variant);
                            // start the game for the given variant
                            // set the game style
                            let game_session = game_rooms.get_room(variant);
                            let mut session = game_session.lock().unwrap();
                        
                            // Set the game style and start the game
                            if session.game_style.is_none() {
                                session.set_game_style(variant.to_string());
                            }
                        
                            session.game_state.game_started = true;
                        
                            let start_str = json!({
                                "status": "success",
                                "message": format!("Game started in {} room.", variant),
                                "game_started": session.game_state.game_started
                            }).to_string();
                        
                            let start_time = Instant::now();
                            let duration = Duration::from_secs(3);
                            let interval = Duration::from_millis(300);

                            // Repeatedly send message for 3 seconds
                            while start_time.elapsed() < duration {
                                session.connections.retain(|conn| {
                                    if let Ok(mut stream) = conn.lock() {
                                        stream.write_all(start_str.as_bytes()).is_ok()
                                    } else {
                                        false
                                    }
                                });

                                thread::sleep(interval);
                            }

                            // Start game logic
                            start_game(&mut session);

                            continue;
                        }
                        Some("get_game_state") => {
                            // println!("{} requested game state for {}", username, variant);
                            let game_session = game_rooms.get_room(variant);
                            let session = game_session.lock().unwrap();
                            if session.game_state.game_started {
                                // send the game state for the player on this client
                                // simple return for now
                                json!({
                                    "status": "success",
                                    "game_started": session.game_state.game_started,
                                    "players": session.active_players.iter().cloned().collect::<Vec<String>>(),
                                    "game_state": session.game_state
                                })
                            } else {
                                json!({
                                    "status": "success",
                                    "game_started": false,
                                    "players": session.active_players.iter().cloned().collect::<Vec<String>>()
                                })
                            }
                        }
                        _ => json!({ "status": "error", "message": "Invalid action" }),
                    };

                    if action != Some("start") { 
                        let response_str = response.to_string();
                        stream.write_all(response_str.as_bytes()).unwrap();
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read from client: {}", e);
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Could not bind server to port");
    let game_rooms = Arc::new(GameRooms::new());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let rooms = game_rooms.clone();
                thread::spawn(move || {
                    handle_client(stream, rooms);
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}

/// Handle player commands
fn parse_cmd(cmd: String, game_session: &Arc<Mutex<GameSession>>) -> String {
    let mut session = game_session.lock().unwrap();

    match cmd.as_str() {
        "1" => session.register_or_login(),
        "2" => {
            let player_list: Vec<_> = session.active_players.iter().cloned().collect();
            return format!("Current players: {}\n", player_list.join(", "));
        }
        "3" => {
            session.set_game_style("Five-Card Draw".to_string());
            return "Game style set to Five-Card Draw.\n".to_string();
        }
        // "4" => {
        //     session.set_game_style("Razz".to_string());
        //     return "Game style set to Razz.\n".to_string();
        // }
        "4" => {
            session.set_game_style("Seven-Card Stud".to_string());
            return "Game style set to Seven-Card Stud.\n".to_string();
        }
        "5" => {
            session.set_game_style("Texas Hold 'Em".to_string());
            return "Game style set to Texas Hold 'Em.\n".to_string();
        }

        "6" => start_game(&mut session),
        "7" => {
            session.reset();
            return "Game reset successful.\n".to_string();
        }
        "8" => {
            println!("Enter your username to view past results:");
            let mut username = String::new();
            io::stdin()
                .read_line(&mut username)
                .expect("Failed to read input");
            let username = username.trim().to_string();

            if let Some(player) = session.all_players.get(&username) {
                return format!(
                    "\nStatistics for {}:\nTotal Winnings: {}\nTotal Bets: {}\n",
                    player.name,
                    player
                        .game_statistics
                        .iter()
                        .map(|s| s.total_winnings)
                        .sum::<u32>(),
                    player
                        .game_statistics
                        .iter()
                        .map(|s| s.total_bets)
                        .sum::<u32>()
                );
            } else {
                return format!("No statistics found for username: '{}'.\n", username);
            }
        }

        _ => return "Unknown command. Type HELP to see available commands.\n".to_string(),
    }
}

fn start_game(session: &mut GameSession) -> String {
    println!("Starting game...");

    if session.active_players.len() < 2 {
        println!("Not enough players to start the game.");
        return "Not enough players to start the game.\n".to_string();
    }

    if session.game_style.is_none() {
        println!("Please choose a game style first (3, 4, or 5).");
        return "Please choose a game style first (3, 4, or 5).\n".to_string();
    }

    session.next_game();

    println!("Game number: {}", session.game_number);
    let game_mode = session.game_style.clone().unwrap();

    println!("Game mode: {}", game_mode);

    let mut players: Vec<Player> = session
        .active_players
        .iter()
        .filter_map(|name| session.all_players.get(name)) // get existing Player
        .cloned() // clone them
        .collect();

    match game_mode.as_str() {
        "texas_holdem" => {
            println!("Starting Texas Hold 'Em game...");
            let mut game = TexasHoldEm::new(players, session.connections.clone());
            game.play();
        }
        "seven_card_stud" => {
            println!("Starting Seven-Card Stud game...");
            let mut game = SevenCardStud::new(players, session.connections.clone());
            game.play();
            for p in game.players {
                if let Some(orig_player) = session.all_players.get_mut(&p.name) {
                    *orig_player = p;
                }
            }
        }
        "five_card_draw" => {
            println!("Starting Five-Card Draw game...");
            let player_names: Vec<&str> = session
                .active_players
                .iter()
                .map(|name| name.as_str())
                .collect();
            let mut game = FiveCardDraw::new(player_names, session.connections.clone());
            game.play();
        }
        _ => return "Invalid game mode.\n".to_string(),
    }
    session.save_to_file();
    return "Returning to main menu.\n".to_string();
}

fn post_game_menu(session: &mut GameSession) {
    let mut remaining_players = Vec::new();
    println!("\n--- Post-Game Menu ---");

    for player_id in session.active_players.clone() {
        println!("\n{}, choose an action:", player_id);
        println!("1. Play the next game");
        println!("2. Leave the game");
        println!("3. See statistics");
        println!("4. Reset game session");

        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .expect("Failed to read input");

        match choice.trim() {
            "1" => remaining_players.push(player_id.clone()),
            "2" => {
                println!("{} has left the game.", player_id);
                session.active_players.remove(&player_id);
            }
            "3" => {
                if let Some(player) = session.all_players.get(&player_id) {
                    // println!("DEBUG: Fetching stats for {}", player_id);
                    println!("Statistics for {}:", player_id);
                    println!(
                        "Total Bets: {}",
                        player
                            .game_statistics
                            .iter()
                            .map(|s| s.total_bets)
                            .sum::<u32>()
                    );
                    println!(
                        "Total Winnings: {}",
                        player
                            .game_statistics
                            .iter()
                            .map(|s| s.total_winnings)
                            .sum::<u32>()
                    );
                } else {
                    println!("No statistics found for {}.", player_id);
                }
            }
            "4" => {
                println!("Resetting game session...");
                session.reset();
                return;
            }
            _ => println!("Invalid choice. Skipping..."),
        }
    }

    session.active_players = remaining_players.into_iter().collect();

    if session.active_players.is_empty() {
        println!("No active players left. Allowing new users to join.");
        registration_menu(session);
    } else {
        println!("Proceeding to game selection...");
        select_new_game(session);
    }
}

fn registration_menu(session: &mut GameSession) {
    println!("\nAllowing new players to join. Press enter to continue.");
    io::stdin()
        .read_line(&mut String::new())
        .expect("Failed to read input");
    session.register_or_login();
}

fn select_new_game(session: &mut GameSession) {
    println!("Choose a game style (3, 4, 5):");
}
/// Print available commands
fn print_cmds() -> String {
    let commands = "Available Commands:
-------------------------------------
1        - Register as a player
2        - Show all registered players
3        - Select Five-Card Draw
4        - Select Razz
5        - Select Texas Hold 'Em
6        - Start the game (requires 2+ players)
7        - Reset game data and player list
8        - View past results (by username)
-------------------------------------\n";
    commands.to_string()
}

mod tests {
    pub mod card_test;
    pub mod dealer_test;
    #[cfg(test)]
    pub mod evaluate_hand_tests;
    pub mod player_test;
}
