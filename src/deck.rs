use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::{
    card::Zone,
    events::{CardEvent, Event},
    game::{dispatch_event, Game, GameStatus, ObjectId},
};

pub fn draw_card(game: &mut Game, player_id: ObjectId) -> Option<ObjectId> {
    let player = if let Some(player) = game.get_player(player_id) {
        player
    } else {
        return None;
    };

    let card_id = if let Some(card_id) = player.deck.pop() {
        player.hand.push(card_id);
        card_id
    } else {
        game.status = GameStatus::Lose(player_id);
        return None;
    };

    if let Some(card) = game.get_card(card_id) {
        card.zone = Zone::Hand;
    } else {
        panic!("Card {card_id} does not exist.");
    };

    dispatch_event(
        game,
        Event::Draw(CardEvent {
            owner: player_id,
            card: card_id,
            source: None,
        }),
    );

    return Some(card_id);
}

pub fn put_on_deck_top(game: &mut Game, card_id: ObjectId, player_id: ObjectId) {
    if let Some(player) = game.get_player(player_id) {
        player.deck.push(card_id);
    } else {
        return;
    };

    if let Some(card) = game.get_card(card_id) {
        card.zone = Zone::Library;
    } else {
        panic!("Card {card_id} does not exist.");
    };
}

pub fn put_on_deck_bottom(game: &mut Game, card_id: ObjectId, player_id: ObjectId) {
    if let Some(player) = game.get_player(player_id) {
        player.deck.insert(0, card_id);
    } else {
        return;
    };

    if let Some(card) = game.get_card(card_id) {
        card.zone = Zone::Library;
    } else {
        panic!("Card {card_id} does not exist.");
    };
}

pub fn shuffle_deck(game: &mut Game, player_id: ObjectId) {
    if let Some(player) = game.get_player(player_id) {
        player.deck.shuffle(&mut thread_rng());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        card::Card,
        deck::{draw_card, put_on_deck_bottom, put_on_deck_top},
        game::{Game, GameStatus, Player},
    };

    #[test]
    fn test_put_on_deck_top() {
        let mut game = Game::new();
        let player = Player::new();
        let player_id = game.add_player(player);

        let mut card = Card::default();
        card.name = String::from("Forest");
        card.owner_id = player_id;
        let forest_id = game.add_card(card);

        let mut card = Card::default();
        card.name = String::from("Mountain");
        card.owner_id = player_id;
        let mountain_id = game.add_card(card);

        put_on_deck_top(&mut game, forest_id, player_id);
        put_on_deck_top(&mut game, mountain_id, player_id);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.deck[0], forest_id);
        assert_eq!(player.deck[1], mountain_id);
    }

    #[test]
    fn test_put_on_deck_bottom() {
        let mut game = Game::new();
        let player = Player::new();
        let player_id = game.add_player(player);

        let mut card = Card::default();
        card.name = String::from("Forest");
        card.owner_id = player_id;
        let forest_id = game.add_card(card);

        let mut card = Card::default();
        card.name = String::from("Mountain");
        card.owner_id = player_id;
        let mountain_id = game.add_card(card);

        put_on_deck_bottom(&mut game, forest_id, player_id);
        put_on_deck_bottom(&mut game, mountain_id, player_id);

        let player = game.get_player(player_id).unwrap();
        assert_eq!(player.deck[0], mountain_id);
        assert_eq!(player.deck[1], forest_id);
    }

    #[test]
    fn test_draw_card() {
        let mut game = Game::new();
        let player = Player::new();
        let player_id = game.add_player(player);

        let mut card = Card::default();
        card.owner_id = player_id;
        let card_id = game.add_card(card);

        put_on_deck_top(&mut game, card_id, player_id);
        let drawn_card = draw_card(&mut game, player_id);

        assert_eq!(drawn_card, Some(card_id));
    }

    #[test]
    fn test_draw_card_lose_game() {
        let mut game = Game::new();
        let player = Player::new();
        let player_id = game.add_player(player);

        let result = draw_card(&mut game, player_id);
        assert_eq!(result, None);
        assert_eq!(game.status, GameStatus::Lose(player_id));
    }
}
