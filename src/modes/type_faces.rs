use crate::{
    deck::Face,
    random::{GetRandom, IntoIterShuffled},
    stats::Stats,
    weighted_list::WeightedList,
    DeckCard, OptionTuple, Progress,
};
use rand::rngs::ThreadRng;

use crate::{deck::Deck, terminal::TerminalWrapper, FlashrError, ModeArguments};

pub fn type_faces(mut term: TerminalWrapper, args: ModeArguments) -> Result<Progress, FlashrError> {
    let term = &mut term;
    let rng = &mut rand::thread_rng();
    let stats = &mut Stats::load_from_user_home()?;
    let mut problems = TypeProblemIterator::new(args.deck_cards, stats, args.faces, rng);

    let mut total_correct = 0;

    todo!()
}

struct TypeProblemIterator<'a> {
    rng: &'a mut ThreadRng,
    cards: WeightedList<DeckCard<'a>>,
    faces: Option<Vec<String>>,
}

impl<'a> TypeProblemIterator<'a> {
    fn new(
        deck_cards: Vec<DeckCard<'a>>,
        stats: &mut Stats,
        faces: Option<Vec<String>>,
        rng: &'a mut ThreadRng,
    ) -> Self {
        let cards = deck_cards
            .into_iter()
            .map(|deck_card| (deck_card, stats.for_card(&deck_card).weight()))
            .collect();
        Self { rng, cards, faces }
    }
}

impl<'a> Iterator for TypeProblemIterator<'a> {
    type Item = TypeProblem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (deck_card, index) = self.cards.get_random(self.rng)?;
        let (question, answer) = deck_card
            .deck
            .faces
            .iter()
            .enumerate()
            .filter_map(|(i, face)| {
                if self
                    .faces
                    .as_ref()
                    .is_some_and(|faces| faces.iter().any(|specified| face != specified))
                {
                    return None;
                }
                deck_card.card[i]
                    .as_ref()
                    .map(|card_face| (face, card_face))
            })
            .collect::<Vec<_>>()
            .into_iter_shuffled(self.rng)
            .collect::<OptionTuple<_>>()
            .unwrap();

        Some(TypeProblem {
            deck: deck_card.deck,
            question,
            answer,
            index,
        })
    }
}

struct TypeProblem<'a> {
    deck: &'a Deck,
    question: (&'a String, &'a Face),
    answer: (&'a String, &'a Face),
    index: usize,
}

fn show_type_problem(
    term: &TerminalWrapper,
    problem: &TypeProblem,
    progress: (usize, usize),
) -> Result<Progress, FlashrError> {
    todo!()
}
