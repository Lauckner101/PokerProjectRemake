use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::connection_manager::ConnectionManager;
use crate::GameState;

// Game state structures to deserialize server responses
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

#[derive(Debug, Deserialize, Serialize)]
struct PlayerState {
    name: String,
    cards: Vec<String>,
    chips: i32,
    current_bet: i32,
    folded: bool,
    is_active: bool,
    face_up_index: Vec<usize>, // Stores indices of face-up cards
}

// Structure to hold loaded card textures
struct CardTextures {
    cards: HashMap<String, Texture2D>,
    card_back: Texture2D,
}

impl CardTextures {
    async fn load() -> Self {
        let mut cards = HashMap::new();
        let suits = vec!["hearts", "diamonds", "clubs", "spades"];
        let values = vec![
            "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king", "ace",
        ];

        for suit in suits {
            for value in &values {
                let card_id = format!("{}_of_{}", value, suit);
                let texture_path = format!("assets/cards/{}_of_{}.png", value, suit);
                if let Ok(texture) = load_texture(&texture_path).await {
                    cards.insert(card_id, texture);
                } else {
                    eprintln!("Failed to load texture: {}", texture_path);
                }
            }
        }

        let card_back = match load_texture("assets/cards/card_back.png").await {
            Ok(texture) => texture,
            Err(e) => {
                eprintln!("Failed to load card back: {}", e);
                load_texture("assets/cards/card_back.png").await.unwrap() // Fallback
            }
        };

        Self { cards, card_back }
    }

    fn get_card(&self, card_id: &str) -> &Texture2D {
        self.cards.get(card_id).unwrap_or(&self.card_back)
    }
}

#[derive(Debug, PartialEq)]
pub enum GameVariant {
    FiveCardDraw,
    SevenCardStud,
    TexasHoldEm,
}

impl GameVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameVariant::FiveCardDraw => "five_card_draw",
            GameVariant::SevenCardStud => "seven_card_stud",
            GameVariant::TexasHoldEm => "texas_holdem",
        }
    }
}

async fn select_game_variant() -> GameVariant {
    loop {
        clear_background(WHITE);
        let screen_width = screen_width();
        let screen_height = screen_height();

        // Draw title
        let title = "Select Game Variant";
        let title_width = measure_text(title, None, 40, 1.0).width;
        draw_text(
            title,
            screen_width / 2.0 - title_width / 2.0,
            100.0,
            40.0,
            BLACK,
        );

        // Button dimensions
        let button_width = 200.0;
        let button_height = 50.0;
        let spacing = 20.0;

        // Calculate starting y position to center buttons vertically
        let total_height = (button_height * 3.0) + (spacing * 2.0);
        let start_y = (screen_height - total_height) / 2.0;
        let start_x = screen_width / 2.0 - button_width / 2.0;

        // Five Card Draw button
        draw_rectangle(start_x, start_y, button_width, button_height, BLUE);
        let text = "Five Card Draw";
        let text_width = measure_text(text, None, 25, 1.0).width;
        draw_text(
            text,
            screen_width / 2.0 - text_width / 2.0,
            start_y + 35.0,
            25.0,
            WHITE,
        );

        // Seven Card Stud button
        let seven_y = start_y + button_height + spacing;
        draw_rectangle(start_x, seven_y, button_width, button_height, BLUE);
        let text = "Seven Card Stud";
        let text_width = measure_text(text, None, 25, 1.0).width;
        draw_text(
            text,
            screen_width / 2.0 - text_width / 2.0,
            seven_y + 35.0,
            25.0,
            WHITE,
        );

        // Texas Hold'em button
        let texas_y = seven_y + button_height + spacing;
        draw_rectangle(start_x, texas_y, button_width, button_height, BLUE);
        let text = "Texas Hold'em";
        let text_width = measure_text(text, None, 25, 1.0).width;
        draw_text(
            text,
            screen_width / 2.0 - text_width / 2.0,
            texas_y + 35.0,
            25.0,
            WHITE,
        );

        // Check for button clicks
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();

            if mouse_x >= start_x && mouse_x <= start_x + button_width {
                if mouse_y >= start_y && mouse_y <= start_y + button_height {
                    return GameVariant::FiveCardDraw;
                }
                if mouse_y >= seven_y && mouse_y <= seven_y + button_height {
                    return GameVariant::SevenCardStud;
                }
                if mouse_y >= texas_y && mouse_y <= texas_y + button_height {
                    return GameVariant::TexasHoldEm;
                }
            }
        }

        next_frame().await;
    }
}

pub async fn game_screen(game_state: &mut GameState) {

    let username = game_state.username.as_deref().unwrap_or("");
    
    let game_variant = select_game_variant().await;
    let mut connection = ConnectionManager::new();

    // Try to join the room
    if let Some(username) = &game_state.username {
        if !connection.join_room(game_variant.as_str(), username) {
            eprintln!("Failed to join game room");
            return;
        }
    } else {
        eprintln!("Error: Username is not set in game_state.");
        return;
    }

    

    let background_texture = load_texture("assets/gameTable.png").await.unwrap();
    let card_textures = CardTextures::load().await;

    // Bet input state
    let mut bet_amount = 0;
    let mut show_bet_input = false;


    let mut game = GameStateUpdate {
        players: vec![],
        community_cards: vec![],
        current_player: String::new(),
        pot: 0,
        current_bet: 0,
        phase: String::new(),
        winner: None,
    };

    loop {
        
        clear_background(WHITE);
        let screen_width = screen_width();
        let screen_height = screen_height();

        let variant_text = match game_variant {
            GameVariant::FiveCardDraw => "Five-Card Draw",
            GameVariant::SevenCardStud => "Seven-Card Stud",
            GameVariant::TexasHoldEm => "Texas Hold 'Em",
        };

        let players = connection.get_players().clone(); // Clone the players to avoid holding the immutable borrow
        let game_started = connection.game_started();


        
        // Display title based on game state
        let state_text = if game_started {
            // poll for game state updates
            if let Some(username) = game_state.username.as_deref() {
                if connection.should_poll() {
                    if let Some(updated_game) = connection.fetch_game_state_update(game_variant.as_str(), username) {
                        game = updated_game;
                    }
                }



            } else {
                eprintln!("Error: Username is not set in game_state.");
                return;
            }

            format!("{} - Game In Progress", variant_text)
        } else {
            // Poll for updates only in waiting room mode
            if let Some(username) = game_state.username.as_deref() {
                connection.update_players(game_variant.as_str(), username);
            } else {
                eprintln!("Error: Username is not set in game_state.");
                return;
            }

            format!("{} - Waiting for Players", variant_text)
        };

        

        // Always display the title
        let text_width = measure_text(&state_text, None, 30, 1.0).width;
        draw_text(
            &state_text,
            screen_width / 2.0 - text_width / 2.0,
            30.0,
            30.0,
            BLACK,
        );


        // Split into two distinct display modes
        if !game_started {
            // WAITING ROOM MODE
            display_waiting_room(
                players.clone(),
                screen_width,
                screen_height,
                &mut connection,
                game_variant.as_str(),
            );
        } else {
            // GAME IN PROGRESS MODE
            // Draw game table background
            let bg_width = background_texture.width();
            let bg_height = background_texture.height();
            let x_pos = (screen_width - bg_width) / 2.0;
            let y_pos = 220.0;
            draw_texture(&background_texture, x_pos, y_pos, WHITE);


            display_game_state(
                &game,
                &card_textures,
                screen_width,
                screen_height,
                username,
                &mut connection,
                &game_variant,
                &mut bet_amount,
                &mut show_bet_input
            );
        }
        

        // Always display back button
        let button_x = screen_width / 2.0 - 100.0;
        let button_y = 700.0;
        let button_width = 200.0;
        let button_height = 50.0;
        draw_rectangle(button_x, button_y, button_width, button_height, RED);

        let back_text = "Back";
        let back_text_width = measure_text(back_text, None, 30, 1.0).width;
        draw_text(
            back_text,
            screen_width / 2.0 - back_text_width / 2.0,
            button_y + 30.0,
            30.0,
            WHITE,
        );

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            if mouse_x > button_x
                && mouse_x < button_x + button_width
                && mouse_y > button_y
                && mouse_y < button_y + button_height
            {
                break;
            }
        }

        // Process keyboard input for bet amount
        if show_bet_input {
            let digit = get_pressed_digit();
            if let Some(d) = digit {
                bet_amount = bet_amount * 10 + d;
            }

            // Backspace to delete a digit
            if is_key_pressed(KeyCode::Backspace) && bet_amount > 0 {
                bet_amount /= 10;
            }

            // Enter to confirm bet
            if is_key_pressed(KeyCode::Enter) {
                connection.send_player_action(game_variant.as_str(), "bet", Some(bet_amount));
                show_bet_input = false;
                bet_amount = 0;
            }

            // Escape to cancel bet
            if is_key_pressed(KeyCode::Escape) {
                show_bet_input = false;
                bet_amount = 0;
            }
        }

        next_frame().await;
    }
}

// New function to display the waiting room interface
fn display_waiting_room(
    players: Vec<String>,
    screen_width: f32,
    screen_height: f32,
    connection: &mut ConnectionManager,
    game_variant_str: &str,
) {
    // Display current players
    display_players(&players, screen_width, 130.0);

    // Start game button (only visible when enough players)
    if players.len() >= 2 {
        let start_button_width = 200.0;
        let start_button_height = 40.0;
        let start_button_x = screen_width / 2.0 - start_button_width / 2.0;
        let start_button_y = 75.0;

        draw_rectangle(
            start_button_x,
            start_button_y,
            start_button_width,
            start_button_height,
            GREEN,
        );

        let start_text = "Start Game";
        let start_text_width = measure_text(start_text, None, 25, 1.0).width;
        draw_text(
            start_text,
            screen_width / 2.0 - start_text_width / 2.0,
            start_button_y + 27.0,
            25.0,
            WHITE,
        );

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            if mouse_x > start_button_x
                && mouse_x < start_button_x + start_button_width
                && mouse_y > start_button_y
                && mouse_y < start_button_y + start_button_height
            {
                if connection.send_start_game(game_variant_str) {
                    println!("Start game message sent to server");
                } else {
                    println!("Failed to send start game message");
                }
            }
        }
    }

}

// Helper function to get pressed digit
fn get_pressed_digit() -> Option<i32> {
    for i in 0..10 {
        if is_key_pressed(match i {
            0 => KeyCode::Key0,
            1 => KeyCode::Key1,
            2 => KeyCode::Key2,
            3 => KeyCode::Key3,
            4 => KeyCode::Key4,
            5 => KeyCode::Key5,
            6 => KeyCode::Key6,
            7 => KeyCode::Key7,
            8 => KeyCode::Key8,
            9 => KeyCode::Key9,
            _ => unreachable!(),
        }) {
            return Some(i);
        }
    }
    None
}

// Function to display the game state
fn display_game_state(
    game_state: &GameStateUpdate,
    card_textures: &CardTextures,
    screen_width: f32,
    screen_height: f32,
    current_player_name: &str,
    connection: &mut ConnectionManager,
    game_variant: &GameVariant,
    bet_amount: &mut i32,
    show_bet_input: &mut bool,
) {
    // Display game phase
    let phase_text = format!("Phase: {}", game_state.phase);
    let phase_width = measure_text(&phase_text, None, 25, 1.0).width;
    draw_text(
        &phase_text,
        screen_width / 2.0 - phase_width / 2.0,
        80.0,
        25.0,
        BLACK,
    );

    // Display pot
    let pot_text = format!("Pot: ${}", game_state.pot);
    let pot_width = measure_text(&pot_text, None, 25, 1.0).width;
    draw_text(
        &pot_text,
        screen_width / 2.0 - pot_width / 2.0,
        110.0,
        25.0,
        BLACK,
    );

    // Display current bet
    let bet_text = format!("Current Bet: ${}", game_state.current_bet);
    let bet_width = measure_text(&bet_text, None, 20, 1.0).width;
    draw_text(
        &bet_text,
        screen_width / 2.0 - bet_width / 2.0,
        135.0,
        20.0,
        BLACK,
    );

    // Display whose turn it is
    if game_state.current_player == current_player_name {
        let turn_text = "YOUR TURN";
        let turn_width = measure_text(turn_text, None, 30, 1.0).width;
        draw_text(
            turn_text,
            screen_width / 2.0 - turn_width / 2.0,
            160.0,
            30.0,
            RED,
        );
    } else {
        let turn_text = format!("{}'s turn", game_state.current_player);
        let turn_width = measure_text(&turn_text, None, 25, 1.0).width;
        draw_text(
            &turn_text,
            screen_width / 2.0 - turn_width / 2.0,
            160.0,
            25.0,
            DARKBLUE,
        );
    }

    // Draw community cards (center of table)
    if !game_state.community_cards.is_empty() {
        let card_width = 60.0;
        let card_height = 90.0;
        let spacing = 10.0;
        let total_width = (card_width * game_state.community_cards.len() as f32)
            + (spacing * (game_state.community_cards.len() - 1) as f32);
        let start_x = (screen_width - total_width) / 2.0;
        let center_y = 300.0;

        for (i, card) in game_state.community_cards.iter().enumerate() {
            let card_x = start_x + (i as f32 * (card_width + spacing));
            let texture = card_textures.get_card(card);
            draw_texture_ex(
                texture,
                card_x,
                center_y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(card_width, card_height)),
                    ..Default::default()
                },
            );
        }
    }

    // Draw player positions and cards
    let table_center_x = screen_width / 2.0;
    let table_center_y = 350.0;
    let radius = 300.0;
    let player_count = game_state.players.len();

    for (i, player) in game_state.players.iter().enumerate() {
        let angle = (i as f32 / player_count as f32) * 2.0 * std::f32::consts::PI;
        let is_current_player = player.name == current_player_name;

        // Position player around a circle
        let player_x = table_center_x + radius * angle.cos();
        let player_y = table_center_y + radius * angle.sin();

        // Draw player name and chips
        let name_text = format!("{}", player.name);
        let name_width = measure_text(&name_text, None, 20, 1.0).width;
        let text_color = if is_current_player { GREEN } else { BLACK };

        draw_text(
            &name_text,
            player_x - name_width / 2.0,
            player_y - 50.0,
            20.0,
            text_color,
        );

        let chips_text = format!("${}", player.chips);
        let chips_width = measure_text(&chips_text, None, 15, 1.0).width;
        draw_text(
            &chips_text,
            player_x - chips_width / 2.0,
            player_y - 30.0,
            15.0,
            text_color,
        );

        if player.folded {
            let folded_text = "FOLDED";
            let folded_width = measure_text(folded_text, None, 15, 1.0).width;
            draw_text(
                folded_text,
                player_x - folded_width / 2.0,
                player_y - 10.0,
                15.0,
                RED,
            );
        } else if player.current_bet > 0 {
            let bet_text = format!("Bet: ${}", player.current_bet);
            let bet_width = measure_text(&bet_text, None, 15, 1.0).width;
            draw_text(
                &bet_text,
                player_x - bet_width / 2.0,
                player_y - 10.0,
                15.0,
                DARKBLUE,
            );
        }

        if !player.cards.is_empty() {
            // Draw player cards
            let card_width = 40.0;
            let card_height = 60.0;
            let card_spacing = 5.0;
            let total_card_width = (card_width * player.cards.len() as f32)
                + (card_spacing * (player.cards.len() - 1) as f32);
            let start_card_x = player_x - total_card_width / 2.0;

            for (card_idx, card) in player.cards.iter().enumerate() {
                let card_x = start_card_x + (card_idx as f32 * (card_width + card_spacing));

                // Only show face-up cards for current player or if they're revealed or if they are designated face up
                let texture = if is_current_player || game_state.phase == "showdown" {
                    card_textures.get_card(card)
                } else if player.face_up_index.contains(&card_idx) {
                    card_textures.get_card(card)
                } else {
                    &card_textures.card_back
                };
    

                draw_texture_ex(
                    texture,
                    card_x,
                    player_y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(card_width, card_height)),
                        ..Default::default()
                    },
                );
            }
        }
    }

    // If winner is declared, show it
    if let Some(winner) = &game_state.winner {
        let winner_text = format!("Winner: {}", winner);
        let winner_width = measure_text(&winner_text, None, 40, 1.0).width;

        // Draw background for winner text
        draw_rectangle(
            screen_width / 2.0 - winner_width / 2.0 - 20.0,
            screen_height / 2.0 - 30.0,
            winner_width + 40.0,
            60.0,
            Color::new(0.0, 0.5, 0.0, 0.8),
        );

        draw_text(
            &winner_text,
            screen_width / 2.0 - winner_width / 2.0,
            screen_height / 2.0 + 10.0,
            40.0,
            WHITE,
        );
    }

    // Display action buttons if it's the current player's turn
    if game_state.current_player == current_player_name && !*show_bet_input {
        display_action_buttons(
            screen_width,
            600.0,
            connection,
            game_variant,
            game_state.current_bet,
            bet_amount,
            show_bet_input,
        );
    }

    // Display bet input if active
    if *show_bet_input {
        display_bet_input(screen_width, 600.0, *bet_amount);
    }
}

// Function to display action buttons for the current player
fn display_action_buttons(
    screen_width: f32,
    y_position: f32,
    connection: &mut ConnectionManager,
    game_variant: &GameVariant,
    current_bet: i32,
    bet_amount: &mut i32,
    show_bet_input: &mut bool,
) {
    let button_width = 100.0;
    let button_height = 40.0;
    let spacing = 20.0;
    let total_width = (button_width * 3.0) + (spacing * 2.0);
    let start_x = (screen_width - total_width) / 2.0;

    // Fold button
    draw_rectangle(start_x, y_position, button_width, button_height, RED);
    let fold_text = "Fold";
    let fold_width = measure_text(fold_text, None, 20, 1.0).width;
    draw_text(
        fold_text,
        start_x + button_width / 2.0 - fold_width / 2.0,
        y_position + 25.0,
        20.0,
        WHITE,
    );

    // Check/Call button
    let check_x = start_x + button_width + spacing;
    draw_rectangle(check_x, y_position, button_width, button_height, BLUE);
    let check_text = if current_bet > 0 { "Call" } else { "Check" };
    let check_width = measure_text(check_text, None, 20, 1.0).width;
    draw_text(
        check_text,
        check_x + button_width / 2.0 - check_width / 2.0,
        y_position + 25.0,
        20.0,
        WHITE,
    );

    // Bet/Raise button
    let bet_x = check_x + button_width + spacing;
    draw_rectangle(bet_x, y_position, button_width, button_height, GREEN);
    let bet_text = if current_bet > 0 { "Raise" } else { "Bet" };
    let bet_width = measure_text(bet_text, None, 20, 1.0).width;
    draw_text(
        bet_text,
        bet_x + button_width / 2.0 - bet_width / 2.0,
        y_position + 25.0,
        20.0,
        WHITE,
    );

    // Check for button clicks
    if is_mouse_button_pressed(MouseButton::Left) {
        let (mouse_x, mouse_y) = mouse_position();

        // Fold button
        if mouse_x >= start_x
            && mouse_x <= start_x + button_width
            && mouse_y >= y_position
            && mouse_y <= y_position + button_height
        {
            connection.send_player_action(game_variant.as_str(), "fold", None);
        }

        // Check/Call button
        if mouse_x >= check_x
            && mouse_x <= check_x + button_width
            && mouse_y >= y_position
            && mouse_y <= y_position + button_height
        {
            connection.send_player_action(
                game_variant.as_str(),
                if current_bet > 0 { "call" } else { "check" },
                None,
            );
        }

        // Bet/Raise button
        if mouse_x >= bet_x
            && mouse_x <= bet_x + button_width
            && mouse_y >= y_position
            && mouse_y <= y_position + button_height
        {
            *show_bet_input = true;
            *bet_amount = 0;
        }
    }
}

// Function to display bet input UI
fn display_bet_input(screen_width: f32, y_position: f32, bet_amount: i32) {
    let input_width = 300.0;
    let input_height = 80.0;
    let x_pos = (screen_width - input_width) / 2.0;

    // Draw input background
    draw_rectangle(
        x_pos,
        y_position,
        input_width,
        input_height,
        Color::new(0.2, 0.2, 0.2, 0.8),
    );

    // Draw bet amount
    let amount_text = format!("Bet Amount: ${}", bet_amount);
    let amount_width = measure_text(&amount_text, None, 25, 1.0).width;
    draw_text(
        &amount_text,
        screen_width / 2.0 - amount_width / 2.0,
        y_position + 30.0,
        25.0,
        WHITE,
    );

    // Draw instructions
    let instruction_text = "Enter amount, press Enter to confirm";
    let instruction_width = measure_text(instruction_text, None, 15, 1.0).width;
    draw_text(
        instruction_text,
        screen_width / 2.0 - instruction_width / 2.0,
        y_position + 60.0,
        15.0,
        WHITE,
    );
}

// Function to display the list of players in the room
fn display_players(players: &Vec<String>, screen_width: f32, y_position: f32) {
    // Draw a header for the player list
    let header_text = "Players in Room:";
    let header_text_width = measure_text(header_text, None, 25, 1.0).width;
    draw_text(
        header_text,
        screen_width / 2.0 - header_text_width / 2.0,
        y_position,
        25.0,
        BLACK,
    );

    // Draw each player name
    let player_y_start = y_position + 30.0;
    let player_spacing = 25.0;

    for (i, player) in players.iter().enumerate() {
        let player_text = format!("• {}", player);
        let player_text_width = measure_text(&player_text, None, 20, 1.0).width;
        draw_text(
            &player_text,
            screen_width / 2.0 - player_text_width / 2.0,
            player_y_start + (i as f32 * player_spacing),
            20.0,
            DARKBLUE,
        );
    }
}
