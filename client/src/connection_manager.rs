use crate::game_screen::GameStateUpdate;
use macroquad::math::bool;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use serde_json::from_str;
use std::time::{Duration, Instant};

pub struct ConnectionManager {
    stream: Option<TcpStream>,
    last_poll_time: Instant,
    poll_interval: Duration,
    cached_players: Vec<String>,
    cached_game_state: bool,
}

// implement default for ConnectionManager
impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            stream: None,
            last_poll_time: Instant::now(),
            poll_interval: Duration::from_secs(5),
            cached_players: Vec::new(),
            cached_game_state: false,
        }
    }

    pub fn connect(&mut self) -> bool {
        let server_address = "127.0.0.1:8080";
        match TcpStream::connect(server_address) {
            Ok(stream) => {
                if let Err(e) = stream.set_nonblocking(true) {
                    eprintln!("Failed to set non-blocking: {}", e);
                    return false;
                }
                self.stream = Some(stream);
                println!("Connected to server at {}", server_address);
                true
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
                false
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn join_room(&mut self, variant: &str, username: &str) -> bool {
        if !self.is_connected() && !self.connect() {
            println!("returning false");
            return false;
        }

        let join_msg = json!({
            "action": "join",
            "variant": variant,
            "username": username
        })
        .to_string();

        println!("join message created");

        if let Some(ref mut stream) = self.stream {
            if stream.write(join_msg.as_bytes()).is_err() {
                self.stream = None;
                return false;
            }

            // Wait briefly for response
            std::thread::sleep(Duration::from_millis(100));

            let mut buffer = [0; 1024];
            match stream.read(&mut buffer) {
                Ok(size) if size > 0 => {
                    let response = String::from_utf8_lossy(&buffer[..size]);
                    println!("response: {}", response);
                    if let Ok(val) = serde_json::from_str::<Value>(&response) {
                        if val["status"] == "success" {
                            // Update cache
                            if let Some(players) = val.get("players").and_then(|v| v.as_array()) {
                                self.cached_players = players
                                    .iter()
                                    .filter_map(|p| p.as_str().map(String::from))
                                    .collect();
                            }
                            if let Some(started) = val.get("game_started").and_then(|v| v.as_bool())
                            {
                                self.cached_game_state = started;
                            }
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn update_players(&mut self, variant: &str, username: &str) -> bool {
        use std::time::Instant;

        if !self.is_connected() && !self.connect() {
            println!("returning false");
            return false;
        }

        let player_request = json!({
            "action": "players",
            "variant": variant,
            "username": username
        })
        .to_string();

        if let Some(ref mut stream) = self.stream {
            // Send the request
            if stream.write_all(player_request.as_bytes()).is_err() {
                println!("failed to write to stream");
                self.stream = None;
                return false;
            }

            // Buffer for reading
            let mut buffer = [0; 2048];
            let mut success = false;
            let start_time = Instant::now();

            // Read multiple messages for up to 500ms
            while start_time.elapsed().as_millis() < 500 {
                match stream.read(&mut buffer) {
                    Ok(size) if size > 0 => {
                        let response = String::from_utf8_lossy(&buffer[..size]);
                        // println!("response: {}", response);

                        // extract the players from the response
                        if let Ok(val) = serde_json::from_str::<Value>(&response) {
                            if val["status"] == "success" {
                                // Update cache
                                if let Some(players) = val.get("players").and_then(|v| v.as_array())
                                {
                                    self.cached_players = players
                                        .iter()
                                        .filter_map(|p| p.as_str().map(String::from))
                                        .collect();
                                }
                                if let Some(started) =
                                    val.get("game_started").and_then(|v| v.as_bool())
                                {

                                    self.cached_game_state = started;
                                    println!("game started: {}", started);
                                }
                                success = true;
                            }
                        }
                    }
                    Ok(_) => {
                        // No data received yet, sleep briefly
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        // println!("read error: {}", e);
                        break;
                    }
                }
            }

            return success;
        }

        false
    }

    pub fn get_game_state(&mut self, variant: &str, username: &str) -> bool {
        // ask the server for the game state
        if !self.is_connected() && !self.connect() {
            println!("returning false");
            return false;
        }
        let game_state_request = json!({
            "action": "game_state",
            "variant": variant,
            "username": username
        })
        .to_string();

        if let Some(ref mut stream) = self.stream {
            // Send the request
            if stream.write_all(game_state_request.as_bytes()).is_err() {
                println!("failed to write to stream");
                self.stream = None;
                return false;
            }

            // Buffer for reading
            let mut buffer = [0; 2048];
            let mut success = false;
            let start_time = Instant::now();

            //println!("waiting for game state response");

            // Read multiple messages for up to 500ms
            while start_time.elapsed().as_millis() < 500 {
                match stream.read(&mut buffer) {
                    Ok(size) if size > 0 => {
                        let response = String::from_utf8_lossy(&buffer[..size]);
                        // println!("response: {}", response);

                        // extract the players from the response
                        if let Ok(val) = serde_json::from_str::<Value>(&response) {
                            if val["status"] == "success" {
                                // Update cache
                                if let Some(players) = val.get("players").and_then(|v| v.as_array())
                                {
                                    self.cached_players = players
                                        .iter()
                                        .filter_map(|p| p.as_str().map(String::from))
                                        .collect();
                                }
                                if let Some(started) =
                                    val.get("game_started").and_then(|v| v.as_bool())
                                {
                                    println!("game started: {}", started);
                                    self.cached_game_state = started;
                                    
                                }
                                success = true;
                            }
                        }
                    }
                    Ok(_) => {
                        // No data received yet, sleep briefly
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        // println!("read error: {}", e);
                        break;
                    }
                }
            }

            return success;
        }

        false
    }

    pub fn get_players(&self) -> &Vec<String> {
        &self.cached_players
    }

    pub fn game_started(&self) -> bool {
        self.cached_game_state
    }

    pub fn send_start_game(&mut self, variant: &str) -> bool {
        if !self.is_connected() {
            return false;
        }

        let start_msg = json!({
            "action": "start",
            "variant": variant
        })
        .to_string();

        if let Some(ref mut stream) = self.stream {
            stream.write(start_msg.as_bytes()).is_ok()
        } else {
            false
        }
    }

    pub fn send_player_action(&mut self, variant: &str, action: &str, amount: Option<i32>) -> bool {
        let action_msg = match amount {
            Some(bet_amount) => json!({
                "action": "player_action",
                "variant": variant,
                "player_action": action,
                "amount": bet_amount
            }),
            None => json!({
                "action": "player_action",
                "variant": variant,
                "player_action": action
            }),
        }
        .to_string();

        if let Some(stream) = &mut self.stream {
            if let Err(e) = stream.write(action_msg.as_bytes()) {
                println!("Failed to send player action: {}", e);
                return false;
            }
            return true;
        }
        false
    }


    pub fn fetch_game_state_update(
        &mut self,
        variant: &str,
        username: &str,
    ) -> Option<GameStateUpdate> {


        if !self.is_connected() && !self.connect() {
            return None;
        }

        let request = json!({
            "action": "game_state_update",
            "variant": variant,
            "username": username
        })
        .to_string();

        if let Some(ref mut stream) = self.stream {
            if stream.write_all(request.as_bytes()).is_err() {
                self.stream = None;
                return None;
            }

            let mut buffer = [0; 4096];
            let start_time = Instant::now();

            while start_time.elapsed().as_millis() < 500 {
                match stream.read(&mut buffer) {
                    Ok(size) if size > 0 => {
                        let response = String::from_utf8_lossy(&buffer[..size]);
                        match from_str::<GameStateUpdate>(&response) {
                            Ok(update) => return Some(update),
                            Err(e) => {
                                eprintln!("Failed to parse GameStateUpdate: {}", e);
                                eprintln!("Raw response: {}", response);
                                return None;
                            }
                        }
                    }
                    Ok(_) => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        }

        None
    }
}
