use crate::card::Card;
use crate::card::Rank;
use crate::card::Suit;
use crate::dealer::Dealer;
use crate::hand_evaluator::HandEvaluator;
use crate::player::Player;
use std::thread;
use std::time::Instant;
use itertools::Itertools;
use serde::Serialize;
use serde::Deserialize;
use serde_json::Value;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::{
    io::ErrorKind,
    //net::TcpListener,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct FiveCardDraw {
    players: Vec<Player>,
    dealer: Dealer,
    bet_pool: u32,
    pub clients: Vec<Arc<Mutex<TcpStream>>>,
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

impl FiveCardDraw {
    pub fn new(player_names: Vec<&str>, clients: Vec<Arc<Mutex<TcpStream>>>) -> Self {
        let players = player_names
            .into_iter()
            .map(|name| Player::new(name))
            .collect();
        let dealer = Dealer::new();
        Self {
            players,
            dealer,
            bet_pool: 0,
            clients, // Save the streams
        }
    }

    pub fn deal_initial_hands(&mut self) {
        for player in &mut self.players {
            player.add_cards(self.dealer.deal_cards(5));
        }
    }

    pub fn draw_phase(&mut self) {
        println!("Draw phase begins...");

        for i in 0..self.players.len() {
            if !self.players[i].is_active() {
                continue;
            }


            let current_player_name = self.players[i].name.clone();
        
            self.send_game_state_update("Draw", &current_player_name, None);
        
            let player = &mut self.players[i];

            loop {
                println!("{}'s hand: ", player.name);
                for card in &player.hand {
                    print!("|{}| ", card); // Print each card in player's hand
                }
                println!("\nEnter the positions (0-4) of the cards you want to discard, separated by spaces (or 'none' to keep all cards):");

                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read input");
                let input = input.trim().to_lowercase();

                if input == "none" {
                    break; // Player keeps all cards
                }

                let positions: Vec<usize> = input
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();

                if positions.len() > 3 {
                    println!("You can only discard up to 3 cards. Try again.");
                    continue; // Loop back for re-entry
                }

                if positions.iter().any(|&pos| pos >= player.hand.len()) {
                    println!("Invalid position entered. Try again.");
                    continue; // Loop back for re-entry
                }

                let mut new_cards = self.dealer.deal_cards(positions.len());
                for &pos in positions.iter().rev() {
                    if let Some(card) = new_cards.pop() {
                        player.hand[pos] = card;
                    }
                }

                print!("{}'s new hand: ", player.name);
                for card in &player.hand {
                    print!("|{}| ", card); // Print each card in player's hand
                }
                break; // Exit loop after a valid input and replacement
            }
        }
    }

    pub fn play(&mut self) {
        for player in &mut self.players {
            player.start_new_game();
        }

        self.dealer.shuffle_deck();

        self.betting_round("Ante");

        self.deal_initial_hands();

        // Pre-draw betting
        self.betting_round("Pre-draw");

        // Draw phase
        if self
            .players
            .iter()
            .filter(|player| player.is_active)
            .count()
            >= 2
        {
            // only continue game if there are at least 2 players active
            self.draw_phase();
            // Post-draw betting
            self.betting_round("Post-draw");
        }

        // Determine winner
        //let mut winners = self.determine_winner().clone();
        let num_winners = self.determine_winner().len() as u32;

        let winnings_per_player = self.bet_pool / num_winners;

        if self.determine_winner().len() == 1 {
            // If there's only one winner, display their name
            println!("The winner is {}!", self.determine_winner()[0].name);
            println!("They win {}!", winnings_per_player);
            self.send_game_state_update("showdown", "Nobody", Some(self.determine_winner()[0].name.to_string()));
        } else {
            // If there are multiple winners, display their names
            println!("It's a tie between the following players:");
            for i in 0..self.determine_winner().len() {
                println!("{}", self.determine_winner()[i].name);

                for j in 0..self.players.len() {
                    if self.players[j].name == self.determine_winner()[i].name {
                        self.players[j].add_winnings(winnings_per_player);
                        //  self.players[j].add_winnings(winnings_per_player, unsafe { GAME_NUMBER })
                    }
                }
            }
            println!("They each win {}!", winnings_per_player);
        }

        // save stats
        for player in &self.players {
            // if let Err(e) = player.save_stats() {
            //     eprintln!("Failed to save stats for {}: {}", player.name, e);
            // }
        }

        self.bet_pool = 0; // Reset the pot after winnings are distributed
    }

    fn betting_round(&mut self, phase: &str) {
        println!("{} betting round. You have the following options:", phase);
        println!("1. 'fold' - Fold your hand and forfeit this round.");
        println!("2. 'check' - If your current bet equals the highest bet, you can check.");
        println!("3. 'call' - Match the highest bet if your current bet is smaller.");
        println!("4. [bet_amount] - Enter an amount to raise the bet.");
        println!();

        // Display the current pot and highest bet
        println!("Current pot: {}", self.bet_pool);

        // Find the highest bet among active players
        let mut highest_bet = self
            .players
            .iter()
            .filter(|player| player.is_active()) // Filter only active players
            .map(|player| player.current_bet) // Get each player's current bet
            .max() // Find the highest bet
            .unwrap_or(0); // If no bets, default to 0

        println!();

        // Iterate over each player to get their move
        for i in 0..self.players.len() {
            if !self.players[i].is_active() {
                continue;
            }


            let current_player_name = self.players[i].name.clone();
        
            self.send_game_state_update(phase, &current_player_name, None);
        
            let player = &mut self.players[i];

            // Display player's hand
            println!("{}'s hand: ", player.name);
            for card in &player.hand {
                print!("|{}| ", card); // Print each card in player's hand
            }
            println!();
            println!("\n{}'s current bet: {}", player.name, player.current_bet);
            println!("Highest bet: {}", highest_bet);
            println!();

            let mut valid_action = false; // Flag to check for valid input

            while !valid_action {
                print!("{}'s action: ", player.name);
                io::stdout().flush().unwrap(); // Ensure the prompt is displayed before input

                let mut action = String::new();
                let stdin = io::stdin();

                if stdin.read_line(&mut action).is_ok() {
                    let action = action.trim().to_lowercase();

                    if action == "fold" {
                        player.fold();
                        println!("{} has folded.", player.name);
                        valid_action = true;
                    } else if action == "check" {
                        if player.current_bet == highest_bet {
                            println!("{} checks.", player.name);
                            valid_action = true;
                        } else {
                            println!(
                                "{} cannot check because they need to match the highest bet.",
                                player.name
                            );
                        }
                    } else if action == "call" {
                        // Calling means matching the highest bet
                        if player.current_bet < highest_bet {
                            player.call(highest_bet);
                            println!(
                                "{} calls with {}.",
                                player.name,
                                highest_bet - player.current_bet
                            );
                            valid_action = true;
                        } else {
                            println!("{} cannot call because their bet is already equal to or greater than the highest bet.", player.name);
                        }
                    } else {
                        // Parse the action as a bet
                        match action.parse::<u32>() {
                            Ok(amount) => {
                                if amount > highest_bet {
                                    player.bet(amount); // Raise the bet
                                    println!("{} raises to {}", player.name, amount);
                                    highest_bet = amount;
                                    valid_action = true;
                                } else if amount == highest_bet {
                                    player.bet(amount);
                                    println!("{} calls the bet (check).", player.name);
                                    valid_action = true;
                                } else {
                                    println!(
                                        "{} cannot bet less than the current highest bet.",
                                        player.name
                                    );
                                }
                            }
                            Err(_) => {
                                println!("Invalid input! Please enter a valid bet amount, 'call', or 'fold'.");
                            }
                        }
                    }
                }
            }
        }
        // update pool
        self.bet_pool = self
            .players
            .iter()
            .filter(|player| player.is_active())
            .map(|player| player.current_bet)
            .sum();
    }

    pub fn determine_winner(&self) -> Vec<&Player> {
        // ------------------------------------------- need to fix
        let mut best_hand_value = u32::MAX;
        let mut best_players: Vec<&Player> = Vec::new(); // Track all players with the best hand

        // For each player, determine their best hand and compare
        for player in &self.players {
            if !player.is_active() {
                continue;
            }

            // Determine the best hand for the player
            let hand_value = HandEvaluator::evaluate_hand_strength(&player.hand);

            // If the current player's hand is better than the best hand found so far
            if hand_value < best_hand_value {
                best_hand_value = hand_value;
                best_players = vec![player]; // New best hand, so reset the winners list
            }
            // If the current player's hand is equal to the best hand found so far, add them to the winners list
            else if hand_value == best_hand_value {
                best_players.push(player);
            }
            println!("hand: {}, player: {}", hand_value, player.name);
        }

        if best_players.len() > 1 {
            for i in 0..5 {
                let highest_card = best_players
                    .iter()
                    .map(|player| self.get_nth_highest_card_for_player(player, i))
                    .max()
                    .unwrap_or(0);

                // Keep only players who have this highest card at position `i`
                best_players.retain(|player| {
                    self.get_nth_highest_card_for_player(player, i) == highest_card
                });

                // If only one player remains, break early
                if best_players.len() == 1 {
                    break;
                }
            }
        }

        best_players // Return all players who are tied for the best hand
    }

    fn get_nth_highest_card_for_player(&self, player: &Player, n: usize) -> u32 {
        let mut hand = player.hand.clone();
        hand.sort_by(|a, b| a.rank.cmp(&b.rank)); // Sort by rank
        hand[hand.len() - 1 - n].rank as u32 // Return the nth highest card
    }



    pub fn send_game_state_update(&mut self, phase: &str, current_player: &str, winner_name: Option<String>) {
        let community_cards = self.dealer
            .get_community_cards()
            .iter()
            .map(|card| format!("{}", card).to_lowercase()) // proper format is lowercased
            .collect();
    
        let current_player = current_player.to_string();
    
        let highest_bet = self.players
            .iter()
            .map(|p| p.current_bet)
            .max()
            .unwrap_or(0) as i32;
    
        let player_states: Vec<PlayerState> = self.players.iter().map(|player| {
            PlayerState {
                name: player.name.clone(),
                cards: player.hand.iter().map(|card| format!("{}", card).to_lowercase()).collect(),
                chips: player.get_winnings() as i32, // or use a `chips` field if added
                current_bet: player.current_bet as i32,
                folded: !player.is_active(), // folded = inactive
                is_active: player.is_active(),
                face_up_index: vec![], // Currently unused
            }
        }).collect();
    
        let update = GameStateUpdate {
            players: player_states,
            community_cards,
            current_player,
            pot: self.bet_pool as i32,
            current_bet: highest_bet,
            phase: phase.to_string(),
            winner: winner_name,
        };
    
        let message = serde_json::to_string(&update).expect("Failed to serialize GameStateUpdate");
        let message_with_newline = format!("{}\n", message);
    
        let start_time = Instant::now();
        let duration = Duration::from_secs(5);
        let interval = Duration::from_millis(100); // ~10 updates over 5 seconds
    
        while start_time.elapsed() < duration {
            self.clients.retain_mut(|stream_arc| {
                match stream_arc.lock() {
                    Ok(mut stream) => match stream.write_all(message_with_newline.as_bytes()) {
                        Ok(_) => true,
                        Err(e) => {
                            eprintln!("Failed to send update to a client: {}", e);
                            false
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to lock stream for writing: {}", e);
                        false
                    }
                }
            });
    
            thread::sleep(interval);
        }
    }
}
