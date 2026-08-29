use crate::game_screen::GameStateUpdate;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default)]
struct SharedState {
    players: Vec<String>,
    game_started: bool,
    last_game_update: Option<GameStateUpdate>,
    last_error: Option<String>,
}

pub struct ConnectionManager {
    stream: Option<TcpStream>,
    shared: Arc<Mutex<SharedState>>,
    last_poll_time: Instant,
    poll_interval: Duration,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ConnectionManager {
    fn drop(&mut self) {
        // unblock stream.try_clone() handle so doesnt sit in blocking read
        if let Some(stream) = &self.stream {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            stream: None,
            shared: Arc::new(Mutex::new(SharedState::default())),
            last_poll_time: Instant::now(),
            poll_interval: Duration::from_secs(5),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn connect(&mut self) -> bool {
        let server_address = "127.0.0.1:8080";
        let stream = match TcpStream::connect(server_address) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Connection failed: {}", e);
                return false;
            }
        };

        let read_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to clone stream: {}", e);
                return false;
            }
        };

        self.stream = Some(stream);
        println!("Connected to server at {}", server_address);

 
        let shared = self.shared.clone();
        thread::spawn(move || {
            let reader = BufReader::new(read_stream);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        if let Ok(val) = serde_json::from_str::<Value>(&line) {
                            Self::dispatch(&shared, val);
                        }
                    }
                    Err(_) => break, // connection closed/reset
                }
            }
        });

        true
    }

    fn dispatch(shared: &Arc<Mutex<SharedState>>, val: Value) {
        let mut state = shared.lock().unwrap();

        if val.get("phase").is_some() && val.get("current_player").is_some() {
            if let Ok(update) = serde_json::from_value::<GameStateUpdate>(val) {
                state.last_game_update = Some(update);
            }
            return;
        }

        if val.get("status").and_then(|s| s.as_str()) == Some("error") {
            state.last_error = val.get("message").and_then(|m| m.as_str()).map(String::from);
            return;
        }

        if let Some(players) = val.get("players").and_then(|v| v.as_array()) {
            state.players = players
                .iter()
                .filter_map(|p| p.as_str().map(String::from))
                .collect();
        }
        if let Some(started) = val.get("game_started").and_then(|v| v.as_bool()) {
            state.game_started = started;
        }
    }

    pub fn join_room(&mut self, variant: &str, username: &str) -> bool {
        if !self.is_connected() && !self.connect() {
            return false;
        }

        let join_msg = format!(
            "{}\n",
            json!({
                "action": "join",
                "variant": variant,
                "username": username
            })
        );

        if let Some(ref mut stream) = self.stream {
            if stream.write_all(join_msg.as_bytes()).is_err() {
                self.stream = None;
                return false;
            }
            // The background reader thread will pick up the join ack
            thread::sleep(Duration::from_millis(150));
            true
        } else {
            false
        }
    }

    pub fn update_players(&mut self, variant: &str, username: &str) -> bool {
        if !self.is_connected() && !self.connect() {
            return false;
        }
        let request = format!(
            "{}\n",
            json!({
                "action": "players",
                "variant": variant,
                "username": username
            })
        );

        if let Some(ref mut stream) = self.stream {
            stream.write_all(request.as_bytes()).is_ok()
        } else {
            false
        }
    }

    pub fn get_players(&self) -> Vec<String> {
        self.shared.lock().unwrap().players.clone()
    }

    pub fn game_started(&self) -> bool {
        self.shared.lock().unwrap().game_started
    }

    pub fn send_start_game(&mut self, variant: &str) -> bool {
        if !self.is_connected() {
            return false;
        }
        let start_msg = format!("{}\n", json!({ "action": "start", "variant": variant }));
        if let Some(ref mut stream) = self.stream {
            stream.write_all(start_msg.as_bytes()).is_ok()
        } else {
            false
        }
    }

    pub fn send_player_action(&mut self, variant: &str, username: &str, action: &str, amount: Option<i32>) -> bool {
        let action_msg = format!(
            "{}\n",
            match amount {
                Some(bet_amount) => json!({
                    "action": "player_action",
                    "variant": variant,
                    "username": username,
                    "player_action": action,
                    "amount": bet_amount
                }),
                None => json!({
                    "action": "player_action",
                    "variant": variant,
                    "username": username,
                    "player_action": action
                }),
            }
        );

        if let Some(stream) = &mut self.stream {
            if let Err(e) = stream.write_all(action_msg.as_bytes()) {
                println!("Failed to send player action: {}", e);
                return false;
            }
            true
        } else {
            false
        }
    }

    pub fn take_last_error(&mut self) -> Option<String> {
        self.shared.lock().unwrap().last_error.take()
    }

    pub fn should_poll(&mut self) -> bool {
        if self.last_poll_time.elapsed() >= self.poll_interval {
            self.last_poll_time = Instant::now();
            true
        } else {
            false
        }
    }

    // Sends an explicit request for the server's cached last state. useful as a resilience fallback
    pub fn request_game_state_update(&mut self, variant: &str, username: &str) {
        if !self.is_connected() && !self.connect() {
            return;
        }
        let request = format!(
            "{}\n",
            json!({
                "action": "game_state_update",
                "variant": variant,
                "username": username
            })
        );
        if let Some(ref mut stream) = self.stream {
            let _ = stream.write_all(request.as_bytes());
        }
    }

    // read of whatever the latest known game state is
    pub fn get_latest_game_update(&self) -> Option<GameStateUpdate> {
        self.shared.lock().unwrap().last_game_update.clone()
    }
}