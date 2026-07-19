use crate::card::Card;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub winnings: u32,
    pub is_active: bool,
    pub current_bet: u32,
    pub game_statistics: Vec<PlayerStats>,
    #[serde(skip_serializing, skip_deserializing)]
    pub hand: Vec<Card>, // face-down cards
    #[serde(skip_serializing, skip_deserializing)]
    pub face_up_cards: Vec<Card>, // face-up cards for 7-Card Stud
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub total_bets: u32,     // total money bet in game
    pub total_winnings: u32, //  total winnings in game
}

impl Player {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            hand: Vec::new(),
            face_up_cards: Vec::new(),
            winnings: 0,
            is_active: true,
            current_bet: 0,
            game_statistics: Vec::new(), // vecotr of games each with player stats
        }
    }

    pub fn add_cards(&mut self, cards: Vec<Card>) {
        self.hand.extend(cards); // append the vector of cards to the back of the hand
    }

    pub fn add_face_up_card(&mut self, card: Card) {
        self.face_up_cards.push(card);
    }

    pub fn show_hand(&self) -> Vec<Card> {
        self.hand.clone()
    }

    pub fn show_face_up_cards(&self) -> Vec<Card> {
        self.face_up_cards.clone()
    }

    pub fn add_winnings(&mut self, amount: u32) {
        self.winnings += amount;
        if let Some(stats) = self.game_statistics.last_mut() {
            stats.total_winnings += amount;
        }
    }

    pub fn fold(&mut self) {
        self.is_active = false;
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn get_winnings(&self) -> u32 {
        self.winnings
    }

    pub fn bet(&mut self, amount: u32) {
        // this function is used for raising
        self.current_bet = amount;
        if let Some(stats) = self.game_statistics.last_mut() {
            stats.total_bets = self.current_bet;
        }
    }

    pub fn check(&self) -> bool {
        self.current_bet == 0
    }

    pub fn call(&mut self, highest_bet: u32) {
        // match highest bet
        self.bet(highest_bet);
    }

    pub fn clear_hand(&mut self) {
        self.hand.clear();
        self.face_up_cards.clear();
    }

    pub fn reset_for_new_round(&mut self) {
        self.hand.clear();
        self.face_up_cards.clear();
        self.is_active = true;
        self.current_bet = 0;
    }

    pub fn start_new_game(&mut self) {
        self.game_statistics.push(PlayerStats {
            total_bets: 0,
            total_winnings: 0,
        });
    }
}
