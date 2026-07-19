use crate::card::{Card, Deck};
use crate::player::Player;


pub struct Dealer {
    pub deck: Deck,
    pub community_cards: Vec<Card>,
}

impl Dealer {
    pub fn new() -> Self {
        Self {
            deck: Deck::new_standard(),
            community_cards: Vec::new()
        }
    }

    
    pub fn deal_cards(&mut self, num_cards: usize) -> Vec<Card> {
        let mut dealt_cards = Vec::new();
    
        for _ in 0..num_cards {
            if let Some(card) = self.deck.pop_one_card() {
                dealt_cards.push(card);
            }
        }
    
        dealt_cards
    }

    pub fn set_community_cards(&mut self, cards: Vec<Card>) {
        self.community_cards = cards;
    }
    
    pub fn deal_community_card(&mut self) {
        if let Some(card) = self.deck.pop_one_card() {
            self.community_cards.push(card);
        }
    }

    pub fn get_community_cards(&self) -> &Vec<Card> {
        &self.community_cards
    }

    pub fn shuffle_deck(&mut self) {
        self.deck.shuffle();
    }

}
