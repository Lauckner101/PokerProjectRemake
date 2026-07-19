use crate::card::{Card,Deck};
use crate::player::Player;
use crate::player::PlayerStats;
use crate::dealer::Dealer;
use std::collections::HashMap;
use std::io;
use std::io::Write;
pub struct Razz{
    dealer:Dealer,
    pub players:Vec<Player>,
    bet_pool: u32,

}

impl Razz{
    pub fn new(players: Vec<Player>) -> Self {
        //let players = player_names.into_iter().map(|name| Player::new(name)).collect();
        let dealer = Dealer::new();
        Self { players, dealer, bet_pool: 0 }
    }
    pub fn play(&mut self){
        for player in &mut self.players {
            player.start_new_game();
        }

        println!("Starting a game of razz");
        self.dealer.shuffle_deck();
        //3 initial cards, 2 hidden, 1 visible
        for player in &mut self.players{
            let mut hand = self.dealer.deal_cards(2); //hidden
            hand.push(self.dealer.deal_cards(1)[0]); //visible
            player.add_cards(hand);

        }
        println!("third street: 2 hidden cards and 1 visible card");
        self.show_player_hands();
        self.betting_round("Third Street");
        for _ in 0..3{
            for player in &mut self.players{
                if player.is_active{
                let card = self.dealer.deal_cards(1);
                player.add_cards(card);}
            }
            let street_name = match self.players[0].hand.len(){
                4 => "Fourth Street",
                5 => "Fifth Street",
                6 => "Sixth Street",
                7 => "Seventh Street",
                _ => "Unknown Street",
            };
            println!("{}: 1 card face up", street_name);
            self.show_player_hands();
            self.betting_round(street_name);
            self.players.retain(|p| p.is_active);//remove folded player 
        }
        println!("determining the winner");
        let winner = self.determine_winner();
        println!("winner is {}",winner);
    }

    fn betting_round(&mut self, round_name: &str) {
        println!("{} betting round. Enter your bets (or type 'fold' to fold):", round_name);
    
        let mut highest_bet = 0;
        
        // Determine betting order for Third Street (First round)
        let mut sorted_players = if round_name == "Third Street" {
            // Sort by highest visible card (worst low hand starts)
            let mut players = self.players.clone();
            players.sort_by_key(|p| p.hand.last().unwrap().rank as u32); // Sort by worst card
            players.reverse(); // Highest card goes first
            players
        } else {
            // Sort by lowest hand (best low hand starts)
            let mut players = self.players.clone();
            players.sort_by_key(|p| self.evaluate_razz_hand(&p.hand)); 
            players
        };
    
        for player in &mut sorted_players {
            if !player.is_active {
                continue;
            }
    
            println!(
                "{}'s turn. Current highest bet: {}. Your current bet: {}.",
                player.name, highest_bet, player.current_bet
            );
            println!("Enter action: 'fold', 'check', 'call', or 'bet <amount>'");
    
            io::stdout().flush().unwrap(); // Ensure prompt appears before input
    
            let mut action = String::new();
            match io::stdin().read_line(&mut action) {
                Ok(_) => {
                    let action = action.trim().to_string(); // Trim newline
    
                    match action.as_str() {
                        "fold" => {
                            player.fold();
                            println!("{} has folded.", player.name);
                        }
                        "check" => {
                            if highest_bet == 0 {
                                println!("{} checks.", player.name);
                            } else {
                                println!("Invalid action! There is an active bet. You must call or raise.");
                                println!("Auto-calling {} to stay in the hand.", highest_bet);
                                player.call(highest_bet);
                            }
                        }
                        "call" => {
                            if highest_bet == 0 {
                                println!("Nothing to call. You check instead.");
                            } else if player.current_bet < highest_bet {
                                let call_amount = highest_bet - player.current_bet;
                                player.bet(call_amount);
                                println!("{} calls and matches the highest bet of {}.", player.name, highest_bet);
                            } else {
                                println!("You have already matched the highest bet.");
                            }
                        }
                        _ if action.starts_with("bet ") => {
                            let amount: u32 = action[4..].trim().parse().unwrap_or(0);
                            if amount > highest_bet {
                                highest_bet = amount;
                                player.bet(amount);
                                println!("{} bets {}.", player.name, amount);
                            } else {
                                println!("Bet must be higher than the current highest bet.");
                            }
                        }
                        _ => {
                            println!("Invalid action. Please enter 'fold', 'check', 'call', or 'bet <amount>'.");
                        }
                    }
                }
                Err(e) => {
                    println!("DEBUG: Failed to read input: {}", e);
                }
            }
        }
    }
    
    fn determine_winner(&mut self) -> String {
        let mut player_scores: Vec<(&String, u32)> = self.players
            .iter()
            .filter(|p| p.is_active)
            .map(|p| (&p.name, self.evaluate_razz_hand(&p.hand)))
            .collect();
    
        player_scores.sort_by_key(|&(_, score)| score);
    
        let winner_name = player_scores[0].0.clone();
        //println!("DEBUG: Winner is: {} with hand value {}", winner_name, player_scores[0].1);
    
        for player in &mut self.players {
            if player.name == winner_name {
                player.winnings += 1; 
                if player.game_statistics.is_empty() {
                   
                    player.game_statistics.push(PlayerStats { total_bets: 0, total_winnings: 0 });
                }
                if let Some(stats) = player.game_statistics.last_mut() {
                   
                    stats.total_winnings += 1;
                    //println!("DEBUG: After update: total_bets = {}, total_winnings = {}", stats.total_bets, stats.total_winnings);
                }
            }
        }
    
        winner_name
    }
    
    
    
    
    
    fn evaluate_razz_hand(&self, hand: &[Card]) -> u32 {
        let mut sorted_hand = hand.to_vec();
        sorted_hand.sort_by(|a, b| a.rank.cmp(&b.rank));

        let hand_value = sorted_hand.iter().map(|c| c.rank as u32).sum();

        hand_value  // Lower hand value is better
    }
    fn show_player_hands(&self) {
        for player in &self.players {
            let visible_cards: Vec<String> = player.show_hand().iter().map(|c| format!("{:?}", c)).collect();
            println!("{}'s hand: {}", player.name, visible_cards.join(", "));
        }
    }

}