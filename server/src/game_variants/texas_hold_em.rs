use crate::card::Card;
use crate::card::Rank;
use crate::card::Suit;
use crate::dealer::Dealer;
use crate::hand_evaluator::HandEvaluator;
use crate::player::Player;
use itertools::Itertools;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct TexasHoldEm {
    pub players: Vec<Player>,
    pub dealer: Dealer,
    pub bet_pool: u32,
    pub clients: HashMap<String, Arc<Mutex<TcpStream>>>,
    pub action_receivers: HashMap<String, Receiver<Value>>,
    pub skip: bool,
    pub starting_player_index: usize,
    pub last_state: Option<GameStateUpdate>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GameStateUpdate {
    players: Vec<PlayerState>,
    community_cards: Vec<String>,
    current_player: String,
    pot: i32,
    current_bet: i32,
    phase: String,
    winner: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PlayerState {
    name: String,
    cards: Vec<String>,
    chips: i32,
    current_bet: i32,
    folded: bool,
    is_active: bool,
    face_up_index: Vec<usize>,
}

impl TexasHoldEm {
    pub fn new(
        players: Vec<Player>,
        clients: HashMap<String, Arc<Mutex<TcpStream>>>,
        action_receivers: HashMap<String, Receiver<Value>>,
    ) -> Self {
        let dealer = Dealer::new();
        Self {
            players,
            dealer,
            bet_pool: 0,
            clients,
            action_receivers,
            skip: false,
            starting_player_index: 0,
            last_state: None,
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

        if self.players.iter().filter(|player| player.is_active).count() >= 2 {
            self.deal_flop();
            if !self.skip {
                self.collect_move("Flop");
            }
        }

        if self.players.iter().filter(|player| player.is_active).count() >= 2 {
            self.deal_turn();
            if !self.skip {
                self.collect_move("Turn");
            }
        }

        if self.players.iter().filter(|player| player.is_active).count() >= 2 {
            self.deal_river();
            if !self.skip {
                self.collect_move("River");
            }
        }

        let winners = self.determine_winner();
        let num_winners = winners.len() as u32;

        if num_winners == 0 {
            println!("No active players remain — hand ends with no winner.");
            self.bet_pool = 0;
            self.send_game_state_update("showdown", "Nobody", None);
            return;
        }

        let winner_names: Vec<String> = winners.iter().map(|p| p.name.clone()).collect();
        let winnings_per_player = self.bet_pool / num_winners;

        if winner_names.len() == 1 {
            println!("The winner is {}!", winner_names[0]);
            println!("They win {}!", winnings_per_player);
            for p in &mut self.players {
                if p.name == winner_names[0] {
                    p.add_winnings(winnings_per_player);
                }
            }
            self.send_game_state_update("showdown", "Nobody", Some(winner_names[0].clone()));
        } else {
            println!("It's a tie between the following players:");
            for name in &winner_names {
                println!("{}", name);
                for p in &mut self.players {
                    if &p.name == name {
                        p.add_winnings(winnings_per_player);
                    }
                }
            }
            println!("They each win {}!", winnings_per_player);
            self.send_game_state_update(
                "showdown",
                "Nobody",
                Some(format!("Tie: {}", winner_names.join(", "))),
            );
        }

        self.bet_pool = 0;
    }

    fn send_invalid_move(stream_arc: &Arc<Mutex<TcpStream>>, message: &str) {
        if let Ok(mut s) = stream_arc.lock() {
            let msg = serde_json::json!({
                "status": "error",
                "message": message
            })
            .to_string();
            let _ = s.write_all(format!("{}\n", msg).as_bytes());
        }
    }

    fn collect_move(&mut self, phase: &str) {
        println!("{} betting round.", phase);

        'betting: loop {
            println!("Current pot: {}", self.bet_pool);

            let mut highest_bet = self
                .players
                .iter()
                .filter(|player| player.is_active())
                .map(|player| player.current_bet)
                .max()
                .unwrap_or(0);

            let community_cards = self.dealer.get_community_cards();
            println!("Community cards: ");
            if community_cards.is_empty() {
                println!("No community cards yet.");
            } else {
                for card in community_cards.iter() {
                    print!("{} ", card);
                }
            }
            println!();

            let num_players = self.players.len();
            for offset in 0..num_players {
                let i = (self.starting_player_index + offset) % num_players;
                if !self.players[i].is_active() {
                    continue;
                }

                if self.players.iter().filter(|p| p.is_active()).count() <= 1 {
                    break 'betting;
                }

                let current_player_name = self.players[i].name.clone();
                self.send_game_state_update(phase, &current_player_name, None);

                {
                    let player = &self.players[i];
                    println!("{}'s hand: ", player.name);
                    for card in &player.hand {
                        print!("|{}| ", card);
                    }
                    println!();
                    println!("\n{}'s current bet: {}", player.name, player.current_bet);
                    println!("Highest bet: {}", highest_bet);
                    println!();
                }

                let stream_for_errors = self.clients.get(&current_player_name).cloned();

                if !self.action_receivers.contains_key(&current_player_name) {
                    self.players[i].fold();
                    println!("{} auto-folded (no connection).", current_player_name);
                    continue;
                }

                let mut valid_action = false;
                while !valid_action {
                    let recv_result = self
                        .action_receivers
                        .get(&current_player_name)
                        .unwrap()
                        .recv_timeout(Duration::from_secs(60));

                    match recv_result {
                        Ok(json_msg) => {
                            let action = json_msg
                                .get("player_action")
                                .and_then(|a| a.as_str())
                                .unwrap_or("")
                                .to_lowercase();

                            if action == "__disconnect__" {
                                println!("{} disconnected — auto-folding.", current_player_name);
                                self.players[i].fold();
                                valid_action = true;
                                continue;
                            }

                            let amount = json_msg
                                .get("amount")
                                .and_then(|a| a.as_i64())
                                .map(|a| a as u32);

                            let player = &mut self.players[i];

                            if action == "fold" {
                                player.fold();
                                println!("{} has folded.", player.name);
                                valid_action = true;
                            } else if action == "check" {
                                if player.current_bet == highest_bet {
                                    println!("{} checks.", player.name);
                                    valid_action = true;
                                } else {
                                    println!("Invalid check — must match highest bet.");
                                    if let Some(s) = &stream_for_errors {
                                        Self::send_invalid_move(
                                            s,
                                            "Invalid move: you must call or fold, not check.",
                                        );
                                    }
                                }
                            } else if action == "call" {
                                if player.current_bet < highest_bet {
                                    player.call(highest_bet);
                                    println!("{} calls.", player.name);
                                    valid_action = true;
                                } else {
                                    println!("Invalid call.");
                                    if let Some(s) = &stream_for_errors {
                                        Self::send_invalid_move(
                                            s,
                                            "Invalid move: there's nothing to call.",
                                        );
                                    }
                                }
                            } else if action == "bet" || action == "raise" {
                                if let Some(bet_amount) = amount {
                                    if bet_amount > highest_bet {
                                        player.bet(bet_amount);
                                        println!("{} raises to {}", player.name, bet_amount);
                                        highest_bet = bet_amount;
                                        valid_action = true;
                                    } else if bet_amount == highest_bet {
                                        player.bet(bet_amount);
                                        println!("{} calls.", player.name);
                                        valid_action = true;
                                    } else {
                                        println!("Bet too low.");
                                        if let Some(s) = &stream_for_errors {
                                            Self::send_invalid_move(
                                                s,
                                                "Invalid move: bet must be higher than the current highest bet.",
                                            );
                                        }
                                    }
                                }
                            } else {
                                if let Some(s) = &stream_for_errors {
                                    Self::send_invalid_move(s, "Invalid move: unrecognized action.");
                                }
                            }
                        }
                        Err(_) => {
                            println!(
                                "Timed out waiting for action from {} — auto-folding.",
                                current_player_name
                            );
                            self.players[i].fold();
                            valid_action = true;
                        }
                    }
                }
            }

            self.bet_pool = self.players.iter().map(|player| player.current_bet).sum();

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
                break 'betting;
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
        let mut hand = player.hand.clone();
        hand.sort_by(|a, b| a.rank.cmp(&b.rank));
        hand.last().unwrap().rank as u32
    }

    fn get_second_highest_card_for_player(&self, player: &Player) -> u32 {
        let mut hand = player.hand.clone();
        hand.sort_by(|a, b| a.rank.cmp(&b.rank));
        hand[hand.len() - 2].rank as u32
    }

    pub fn send_game_state_update(
        &mut self,
        phase: &str,
        current_player: &str,
        winner_name: Option<String>,
    ) {
        let community_cards = self
            .dealer
            .get_community_cards()
            .iter()
            .map(|card| format!("{}", card).to_lowercase())
            .collect();

        let current_player = current_player.to_string();

        let highest_bet = self
            .players
            .iter()
            .map(|p| p.current_bet)
            .max()
            .unwrap_or(0) as i32;

        let player_states: Vec<PlayerState> = self
            .players
            .iter()
            .map(|player| PlayerState {
                name: player.name.clone(),
                cards: player
                    .hand
                    .iter()
                    .map(|card| format!("{}", card).to_lowercase())
                    .collect(),
                chips: player.get_winnings() as i32,
                current_bet: player.current_bet as i32,
                folded: !player.is_active(),
                is_active: player.is_active(),
                face_up_index: vec![],
            })
            .collect();

        let update = GameStateUpdate {
            players: player_states,
            community_cards,
            current_player,
            pot: self.bet_pool as i32,
            current_bet: highest_bet,
            phase: phase.to_string(),
            winner: winner_name,
        };

        self.last_state = Some(update.clone());

        let message = serde_json::to_string(&update).expect("Failed to serialize GameStateUpdate");
        let message_with_newline = format!("{}\n", message);

        self.clients.retain(|_, stream_arc| match stream_arc.lock() {
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
        });
    }
}