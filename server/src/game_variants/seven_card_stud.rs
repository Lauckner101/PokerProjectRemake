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

pub struct SevenCardStud {
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


impl SevenCardStud {
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

    pub fn deal_initial_cards(&mut self) {
        for player in &mut self.players {
            let cards = self.dealer.deal_cards(3);
            player.add_cards(vec![cards[0].clone(), cards[1].clone()]); // 2 face-down
            player.add_face_up_card(cards[2].clone()); // 1 face-up
        }
    }

    pub fn deal_face_up_card(&mut self) {
        for player in &mut self.players {
            if player.is_active() {
                let card = self.dealer.deal_cards(1)[0].clone();
                player.add_face_up_card(card);
            }
        }
    }

    pub fn deal_river(&mut self) {
        for player in &mut self.players {
            if player.is_active() {
                let card = self.dealer.deal_cards(1)[0].clone();
                player.add_cards(vec![card]);
            }
        }
    }

    pub fn play(&mut self) {
        for player in &mut self.players {
            player.start_new_game();
        }
        self.dealer.shuffle_deck();

        println!("Starting a new game of 7-Card Stud!");

        if !self.skip {
            self.collect_move("Ante");
        }

        self.deal_initial_cards();
        if !self.skip {
            self.collect_move("Third Street");
        }
 
        if self.players.iter().filter(|p| p.is_active()).count() >= 2 {
            self.deal_face_up_card();
            if !self.skip {
                self.collect_move("Fourth Street");
            }
        }

        if self.players.iter().filter(|p| p.is_active()).count() >= 2 {
            self.deal_face_up_card();
            if !self.skip {
                self.collect_move("Fifth Street");
            }
        }

        if self.players.iter().filter(|p| p.is_active()).count() >= 2 {
            self.deal_face_up_card();
            if !self.skip {
                self.collect_move("Sixth Street");
            }
        }

        if self.players.iter().filter(|p| p.is_active()).count() >= 2 {
            self.deal_river();
            if !self.skip {
                self.collect_move("Seventh Street");
            }
        }

        let winners: Vec<String> = self
            .determine_winner()
            .iter()
            .map(|player| player.name.clone())
            .collect();
        let num_winners = winners.len() as u32;
        let winnings_per_player = self.bet_pool / num_winners;

        if winners.len() == 1 {
            println!("The winner is {}!", winners[0]);
            println!("They win {}!", winnings_per_player);
            for player in &mut self.players {
                if player.name == winners[0] {
                    player.add_winnings(winnings_per_player);
                }
            }
            self.send_game_state_update("showdown", "Nobody", Some(winners[0].to_string()));
        } else {
            println!("It's a tie between the following players:");
            for winner_name in &winners {
                println!("{}", winner_name);
                for player in &mut self.players {
                    if player.name == *winner_name {
                        player.add_winnings(winnings_per_player);
                    }
                }
            }
            println!("They each win {}!", winnings_per_player);
        }

        self.bet_pool = 0;
    }

    fn collect_move(&mut self, phase: &str) {
        println!("{} betting round. Options:", phase);
        println!("1. 'fold' - Fold your hand.");
        println!("2. 'check' - If your bet matches the highest.");
        println!("3. 'call' - Match the highest bet.");
        println!("4. [amount] - Raise the bet.");
        println!("Current pot: {}", self.bet_pool);
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
            let mut highest_bet = self
                .players
                .iter()
                .filter(|p| p.is_active())
                .map(|p| p.current_bet)
                .max()
                .unwrap_or(0);
            println!("Highest bet: {}", highest_bet);
        
            let num_players = self.players.len();
            for offset in 0..num_players {
                let i = (self.starting_player_index + offset) % num_players;
                if !self.players[i].is_active() {
                    continue;
                }

                let current_player_name = self.players[i].name.clone();
                self.send_game_state_update(phase, &current_player_name, None);
        
                let player = &mut self.players[i];
        
                println!("{}'s hand:", player.name);
                println!("Face-down: {} cards", player.hand.len());
                println!("Face-up: {:?}", player.face_up_cards);
                println!("Current bet: {}", player.current_bet);
        
                let mut valid_action = false;
                while !valid_action {
                    print!("{}'s action: ", player.name);
                    io::stdout().flush().unwrap();
        
                    let mut action = String::new();
                    io::stdin().read_line(&mut action).unwrap();
                    let action = action.trim().to_lowercase();
        
                    match action.as_str() {
                        "fold" => {
                            player.fold();
                            println!("{} has folded.", player.name);
                            valid_action = true;
                        }
                        "check" => {
                            if player.current_bet == highest_bet {
                                println!("{} checks.", player.name);
                                valid_action = true;
                            } else {
                                println!("Cannot check; must match {}", highest_bet);
                            }
                        }
                        "call" => {
                            if player.current_bet < highest_bet {
                                player.call(highest_bet);
                                println!(
                                    "{} calls with {}.",
                                    player.name,
                                    highest_bet - player.current_bet
                                );
                                valid_action = true;
                            } else {
                                println!("Bet already matches or exceeds highest.");
                            }
                        }
                        _ => {
                            if let Ok(amount) = action.parse::<u32>() {
                                if amount >= highest_bet {
                                    player.bet(amount);
                                    println!("{} bets {}.", player.name, amount);
                                    highest_bet = amount;
                                    valid_action = true;
                                } else {
                                    println!("Bet must be at least {}.", highest_bet);
                                }
                            } else {
                                println!("Invalid input. Use 'fold', 'check', 'call', or [amount].");
                            }
                        }
                    }
                }
            }
        
            self.bet_pool = self
                .players
                .iter()
                .filter(|p| p.is_active())
                .map(|p| p.current_bet)
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



    pub fn determine_winner(&self) -> Vec<&Player> {
        let mut best_hand_value = u32::MAX;
        let mut best_players: Vec<&Player> = Vec::new();

        for player in &self.players {
            if !player.is_active() {
                continue;
            }

            let all_cards: Vec<Card> = player
                .hand
                .iter()
                .chain(player.face_up_cards.iter())
                .cloned()
                .collect();
            let hand_value = HandEvaluator::evaluate_hand_strength(&all_cards);

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
        let mut all_cards: Vec<Card> = player
            .hand
            .iter()
            .chain(player.face_up_cards.iter())
            .cloned()
            .collect();
        all_cards.sort_by(|a, b| a.rank.cmp(&b.rank));
        all_cards.last().unwrap().rank as u32
    }

    fn get_second_highest_card_for_player(&self, player: &Player) -> u32 {
        let mut all_cards: Vec<Card> = player
            .hand
            .iter()
            .chain(player.face_up_cards.iter())
            .cloned()
            .collect();
        all_cards.sort_by(|a, b| a.rank.cmp(&b.rank));
        all_cards[all_cards.len() - 2].rank as u32
    }


   pub fn send_game_state_update(&mut self, phase: &str, current_player: &str, winner_name: Option<String>) {
        let community_cards = self.dealer
            .get_community_cards()
            .iter()
            .map(|card| format!("{}", card).to_lowercase())
            .collect();

        let current_player = current_player.to_string();

        let highest_bet = self.players
            .iter()
            .map(|p| p.current_bet)
            .max()
            .unwrap_or(0) as i32;

        let player_states: Vec<PlayerState> = self.players.iter().map(|player| {
            let mut cards: Vec<String> = Vec::new();
            let mut face_up_index: Vec<usize> = Vec::new();

            // Add face-down cards first
            for card in &player.hand {
                cards.push(format!("{}", card).to_lowercase());
            }

            // Add face-up cards and record their indices
            for (i, card) in player.face_up_cards.iter().enumerate() {
                cards.push(format!("{}", card).to_lowercase());
                face_up_index.push(player.hand.len() + i); // indices are offset by face-down count
            }

            PlayerState {
                name: player.name.clone(),
                cards,
                chips: player.get_winnings() as i32,
                current_bet: player.current_bet as i32,
                folded: !player.is_active(),
                is_active: player.is_active(),
                face_up_index,
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
        let interval = Duration::from_millis(100);

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
