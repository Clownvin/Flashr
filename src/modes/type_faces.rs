use crate::{
    random::{GetRandom, IntoIterShuffled, WeightedList},
    stats::Stats,
    CorrectIncorrect, FaceCardIndex, OptionTuple,
};
use rand::rngs::ThreadRng;

use crate::{
    deck::{Card, Deck, Face},
    terminal::TerminalWrapper,
    FlashrError, ModeArguments, ModeResult, ProblemResult,
};

pub fn type_faces(
    mut term: TerminalWrapper,
    args: ModeArguments,
) -> Result<ModeResult, FlashrError> {
    let term = &mut term;
    let rng = &mut rand::thread_rng();
    let mut stats = args.stats;
    let mut problems = TypeProblemIterator::new(args.deck_cards, &mut stats, args.faces, rng);

    let mut total_correct = 0;

    // if let Some(count) = args.problem_count {
    //     for _ in 0..count {
    //         let problem = problems.next().unwrap();
    //         let result = show_type_problem(term, &problem, (total_correct, count))?;
    //
    //         if result.is_quit() {
    //             return Ok(((total_correct, count), stats));
    //         } else {
    //             let stats = stats.for_card_mut();
    //
    //             if result.is_correct() {
    //                 stats.correct += 1;
    //                 total_correct += 1;
    //             } else {
    //                 stats.incorrect += 1;
    //             }
    //
    //             problems.change_weight(problem.question.2, stats.weight());
    //         }
    //     }
    //
    //     Ok(((total_correct, count), stats))
    // } else {
    //     let mut total = 0;
    //
    //     for (i, problem) in problems.enumerate() {
    //         let result = show_type_problem(term, problem, (total_correct, i))?;
    //
    //         total += 1;
    //         match result {
    //             ProblemResult::Correct => total_correct += 1,
    //             ProblemResult::Quit => return Ok(((total_correct, total), stats)),
    //             ProblemResult::Incorrect => {}
    //         }
    //     }
    //
    //     Ok(((total_correct, total), stats))
    // }
    todo!()
}

struct TypeProblemIterator<'a> {
    rng: &'a mut ThreadRng,
    cards: WeightedList<(&'a Deck, &'a Card)>,
    faces: Option<Vec<String>>,
}

impl<'a> TypeProblemIterator<'a> {
    fn new(
        deck_cards: Vec<(&'a Deck, &'a Card)>,
        stats: &mut Stats,
        faces: Option<Vec<String>>,
        rng: &'a mut ThreadRng,
    ) -> Self {
        let cards = deck_cards
            .into_iter()
            .map(|deck_card| (deck_card, stats.for_card(deck_card).weight()))
            .collect();
        Self { rng, cards, faces }
    }
}

impl<'a> Iterator for TypeProblemIterator<'a> {
    type Item = TypeProblem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let ((deck, card), index) = self.cards.get_random(self.rng)?;
        let (question, answer) = deck
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
                card[i].as_ref().map(|card_face| (face, card_face))
            })
            .collect::<Vec<_>>()
            .into_iter_shuffled(self.rng)
            .collect::<OptionTuple<_>>()
            .unwrap();

        // Some(TypeProblem {
        //     deck,
        //     question: (question, answer, index),
        // })
        todo!()
    }
}

struct TypeProblem<'a> {
    deck: &'a Deck,
    question: FaceCardIndex<'a>,
}

fn show_type_problem(
    term: &TerminalWrapper,
    problem: &TypeProblem,
    progress: CorrectIncorrect,
) -> Result<ProblemResult, FlashrError> {
    todo!()
}
