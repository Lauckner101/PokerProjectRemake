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

#[derive(Default)]
pub struct GameState {
    pub username: Option<String>,
    pub game_started: bool,
    pub connection: ConnectionManager,
    pub phase: String,
    pub current_player: String,
}

enum Screen {
    Login,
    MainMenu,
    Game,
    Help,
}

#[macroquad::main("Main Menu")]
async fn main() {
    set_window_size(1000, 800);

    let mut game_state = GameState::default();

    let background_texture = load_texture("assets/menuBackground.png").await.unwrap();

    let mut current_screen = Screen::Login;

    let mut username = String::new();
    let mut password = String::new();

    let mut username_active = false;
    let mut password_active = false;

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

                let title_text = "Login";
                let title_width = measure_text(title_text, None, 50, 1.0).width;
                draw_text(
                    title_text,
                    screen_center_x - title_width / 2.0,
                    50.0,
                    50.0,
                    WHITE,
                );

                let input_width = 400.0;
                let input_height = 40.0;
                let input_x = screen_center_x - input_width / 2.0;

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

                let cursor_x = if username_active {
                    input_x + 10.0 + measure_text(&username, None, 30, 1.0).width
                } else {
                    input_x + 10.0 + measure_text(&"*".repeat(password.len()), None, 30, 1.0).width
                };

                let cursor_y = if username_active { 205.0 } else { 305.0 };
                if cursor_blink_timer > 0.5 {
                    draw_text("|", cursor_x, cursor_y, 30.0, WHITE);
                }

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

                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mouse_x, mouse_y) = mouse_position();

                    if mouse_x > button_x && mouse_x < button_x + button_width {
                        if mouse_y > 400.0 && mouse_y < 450.0 {
                            if create_user(&username, &password, &mut game_state) {
                                current_screen = Screen::MainMenu;
                            }
                        } else if mouse_y > 475.0 && mouse_y < 525.0 {
                            if login_user(&username, &password, &mut game_state) {
                                current_screen = Screen::MainMenu;
                            }
                        }
                    }
                }

                cursor_blink_timer += get_frame_time();
                if cursor_blink_timer > 1.0 {
                    cursor_blink_timer = 0.0;
                }
            }
            Screen::MainMenu => {
                let title_text = "Poker Game";
                let title_width = measure_text(title_text, None, 50, 1.0).width;
                draw_text(
                    title_text,
                    screen_width / 2.0 - title_width / 2.0,
                    50.0,
                    50.0,
                    WHITE,
                );

                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mouse_x, mouse_y) = mouse_position();

                    if mouse_x > screen_width / 2.0 - 100.0 && mouse_x < screen_width / 2.0 + 100.0
                    {
                        if mouse_y > 100.0 && mouse_y < 150.0 {
                            current_screen = Screen::Game;
                        } else if mouse_y > 200.0 && mouse_y < 250.0 {
                            current_screen = Screen::Help;
                        } else if mouse_y > 300.0 && mouse_y < 350.0 {
                            ping_server();
                        } else if mouse_y > 400.0 && mouse_y < 450.0 {
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
                    (button_x, 100.0, "Game"),
                    (button_x, 200.0, "Help"),
                    (button_x, 300.0, "Ping"),
                    (button_x, 400.0, "Logout"),
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
            Screen::Game => {
                game_screen(&mut game_state).await;
                current_screen = Screen::MainMenu;
            }
            Screen::Help => {
                let text = "Help Screen";
                let text_width = measure_text(text, None, 30, 1.0).width;
                draw_text(
                    text,
                    screen_width / 2.0 - text_width / 2.0,
                    50.0,
                    30.0,
                    WHITE,
                );

                let rules = [
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

fn create_user(username: &str, password: &str, game_state: &mut GameState) -> bool {
    let msg = json!({
        "action": "register",
        "username": username,
        "password": password,
    })
    .to_string();

    if send_to_server(&msg) {
        game_state.username = Some(username.to_string());
        game_state.connection = ConnectionManager::new();
        true
    } else {
        false
    }
}

fn login_user(username: &str, password: &str, game_state: &mut GameState) -> bool {
    let msg = json!({
        "action": "login",
        "username": username,
        "password": password,
    })
    .to_string();

    if send_to_server(&msg) {
        game_state.username = Some(username.to_string());
        game_state.connection = ConnectionManager::new();
        true
    } else {
        false
    }
}

fn send_to_server(msg: &str) -> bool {
    let server_address = "127.0.0.1:8080";

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

    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(3))) {
        println!("Failed to set read timeout: {}", e);
        return false;
    }
    if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(3))) {
        println!("Failed to set write timeout: {}", e);
        return false;
    }

    let framed_msg = format!("{}\n", msg);
    let start = Instant::now();

    if let Err(e) = stream.write_all(framed_msg.as_bytes()) {
        println!("Failed to write message: {}", e);
        return false;
    }

    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(size) => {
            let response = String::from_utf8_lossy(&buffer[..size]);
            match serde_json::from_str::<serde_json::Value>(&response) {
                Ok(val) => val["status"] == "success",
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
    let server_address = "127.0.0.1:8080";

    match TcpStream::connect(server_address) {
        Ok(mut stream) => {
            println!("Connected to server at {}", server_address);

            let msg = "Hello, server!";
            if let Err(e) = stream.write(msg.as_bytes()) {
                eprintln!("Failed to send message: {}", e);
                return;
            }

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