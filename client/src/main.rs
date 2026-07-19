use crate::miniquad::window::set_window_size;
use macroquad::prelude::*;
use serde_json::json;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
mod connection_manager;
mod game_screen;
use connection_manager::ConnectionManager;
use game_screen::game_screen;

#[derive(Clone)]
pub struct PlayerStats {
    pub game_number: u32,
    pub total_bets: u32,
    pub total_winnings: u32,
}

#[derive(Default)]
pub struct GameState {
    pub username: Option<String>,
    pub game_started: bool,
    pub connection: ConnectionManager, // Add this line
    pub phase: String,
    pub current_player: String,
}

enum Screen {
    Login,
    MainMenu,
    Statistics,
    Game,
    Help,
}

#[macroquad::main("Main Menu")]
async fn main() {
    // Set the desired window size
    set_window_size(1000, 800);

    let mut game_state = GameState::default();

    // Load the background image
    let background_texture = load_texture("assets/menuBackground.png").await.unwrap();

    let mut current_screen = Screen::Login;

    let mut player_stats: Vec<PlayerStats> = Vec::new();

    let mut username = String::new();
    let mut password = String::new();

    let mut username_active = false;
    let mut password_active = false;

   

     //for search other users stats
    let mut target_username = String::new();
    let mut target_username_active = false;
    let mut cursor_blink_timer = 0.0;

    loop {
        clear_background(BLACK);
        let screen_width = screen_width();
        let screen_height = screen_height();
        draw_texture_ex(
            &background_texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width, screen_height)),
                ..Default::default()
            },
        );

        match current_screen {
            Screen::Login => {
                let screen_center_x = screen_width / 2.0;

                // Draw the title at the top
                let title_text = "Login";
                let title_width = measure_text(title_text, None, 50, 1.0).width;
                draw_text(
                    title_text,
                    screen_center_x - title_width / 2.0,
                    50.0,
                    50.0,
                    WHITE,
                );

                // Input field dimensions
                let input_width = 400.0;
                let input_height = 40.0;
                let input_x = screen_center_x - input_width / 2.0;

                // Username input box
                let username_label = "Username:";
                let username_label_width = measure_text(username_label, None, 30, 1.0).width;
                draw_text(
                    username_label,
                    screen_center_x - username_label_width / 2.0,
                    150.0,
                    30.0,
                    WHITE,
                );

                draw_rectangle(input_x, 180.0, input_width, input_height, DARKGRAY);
                draw_text(&username, input_x + 10.0, 205.0, 30.0, WHITE);

                // Password input box
                let password_label = "Password:";
                let password_label_width = measure_text(password_label, None, 30, 1.0).width;
                draw_text(
                    password_label,
                    screen_center_x - password_label_width / 2.0,
                    250.0,
                    30.0,
                    WHITE,
                );

                draw_rectangle(input_x, 280.0, input_width, input_height, DARKGRAY);
                draw_text(
                    &"*".repeat(password.len()),
                    input_x + 10.0,
                    305.0,
                    30.0,
                    WHITE,
                );

                // Draw cursor if active
                let cursor_x = if username_active {
                    input_x + 10.0 + measure_text(&username, None, 30, 1.0).width
                } else {
                    input_x + 10.0 + measure_text(&"*".repeat(password.len()), None, 30, 1.0).width
                };

                let cursor_y = if username_active { 205.0 } else { 305.0 };
                if cursor_blink_timer > 0.5 {
                    draw_text("|", cursor_x, cursor_y, 30.0, WHITE);
                }

                // Buttons for Create User or Login
                let button_width = 200.0;
                let button_height = 50.0;
                let button_x = screen_center_x - button_width / 2.0;

                let button_positions =
                    vec![(button_x, 400.0, "Create User"), (button_x, 475.0, "Login")];

                for (x, y, label) in button_positions {
                    draw_rectangle(x, y, button_width, button_height, RED);
                    let text_width = measure_text(label, None, 30, 1.0).width;
                    draw_text(
                        label,
                        x + (button_width - text_width) / 2.0,
                        y + (button_height + 10.0) / 2.0,
                        30.0,
                        WHITE,
                    );
                }

                // Handle text input for username and password
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mouse_x, mouse_y) = mouse_position();

                    if mouse_x > input_x && mouse_x < input_x + input_width {
                        if mouse_y > 180.0 && mouse_y < 220.0 {
                            username_active = true;
                            password_active = false;
                        } else if mouse_y > 280.0 && mouse_y < 320.0 {
                            username_active = false;
                            password_active = true;
                        } else {
                            username_active = false;
                            password_active = false;
                        }
                    }
                }

                // Text input handling
                let backspace_repeat_delay = 0.1;
                let mut backspace_timer = 0.0;
                let current_time = get_time();

                if username_active {
                    if is_key_pressed(KeyCode::Backspace) && !username.is_empty() {
                        username.pop();
                        backspace_timer = current_time + backspace_repeat_delay;
                    } else if is_key_down(KeyCode::Backspace)
                        && current_time > backspace_timer
                        && !username.is_empty()
                    {
                        username.pop();
                        backspace_timer = current_time + backspace_repeat_delay;
                    } else if let Some(input) = get_char_pressed() {
                        username.push(input);
                    }
                }

                if password_active {
                    if is_key_pressed(KeyCode::Backspace) && !password.is_empty() {
                        password.pop();
                        backspace_timer = current_time + backspace_repeat_delay;
                    } else if is_key_down(KeyCode::Backspace)
                        && current_time > backspace_timer
                        && !password.is_empty()
                    {
                        password.pop();
                        backspace_timer = current_time + backspace_repeat_delay;
                    } else if let Some(input) = get_char_pressed() {
                        password.push(input);
                    }
                }

                // Check if the user pressed either button
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mouse_x, mouse_y) = mouse_position();

                    if mouse_x > button_x && mouse_x < button_x + button_width {
                        if mouse_y > 400.0 && mouse_y < 450.0 {
                            if create_user(&username, &password) {
                                current_screen = Screen::MainMenu;
                            }
                        } else if mouse_y > 475.0 && mouse_y < 525.0 {
                            if login_user(&username, &password, &mut game_state) {
                                current_screen = Screen::MainMenu;
                            }
                        }
                    }
                }

                // Update cursor blink timer
                cursor_blink_timer += get_frame_time();
                if cursor_blink_timer > 1.0 {
                    cursor_blink_timer = 0.0;
                }
            }
            Screen::MainMenu => {
                // Draw the title at the top
                let title_text = "Poker Game";
                let title_width = measure_text(title_text, None, 50, 1.0).width;
                draw_text(
                    title_text,
                    screen_width / 2.0 - title_width / 2.0,
                    50.0,
                    50.0,
                    WHITE,
                );

                // Handle button clicks
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mouse_x, mouse_y) = mouse_position();

                    if mouse_x > screen_width / 2.0 - 100.0 && mouse_x < screen_width / 2.0 + 100.0
                    {
                        if mouse_y > 100.0 && mouse_y < 150.0 {
                            println!("Switching to statistics screen!");
                            current_screen = Screen::Statistics;
                            //update_statistics(&mut player_stats);

                            if let Some(name) = &game_state.username {
                                println!("Calling update_statistics for {}", name);
                                update_statistics(&mut player_stats, name);
                            }
                            
                        } else if mouse_y > 200.0 && mouse_y < 250.0 {
                            current_screen = Screen::Game;
                        } else if mouse_y > 300.0 && mouse_y < 350.0 {
                            current_screen = Screen::Help;
                        } else if mouse_y > 400.0 && mouse_y < 450.0 {
                            ping_server();
                        } else if mouse_y > 500.0 && mouse_y < 550.0 {
                            username.clear();
                            password.clear();
                            current_screen = Screen::Login;
                        }
                    }
                }

                let button_width = 200.0;
                let button_height = 50.0;
                let button_x = screen_width / 2.0 - button_width / 2.0;
                let button_positions = vec![
                    (button_x, 100.0, "Statistics"),
                    (button_x, 200.0, "Game"),
                    (button_x, 300.0, "Help"),
                    (button_x, 400.0, "Ping"),
                    (button_x, 500.0, "Logout"),
                ];

                for (x, y, label) in button_positions {
                    draw_rectangle(x, y, button_width, button_height, RED);
                    let text_width = measure_text(label, None, 30, 1.0).width;
                    draw_text(
                        label,
                        x + (button_width - text_width) / 2.0,
                        y + (button_height + 10.0) / 2.0,
                        30.0,
                        WHITE,
                    );
                }
            }
            Screen::Statistics => {
                // Statistics screen
                let text = "Statistics Screen";
                let text_width = measure_text(text, None, 30, 1.0).width;
                draw_text(
                    text,
                    screen_width / 2.0 - text_width / 2.0,
                    50.0,
                    30.0,
                    WHITE,
                );

                // Display the statistics
                let mut y_offset = 100.0;
                //println!("Rendering stats screen. Player stats count: {}", player_stats.len());

                for stat in player_stats.iter() {
                    let stats_text = format!(
                        "Game: {} | Bets: {} | Winnings: {}",
                        stat.game_number, stat.total_bets, stat.total_winnings
                    );
                    //println!("Rendering {} player stats...", player_stats.len());

                    draw_text(
                        &stats_text,
                        screen_width / 2.0 - measure_text(&stats_text, None, 20, 1.0).width / 2.0,
                        y_offset,
                        20.0,
                        WHITE,
                    );
                    y_offset += 30.0;
                }
                //search for other users stats
                // Label
                let prompt = "Enter a username to view their stats:";
                let prompt_width = measure_text(prompt, None, 25, 1.0).width;
                draw_text(prompt, screen_width / 2.0 - prompt_width / 2.0, 470.0, 25.0, WHITE);

                // Input box
                let input_x = 300.0;
                let input_y = 480.0;
                let input_width = 400.0;
                let input_height = 40.0;

                draw_rectangle(input_x, input_y, input_width, input_height, DARKGRAY);
                draw_text(&target_username, input_x + 10.0, input_y + 30.0, 25.0, WHITE);

                // Show cursor when active
                if target_username_active && cursor_blink_timer > 0.5 {
                    let cursor_x = input_x + 10.0 + measure_text(&target_username, None, 25, 1.0).width;
                    draw_text("|", cursor_x, input_y + 30.0, 25.0, WHITE);
                }

                // Detect input box activation
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mouse_x, mouse_y) = mouse_position();
            
                    // Clicked on input box
                    if mouse_x > input_x
                        && mouse_x < input_x + input_width
                        && mouse_y > input_y
                        && mouse_y < input_y + input_height
                    {
                        target_username_active = true;
                    } else {
                        target_username_active = false;
                    }
            
                    // Clicked Search button
                    if mouse_x > 720.0 && mouse_x < 820.0 && mouse_y > 480.0 && mouse_y < 520.0 {
                        println!("Searching stats for: {}", target_username);
                        update_statistics(&mut player_stats, &target_username);
                        // Clear the input box afterward
                        target_username.clear();
                    }
                }

                // Typing logic (same as login)
                if target_username_active {
                    // 1) Grab any typed chars
                    if let Some(c) = get_char_pressed() {
                        target_username.push(c);
                    }
            
                    // 2) Also check for special keys
                    for key in get_keys_pressed() {
                        match key {
                            KeyCode::Backspace => {
                                target_username.pop(); // remove last char
                            }
                            KeyCode::Delete => {
                                target_username.pop(); // treat delete like backspace
                            }
                            // Optional: handle Enter as “Search” right away
                            KeyCode::Enter => {
                                println!("Searching stats for: {}", target_username);
                                update_statistics(&mut player_stats, &target_username);
                                target_username.clear();
                            }
                            _ => {}
                        }
                    }
                    
                }

                // Search button
                draw_rectangle(720.0, 480.0, 100.0, 40.0, RED);
                draw_text("Search", 730.0, 510.0, 25.0, WHITE);

            
            

                // Back button
                if is_mouse_button_pressed(MouseButton::Left)
                    && mouse_position().0 > screen_width / 2.0 - 100.0
                    && mouse_position().0 < screen_width / 2.0 + 100.0
                    && mouse_position().1 > 350.0
                    && mouse_position().1 < 400.0
                {
                    current_screen = Screen::MainMenu;
                }

                draw_rectangle(screen_width / 2.0 - 100.0, 350.0, 200.0, 50.0, RED);
                let back_text = "Back";
                let back_text_width = measure_text(back_text, None, 30, 1.0).width;
                draw_text(
                    back_text,
                    screen_width / 2.0 - back_text_width / 2.0,
                    380.0,
                    30.0,
                    WHITE,
                );
            }
            Screen::Game => {
                game_screen(&mut game_state).await;
                current_screen = Screen::MainMenu;
            }
            Screen::Help => {
                // Help screen title
                let text = "Help Screen";
                let text_width = measure_text(text, None, 30, 1.0).width;
                draw_text(
                    text,
                    screen_width / 2.0 - text_width / 2.0,
                    50.0,
                    30.0,
                    WHITE,
                );

                // Poker Game Rules
                let rules = [
                    "Standard 5 Card Poker:",
                    "  - Each player is dealt 5 cards face down.",
                    "  - Players can exchange up to 3 cards to improve their hand.",
                    "  - The player with the highest hand wins.",
                    "",
                    "7 Card Stub Poker:",
                    "  - Each player is dealt 7 cards, 3 face up and 4 face down.",
                    "  - Players can exchange up to 3 face-down cards.",
                    "  - The player with the best 5-card hand wins.",
                    "",
                    "Texas Hold'em:",
                    "  - Each player is dealt 2 private cards (hole cards).",
                    "  - Five community cards are dealt face up on the board.",
                    "  - Players can use any combination of their hole cards and the community cards.",
                    "  - The player with the best 5-card hand wins."
                ];

                let mut y_offset = 100.0;
                for line in rules.iter() {
                    draw_text(
                        line,
                        screen_width / 2.0 - measure_text(line, None, 20, 1.0).width / 2.0,
                        y_offset,
                        20.0,
                        WHITE,
                    );
                    y_offset += 30.0;
                }

                // Position the back button further down
                let back_button_height = 50.0;
                let back_button_y = screen_height - back_button_height - 10.0;
                if is_mouse_button_pressed(MouseButton::Left)
                    && mouse_position().0 > screen_width / 2.0 - 100.0
                    && mouse_position().0 < screen_width / 2.0 + 100.0
                    && mouse_position().1 > back_button_y
                    && mouse_position().1 < back_button_y + back_button_height
                {
                    current_screen = Screen::MainMenu;
                }

                draw_rectangle(
                    screen_width / 2.0 - 100.0,
                    back_button_y,
                    200.0,
                    back_button_height,
                    RED,
                );
                let back_text = "Back";
                let back_text_width = measure_text(back_text, None, 30, 1.0).width;
                let back_text_y = back_button_y + (back_button_height + 10.0) / 2.0;
                draw_text(
                    back_text,
                    screen_width / 2.0 - back_text_width / 2.0,
                    back_text_y,
                    30.0,
                    WHITE,
                );
            }
        }

        next_frame().await;
    }
}

fn create_user(username: &str, password: &str) -> bool {
    let msg = json!({
        "action": "register",
        "username": username,
        "password": password,
    })
    .to_string();
    //game_state.username = Some(username.to_string()); // need to add gameState to this

    send_to_server(&msg)

}

fn login_user(username: &str, password: &str, game_state: &mut GameState) -> bool {
    let msg = json!({
        "action": "login",
        "username": username,
        "password": password,
    })
    .to_string();

    println!("about to send!");

    if send_to_server(&msg) {
        println!("sent");
        game_state.username = Some(username.to_string());
        game_state.connection = ConnectionManager::new(); // Initialize connection
        true
    } else {
        false
    }
}

// fn update_statistics(player_stats: &mut Vec<PlayerStats>) {
//     // ------------------------------------------------------------- placeholder function, should get server to update
//     player_stats.clear();
//     player_stats.push(PlayerStats {
//         game_number: 1,
//         total_bets: 100,
//         total_winnings: 150,
//     });
//     player_stats.push(PlayerStats {
//         game_number: 2,
//         total_bets: 200,
//         total_winnings: 250,
//     });
//     player_stats.push(PlayerStats {
//         game_number: 3,
//         total_bets: 150,
//         total_winnings: 100,
//     });
// }

fn update_statistics(player_stats: &mut Vec<PlayerStats>, username: &str) {
    println!("Updating statistics for user: {}", username);
    player_stats.clear();

    let request = json!({
        "action": "stats",
        "username": username
    })
    .to_string();

    let server_address = "127.0.0.1:8080";

    if let Ok(mut stream) = TcpStream::connect(server_address) {
        if stream.write_all(request.as_bytes()).is_ok() {
            let mut buffer = [0u8; 1024];
            if let Ok(size) = stream.read(&mut buffer) {
                let response = String::from_utf8_lossy(&buffer[..size]);
                //debug statement
println!("RAW STATS RESPONSE: {}", response);  // 🔍 ADD THIS


                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
                    if parsed["status"] == "success" {
                        if let Some(stats_array) = parsed["stats"].as_array() {
                            for entry in stats_array {
                                if let (Some(game_number), Some(bets), Some(winnings)) = (
                                    
                                        entry["game_number"].as_u64(), 
                                        entry["total_bets"].as_u64(),
                                        entry["total_winnings"].as_u64(),
                                    
                                    
                                ) {
                                    println!("--> Adding stat row: game {}, bets {}, wins {}", game_number, bets, winnings);  // 👈 Add this

                                    player_stats.push(PlayerStats {
                                        game_number: game_number as u32,
                                        total_bets: bets as u32,
                                        total_winnings: winnings as u32,
                                    });
                                }
                            }
                        } else {
                            println!("Parsed data was not an array");
                        }
                    } else {
                        println!("Stats response not success");
                    }
                } else {
                    println!("Failed to parse stats JSON response");
                }
            }
        }
    }
}



fn send_to_server(msg: &str) -> bool {
    let server_address = "127.0.0.1:8080";

    // 1. Set connection timeout
    println!("Attempting to connect to server...");
    let mut stream = match TcpStream::connect_timeout(
        &server_address.parse().unwrap(),
        Duration::from_secs(3),
    ) {
        Ok(s) => s,
        Err(e) => {
            println!("Connection failed: {}", e);
            return false;
        }
    };

    // 2. Set read/write timeouts
    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(3))) {
        println!("Failed to set read timeout: {}", e);
        return false;
    }
    if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(3))) {
        println!("Failed to set write timeout: {}", e);
        return false;
    }

    println!("Connected to server. About to write message...");

    // 3. Write with timeout check
    let start = Instant::now();
    match stream.write(msg.as_bytes()) {
        Ok(_) => println!("Message sent successfully!"),
        Err(e) => {
            println!("Failed to write message: {}", e);
            return false;
        }
    }

    println!("Waiting for response...");

    // 4. Read response with timeout
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(size) => {
            println!("Received {} bytes", size);
            let response = String::from_utf8_lossy(&buffer[..size]);
            println!("Raw response: {}", response);

            match serde_json::from_str::<serde_json::Value>(&response) {
                Ok(val) => {
                    println!("Parsed response: {:?}", val);
                    val["status"] == "success"
                }
                Err(e) => {
                    println!("Failed to parse JSON: {}", e);
                    false
                }
            }
        }
        Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
            println!("Read timed out after {:?}", start.elapsed());
            false
        }
        Err(e) => {
            println!("Read error: {}", e);
            false
        }
    }
}

fn ping_server() {
    // The server address
    let server_address = "127.0.0.1:8080";

    // Connect to the server
    match TcpStream::connect(server_address) {
        Ok(mut stream) => {
            println!("Connected to server at {}", server_address);

            // Send a message to the server
            let msg = "Hello, server!";
            if let Err(e) = stream.write(msg.as_bytes()) {
                eprintln!("Failed to send message: {}", e);
                return;
            }

            // Read the response from the server
            let mut buffer = [0; 1024];
            match stream.read(&mut buffer) {
                Ok(bytes_read) => {
                    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                    println!("Received from server: {}", response);
                }
                Err(e) => eprintln!("Failed to read response: {}", e),
            }
        }
        Err(e) => eprintln!("Failed to connect to server: {}", e),
    }
}



mod tests {
    #[cfg(test)]
    pub mod tests;
}
