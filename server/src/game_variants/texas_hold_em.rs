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

pub struct TexasHoldEm {
    pub players: Vec<Player>,
    pub dealer: Dealer,
    pub bet_pool: u32,
    pub clients: Vec<Arc<Mutex<TcpStream>>>,
    pub skip: bool,
    pub starting_player_index: usize,
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

impl TexasHoldEm {
    pub fn new(players: Vec<Player>, clients: Vec<Arc<Mutex<TcpStream>>>) -> Self {
        let dealer = Dealer::new();
        Self {
            players,
            dealer,
            bet_pool: 0,
            clients, // Save the streams
            skip: false,
            starting_player_index: 0,

        }
    }

    pub fn deal_hole_cards(&mut self) {
        for player in &mut self.players {
            player.add_cards(self.dealer.deal_cards(2));
        }
    }

    pub fn deal_flop(&mut self) {
        self.dealer.deal_community_card();
        self.dealer.deal_community_card();
        self.dealer.deal_community_card();
    }

    pub fn deal_turn(&mut self) {
        self.dealer.deal_community_card();
    }

    pub fn deal_river(&mut self) {
        self.dealer.deal_community_card();
    }

    pub fn play(&mut self) {
        for player in &mut self.players {
            player.start_new_game();
        }

        self.dealer.shuffle_deck();

        println!("Starting a new game of Texas Hold 'Em!");

        if !self.skip {
            self.collect_move("Ante");
        }

        self.deal_hole_cards();

        if !self.skip {
            self.collect_move("Pre-Flop");
        }


        if self
            .players
            .iter()
            .filter(|player| player.is_active)
            .count()
            >= 2
        {
            self.deal_flop();
            if !self.skip {
                self.collect_move("Flop");
            }

        }

        if self
            .players
            .iter()
            .filter(|player| player.is_active)
            .count()
            >= 2
        {
            self.deal_turn();
            if !self.skip {
                self.collect_move("Turn");
            }
        }

        if self
            .players
            .iter()
            .filter(|player| player.is_active)
            .count()
            >= 2
        {
            self.deal_river();
            if !self.skip {
                self.collect_move("River");
            }
        }

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
                        self.players[j].add_winnings(winnings_per_player)
                    }
                }
            }
            println!("They each win {}!", winnings_per_player);
        }

        self.bet_pool = 0; // Reset the pot after winnings are distributed
    }




    fn collect_move(&mut self, phase: &str) {
        println!("{} betting round. You have the following options:", phase);
        println!("1. 'fold' - Fold your hand and forfeit this round.");
        println!("2. 'check' - If your current bet equals the highest bet, you can check.");
        println!("3. 'call' - Match the highest bet if your current bet is smaller.");
        println!("4. [bet_amount] - Enter an amount to raise the bet.");
        println!("5. 'skip' - Skip this betting round entirely.");
        println!();
    
        // Prompt for skip
        print!("Type 'skip' to skip this round or press Enter to continue: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim().eq_ignore_ascii_case("skip") {
            self.skip = true;
            println!("Skipping the {} round.", phase);
            return;
        }
  


        loop {
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

            // Display community cards
            let community_cards = self.dealer.get_community_cards();
            println!("Community cards: ");
            if community_cards.is_empty() {
                println!("No community cards yet.");
            } else {
                for card in community_cards.iter() {
                    print!("{} ", card); // Using the Display trait to print each card
                }
            }
            println!();

            // Iterate over each player to get their move
            let num_players = self.players.len();
            for offset in 0..num_players {
                let i = (self.starting_player_index + offset) % num_players;
                if !self.players[i].is_active() {
                    continue;
                }
            
                // Clone current player's name
                let current_player_name = self.players[i].name.clone();
            
                self.send_game_state_update(phase, &current_player_name, None);
            
                let player = &mut self.players[i];
            
                // Now it's safe to use mutable borrow
                // Display player's hand
                println!("{}'s hand: ", player.name);
                for card in &player.hand {
                    print!("|{}| ", card);
                }
                println!();
                println!("\n{}'s current bet: {}", player.name, player.current_bet);
                println!("Highest bet: {}", highest_bet);
                println!();
            
                let mut valid_action = false;
            
                while !valid_action {
                    print!("{}'s action: ", player.name);
                    io::stdout().flush().unwrap();
            
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
                                println!("{} cannot check because they need to match the highest bet.", player.name);
                            }
                        } else if action == "call" {
                            if player.current_bet < highest_bet {
                                player.call(highest_bet);
                                println!("{} calls with {}.", player.name, highest_bet - player.current_bet);
                                valid_action = true;
                            } else {
                                println!("{} cannot call because their bet is already equal to or greater than the highest bet.", player.name);
                            }
                        } else {
                            match action.parse::<u32>() {
                                Ok(amount) => {
                                    if amount > highest_bet {
                                        player.bet(amount);
                                        println!("{} raises to {}", player.name, amount);
                                        highest_bet = amount;
                                        valid_action = true;
                                    } else if amount == highest_bet {
                                        player.bet(amount);
                                        println!("{} calls the bet (check).", player.name);
                                        valid_action = true;
                                    } else {
                                        println!("{} cannot bet less than the current highest bet.", player.name);
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

            let highest_bet = self
                .players
                .iter()
                .filter(|p| p.is_active())
                .map(|p| p.current_bet)
                .max()
                .unwrap_or(0);
    
            let all_matched = self
                .players
                .iter()
                .filter(|p| p.is_active())
                .all(|p| p.current_bet == highest_bet);
    
            if all_matched || self.players.iter().filter(|p| p.is_active()).count() <= 1 {
                break;
            }
    
            println!("\nNot all bets match. Looping again...\n");


        }

        self.starting_player_index = (self.starting_player_index + 1) % self.players.len();
    }


    pub fn determine_best_hand_for_player(&self, player: &Player) -> u32 {
        let all_cards: Vec<Card> = player
            .hand
            .iter()
            .chain(self.dealer.get_community_cards().iter())
            .cloned()
            .collect();
        HandEvaluator::evaluate_hand_strength(&all_cards)
    }

    pub fn determine_winner(&self) -> Vec<&Player> {
        let mut best_hand_value = u32::MAX;
        let mut best_players: Vec<&Player> = Vec::new();

        for player in &self.players {
            if !player.is_active() {
                continue;
            }
            let hand_value = self.determine_best_hand_for_player(player);
            if hand_value < best_hand_value {
                best_hand_value = hand_value;
                best_players = vec![player];
            } else if hand_value == best_hand_value {
                best_players.push(player);
            }
        }

        if best_players.len() > 1 {
            let highest_card = best_players
                .iter()
                .map(|p| self.get_highest_card_for_player(p))
                .max()
                .unwrap_or(0);
            best_players.retain(|p| self.get_highest_card_for_player(p) == highest_card);

            if best_players.len() > 1 {
                let second_highest_card = best_players
                    .iter()
                    .map(|p| self.get_second_highest_card_for_player(p))
                    .max()
                    .unwrap_or(0);
                best_players
                    .retain(|p| self.get_second_highest_card_for_player(p) == second_highest_card);
            }
        }

        best_players
    }




    fn get_highest_card_for_player(&self, player: &Player) -> u32 {
        // Logic to get the highest card of the player's hand.
        let mut hand = player.hand.clone();
        hand.sort_by(|a, b| a.rank.cmp(&b.rank)); // Sort the cards by rank
        hand.last().unwrap().rank as u32 // Return the rank of the highest card
    }

    fn get_second_highest_card_for_player(&self, player: &Player) -> u32 {
        // Logic to get the second highest card of the player's hand.
        let mut hand = player.hand.clone();
        hand.sort_by(|a, b| a.rank.cmp(&b.rank)); // Sort the cards by rank
        hand[hand.len() - 2].rank as u32 // Return the rank of the second-highest card
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