use crate::player::Player;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
mod db;
use db::{init_db, login_user, register_user};

use serde_json::{json, Value};

use std::net::{TcpListener, TcpStream};
use std::thread;

mod card;
mod dealer;
mod hand_evaluator;
mod player;

mod game_variants {
    pub mod texas_hold_em;
}

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use game_variants::texas_hold_em::TexasHoldEm;
use serde::{Deserialize, Serialize};

struct GameRooms {
    texas_holdem: Arc<Mutex<GameSession>>,
}

impl GameRooms {
    fn new() -> Self {
        GameRooms {
            texas_holdem: Arc::new(Mutex::new(GameSession::new())),
        }
    }
    fn get_room(&self, variant: &str) -> Arc<Mutex<GameSession>> {
        match variant {
            "texas_holdem" => self.texas_holdem.clone(),
            _ => self.texas_holdem.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub suit: String,
    pub value: String,
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

pub struct GameSession {
    pub game_number: u32,
    pub active_players: HashSet<String>,
    pub all_players: HashMap<String, Player>,
    pub game_style: Option<String>,
    pub game_state: GameState,
    pub connections: HashMap<String, Arc<Mutex<TcpStream>>>,
    pub action_senders: HashMap<String, Sender<Value>>,
    pub action_receivers: HashMap<String, Receiver<Value>>,
    pub last_update: Option<String>,
}

impl GameSession {
    pub fn new() -> Self {
        Self {
            game_number: 0,
            active_players: HashSet::new(),
            all_players: HashMap::new(),
            game_style: None,
            game_state: GameState { game_started: false },
            connections: HashMap::new(),
            action_senders: HashMap::new(),
            action_receivers: HashMap::new(),
            last_update: None,
        }
    }

    pub fn set_game_style(&mut self, style: String) {
        self.game_style = Some(style);
    }

    pub fn next_game(&mut self) {
        self.game_number += 1;
    }
}

fn handle_client(mut stream: TcpStream, game_rooms: Arc<GameRooms>) {
    let conn = init_db().expect("Failed to initialize DB");

    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();


    let write_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to clone stream for writing: {}", e);
            return;
        }
    };
    let writer: Arc<Mutex<TcpStream>> = Arc::new(Mutex::new(write_stream));

    let send = |writer: &Arc<Mutex<TcpStream>>, payload: &str| -> bool {
        match writer.lock() {
            Ok(mut s) => s.write_all(payload.as_bytes()).is_ok(),
            Err(_) => false,
        }
    };

    let mut known_username: Option<String> = None;
    let mut known_variant: String = "texas_holdem".to_string();

    let mut read_buf = String::new();
    let mut buffer = [0; 4096];

    'read_loop: loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                if let Some(username) = &known_username {
                    let game_session = game_rooms.get_room(&known_variant);
                    let session = game_session.lock().unwrap();
                    if let Some(sender) = session.action_senders.get(username) {
                        let _ = sender.send(json!({ "player_action": "__disconnect__" }));
                    }
                }
                break;
            }
            Ok(bytes_read) => {
                read_buf.push_str(&String::from_utf8_lossy(&buffer[..bytes_read]));

                while let Some(newline_pos) = read_buf.find('\n') {
                    let line: String = read_buf.drain(..=newline_pos).collect();
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let json_msg = match serde_json::from_str::<serde_json::Value>(line) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("Failed to parse message ({}): {}", e, line);
                            continue;
                        }
                    };

                    let action = json_msg.get("action").and_then(|a| a.as_str());
                    let username = json_msg.get("username").and_then(|u| u.as_str()).unwrap_or("");
                    let password = json_msg.get("password").and_then(|p| p.as_str()).unwrap_or("");
                    let variant = json_msg.get("variant").and_then(|v| v.as_str()).unwrap_or("texas_holdem");

                    if !username.is_empty() {
                        known_username = Some(username.to_string());
                    }
                    known_variant = variant.to_string();

                    // route player actions to the game engine via a channel
                    if action == Some("player_action") {
                        let game_session = game_rooms.get_room(variant);
                        let session = game_session.lock().unwrap();
                        if let Some(sender) = session.action_senders.get(username) {
                            let _ = sender.send(json_msg.clone());
                        }
                        continue;
                    }

                    let response = match action {
                        Some("register") => {
                            if register_user(&conn, username, password).unwrap_or(false) {
                                let game_session = game_rooms.get_room(variant);
                                let mut session = game_session.lock().unwrap();
                                if !session.all_players.contains_key(username) {
                                    let player = Player::new(username);
                                    session.all_players.insert(username.to_string(), player);
                                }
                                json!({ "status": "success", "message": "User registered", "username": username })
                            } else {
                                json!({ "status": "error", "message": "Username already taken" })
                            }
                        }
                        Some("login") => {
                            if login_user(&conn, username, password).unwrap_or(false) {
                                json!({ "status": "success", "message": "Login successful", "username": username })
                            } else {
                                json!({ "status": "error", "message": "Invalid username or password" })
                            }
                        }
                        Some("join") => {
                            println!("{} joined the {} room", username, variant);
                            let game_session = game_rooms.get_room(variant);
                            let mut session = game_session.lock().unwrap();

                            session.connections.insert(username.to_string(), Arc::clone(&writer));

                            if !session.all_players.contains_key(username) {
                                let player = Player::new(username);
                                session.all_players.insert(username.to_string(), player);
                            }

                            session.active_players.insert(username.to_string());

            
                            let (tx, rx) = mpsc::channel::<Value>();
                            session.action_senders.insert(username.to_string(), tx);
                            session.action_receivers.insert(username.to_string(), rx);

                            let player_list: Vec<String> = session.active_players.iter().cloned().collect();

                            json!({
                                "status": "success",
                                "message": format!("Joined {} room.", variant),
                                "players": player_list,
                                "variant": variant,
                                "game_started": session.game_state.game_started
                            })
                        }
                        Some("players") => {
                            let game_session = game_rooms.get_room(variant);
                            let session = game_session.lock().unwrap();
                            let player_list: Vec<String> = session.active_players.iter().cloned().collect();

                            json!({
                                "status": "success",
                                "action": "players",
                                "players": player_list,
                                "game_started": session.game_state.game_started
                            })
                        }
                        Some("start") => {
                            let game_session = game_rooms.get_room(variant);

                            let (players, connections, receivers) = {
                                let mut session = game_session.lock().unwrap();
                                if session.game_style.is_none() {
                                    session.set_game_style(variant.to_string());
                                }
                                session.game_state.game_started = true;
                                session.last_update = None;

                                let players: Vec<Player> = session
                                    .active_players
                                    .iter()
                                    .filter_map(|name| session.all_players.get(name))
                                    .cloned()
                                    .collect();

                                let active_names: Vec<String> = session.active_players.iter().cloned().collect();

                                let mut receivers = HashMap::new();
                                for name in &active_names {
                                    if let Some(rx) = session.action_receivers.remove(name) {
                                        receivers.insert(name.clone(), rx);
                                    }
                                }

                                (players, session.connections.clone(), receivers)
                            };

                            let start_str = format!(
                                "{}\n",
                                json!({
                                    "status": "success",
                                    "message": format!("Game started in {} room.", variant),
                                    "game_started": true
                                })
                            );

                            for _ in 0..10 {
                                for conn in connections.values() {
                                    if let Ok(mut s) = conn.lock() {
                                        let _ = s.write_all(start_str.as_bytes());
                                    }
                                }
                                thread::sleep(Duration::from_millis(300));
                            }

                            if players.len() < 2 {
                                let mut session = game_session.lock().unwrap();
                                session.game_state.game_started = false;
                                continue;
                            }

                            // Run the game start on its own thread, go back to listening for current player
                            let game_session_for_thread = game_session.clone();
                            thread::spawn(move || {
                                let mut game = TexasHoldEm::new(players, connections, receivers);
                                game.play();

                                let mut session = game_session_for_thread.lock().unwrap();
                                if let Some(final_state) = &game.last_state {
                                    session.last_update = serde_json::to_string(final_state).ok();
                                }
                                session.game_state.game_started = false;
                                session.active_players.clear();
                            });

                            continue;
                        }
                        Some("get_game_state") => {
                            let game_session = game_rooms.get_room(variant);
                            let session = game_session.lock().unwrap();
                            if session.game_state.game_started {
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
                        Some("game_state_update") => {
                            let game_session = game_rooms.get_room(variant);
                            let session = game_session.lock().unwrap();
                            let response_str = match &session.last_update {
                                Some(update_json) => format!("{}\n", update_json),
                                None => format!(
                                    "{}\n",
                                    json!({ "status": "error", "message": "No game state available yet" })
                                ),
                            };
                            drop(session);
                            send(&writer, &response_str);
                            continue;
                        }
                        _ => json!({ "status": "error", "message": "Invalid action" }),
                    };

                    let response_str = format!("{}\n", response);
                    if !send(&writer, &response_str) {
                        eprintln!("Failed to write response to client");
                        break 'read_loop;
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut {
                    continue;
                }
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