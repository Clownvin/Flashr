/*
 * Copyright (C) 2024 Clownvin <123clownvin@gmail.com>
 *
 * This file is part of Flashr.
 *
 * Flashr is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Flashr is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Flashr.  If not, see <http://www.gnu.org/licenses/>.
 */

use rand::prelude::{SliceRandom, ThreadRng};

use crate::{
    random::{GetRandom, IntoIterShuffled},
    stats::Stats,
    weighted_list::WeightedList,
    AndThen, DeckCard, FlashrError, OptionTuple, PromptCard,
};

use super::{MatchProblem, ANSWERS_PER_PROBLEM};

pub(super) struct MatchProblemIterator<'a> {
    rng: &'a mut ThreadRng,
    weighted_deck_cards: WeightedList<DeckCard<'a>>,
    faces: Option<Vec<String>>,
    line: bool,
}

impl<'a> MatchProblemIterator<'a> {
    pub fn new(
        deck_cards: Vec<DeckCard<'a>>,
        stats: &mut Stats,
        faces: Option<Vec<String>>,
        line: bool,
        rng: &'a mut ThreadRng,
    ) -> Self {
        Self {
            rng,
            faces,
            line,
            weighted_deck_cards: {
                let mut buf = WeightedList::with_capacity(deck_cards.len());
                deck_cards.into_iter().for_each(|deck_card| {
                    let weight = stats.for_card(&deck_card).weight();
                    buf.add((deck_card, weight));
                });
                buf
            },
        }
    }

    pub fn change_weight(&mut self, index: usize, weight: f64) {
        self.weighted_deck_cards.change_weight(index, weight)
    }
}

impl<'a> Iterator for MatchProblemIterator<'a> {
    type Item = Result<MatchProblem<'a>, FlashrError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (problem_deck_card, problem_index) = self.weighted_deck_cards.get_random(self.rng)?;

        let possible_faces = problem_deck_card.possible_faces();

        let (
            (_question_index, question_face, problem_question_face),
            (_answer_index, answer_face, problem_answer_face),
        ) = match self.faces.as_ref() {
            Some(faces) => {
                let question = possible_faces
                    .clone()
                    .into_iter_shuffled(self.rng)
                    .find(|(_, face, _)| faces.iter().any(|specified| face == &specified))
                    .expect("Unable to find a valid question face");

                let (question_index, _, _) = question;

                //TODO: Abilitiy to specify answer faces as well?
                let answer = possible_faces
                    .into_iter_shuffled(self.rng)
                    .find(|(i, _, _)| *i != question_index)
                    .expect("Unable to find a valid answer face");

                (question, answer)
            }
            None => possible_faces
                .into_iter_shuffled(self.rng)
                .collect::<OptionTuple<_>>()
                .expect("Unable to find valid question and answer faces"),
        };

        let mut seen_faces = Vec::with_capacity(ANSWERS_PER_PROBLEM);
        seen_faces.push(problem_answer_face);

        let mut answer_cards = Vec::with_capacity(ANSWERS_PER_PROBLEM);
        answer_cards.push((
            (problem_answer_face, *problem_deck_card, problem_index),
            true,
        ));

        self.weighted_deck_cards
            .clone()
            .into_iter_shuffled(self.rng)
            .filter_map(|((deck_card, _), card_index)| {
                let card_answer_face =
                    deck_card
                        .deck
                        .faces
                        .iter()
                        .enumerate()
                        .find_map(|(i, face)| {
                            (face == answer_face).and_then(|| deck_card.card[i].as_ref())
                        })?;

                if seen_faces.contains(&card_answer_face) {
                    return None;
                } else {
                    seen_faces.push(card_answer_face);
                }

                let card_question_face_matches_problem = {
                    let card_question_face =
                        deck_card
                            .deck
                            .faces
                            .iter()
                            .enumerate()
                            .find_map(|(i, face)| {
                                (face == question_face).and_then(|| deck_card[i].as_ref())
                            });

                    card_question_face
                        .map(|card_question_face| card_question_face == problem_question_face)
                        .unwrap_or(false)
                };

                if card_question_face_matches_problem {
                    return None;
                }

                Some(((card_answer_face, deck_card, card_index), false))
            })
            .take(ANSWERS_PER_PROBLEM - 1)
            .for_each(|answer_card| answer_cards.push(answer_card));

        if answer_cards.len() < ANSWERS_PER_PROBLEM {
            let deck_name = &problem_deck_card.deck.name;
            return Some(Err(FlashrError::DeckMismatch(format!("Cannot find enough answers for question {problem_question_face}, which is a \"{question_face}\" face, from deck {deck_name}, given answer face \"{answer_face}\""))));
        }

        answer_cards.shuffle(self.rng);

        let answer_index = answer_cards
            .iter()
            .enumerate()
            .find_map(|(i, (_, correct))| correct.then_some(i))
            .expect("Unable to find answer index after shuffling");

        Some(Ok(MatchProblem {
            question: PromptCard {
                prompt: problem_question_face.join_random(self.rng),
                deck_card: *problem_deck_card,
                index: problem_index,
            },
            answers: {
                let mut buf = Vec::with_capacity(ANSWERS_PER_PROBLEM);
                for ((answer_face, answer_deck_card, answer_index), correct) in answer_cards {
                    buf.push((
                        PromptCard {
                            prompt: answer_face.join_random(self.rng),
                            deck_card: answer_deck_card,
                            index: answer_index,
                        },
                        correct,
                    ))
                }
                buf
            },
            answer_index,
            weights: self.line.then(|| self.weighted_deck_cards.weights()),
        }))
    }
}

#[cfg(test)]
mod test {
    use crate::{deck::load_decks, stats::Stats, ModeArguments};

    use super::MatchProblemIterator;

    #[test]
    fn ensure_unique_question_answers() {
        let decks = load_decks(vec!["./tests/deck1.json"]).expect("Unable to load test deck");
        let args = ModeArguments::new(&decks, None, None, false);
        let rng = &mut rand::thread_rng();
        let stats = &mut Stats::new("");
        let problems =
            MatchProblemIterator::new(args.deck_cards, stats, args.faces, args.line, rng);

        for problem in problems.take(1000) {
            let problem = problem.expect("Unable to get problem");
            assert!(problem
                .answers
                .iter()
                //Assert that each problem question is not present in the answers
                .all(|(answer, _)| answer.prompt != problem.question.prompt));
            assert!(problem
                .answers
                .iter()
                .enumerate()
                .all(|(ref i, (answer, correct))| {
                    //Ensure no answers are the same
                    problem
                        .answers
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| i != j)
                        .all(|(_, (other_answer, _))| other_answer.prompt != answer.prompt)

                    //And also ensure that no answer's "question face" is the same as
                    //the problem's
                    && (*correct || answer.deck_card.last() != problem.question.deck_card.last())
                }));
        }
    }

    #[test]
    fn fails_if_not_enough_unique_answers() {
        let decks = load_decks(vec!["./tests/duplicate_cards"])
            .expect("Unable to load duplicate cards test deck");
        let args = ModeArguments::new(&decks, None, None, false);
        let rng = &mut rand::thread_rng();
        let stats = &mut Stats::new("");
        let mut problems =
            MatchProblemIterator::new(args.deck_cards, stats, args.faces, args.line, rng);

        assert!(problems
            .next()
            .is_some_and(|problem| problem
                .is_err_and(|err| matches!(err, crate::FlashrError::DeckMismatch(_)))));
    }
}
