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
    deck::Deck,
    random::{GetRandom, IntoIterShuffled},
    stats::Stats,
    weighted_list::WeightedList,
    AndThen, DeckCard, FlashrError, PromptCard,
};

use super::{MatchProblem, ANSWERS_PER_PROBLEM};

pub(super) struct MatchProblemIterator<'a> {
    rng: &'a mut ThreadRng,
    weighted_deck_cards: WeightedList<DeckCard<'a>>,
    question_faces: Vec<String>,
    answer_faces: Vec<String>,
    line: bool,
}

impl<'a> MatchProblemIterator<'a> {
    pub fn new(
        decks: &'a [Deck],
        stats: &mut Stats,
        question_faces: Option<Vec<String>>,
        answer_faces: Option<Vec<String>>,
        line: bool,
        rng: &'a mut ThreadRng,
    ) -> Result<Self, FlashrError> {
        let (deck_cards, question_faces, answer_faces) =
            deck_cards_with_faces(decks, question_faces, answer_faces)?;

        Ok(Self {
            rng,
            question_faces,
            answer_faces,
            line,
            weighted_deck_cards: {
                let mut buf = WeightedList::with_capacity(deck_cards.len());
                deck_cards.into_iter().for_each(|deck_card| {
                    let weight = stats.for_card(&deck_card).weight();
                    buf.add((deck_card, weight));
                });
                buf
            },
        })
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

        let ((_, question_face, problem_question_face), (_, answer_face, problem_answer_face)) = {
            let question = possible_faces
                .clone()
                .into_iter_shuffled(self.rng)
                .find(|(_, face, _)| {
                    self.question_faces
                        .iter()
                        .any(|question_face| *face == question_face)
                })
                .expect("Unable to find a valid question face");

            let (question_index, _, _) = question;

            let answer = possible_faces
                .into_iter_shuffled(self.rng)
                .find(|(i, face, _)| {
                    *i != question_index
                        && self
                            .answer_faces
                            .iter()
                            .any(|answer_face| *face == answer_face)
                })
                .expect("Unable to find a valid answer face");

            (question, answer)
        };

        let mut seen_faces = Vec::with_capacity(ANSWERS_PER_PROBLEM);
        seen_faces.push(problem_answer_face);

        let mut answer_cards = Vec::with_capacity(ANSWERS_PER_PROBLEM);
        answer_cards.push((
            (problem_answer_face, *problem_deck_card, problem_index),
            //SAFETY: Because we push the "correct" here (cont. below)
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

        //SAFETY: This is safe because we inserted the "correct" above,
        //so it should be impossible not to find it after shuffle
        let answer_index = unsafe {
            answer_cards
                .iter()
                .enumerate()
                .find_map(|(i, (_, correct))| correct.then_some(i))
                .unwrap_unchecked()
        };

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

type DeckCardsWithFaces<'a> = (Vec<DeckCard<'a>>, Vec<String>, Vec<String>);

fn deck_cards_with_faces(
    decks: &[Deck],
    question_faces: Option<Vec<String>>,
    answer_faces: Option<Vec<String>>,
) -> Result<DeckCardsWithFaces, FlashrError> {
    fn all_deck_faces(decks: &[Deck]) -> Vec<String> {
        let mut faces = Vec::new();
        decks.iter().for_each(|deck| {
            deck.faces.iter().for_each(|face| {
                if !faces.contains(face) {
                    faces.push(face.to_owned())
                }
            })
        });
        faces
    }

    let (mut question_faces, mut answer_faces) = match (question_faces, answer_faces) {
        (None, None) => {
            let faces = all_deck_faces(decks);
            (faces.clone(), faces)
        }
        (Some(question_faces), None) => {
            let answer_faces = all_deck_faces(decks);
            (question_faces, answer_faces)
        }
        (None, Some(answer_faces)) => {
            let question_faces = all_deck_faces(decks);
            (question_faces, answer_faces)
        }
        (Some(question_faces), Some(answer_faces)) => (question_faces, answer_faces),
    };

    let mut combined_faces = if question_faces.len() < answer_faces.len() {
        let mut combined_faces = answer_faces.clone();
        question_faces.iter().for_each(|face| {
            if !combined_faces.contains(face) {
                combined_faces.push(face.to_owned());
            }
        });
        combined_faces
    } else {
        let mut combined_faces = question_faces.clone();
        answer_faces.iter().for_each(|face| {
            if !combined_faces.contains(face) {
                combined_faces.push(face.to_owned())
            }
        });
        combined_faces
    };

    // TODO: There's probably a better way
    let total_cards = loop {
        let (face_totals, total_cards) =
            total_faces(decks, &combined_faces, &question_faces, &answer_faces);

        if total_cards < ANSWERS_PER_PROBLEM {
            return Err(FlashrError::DeckMismatch(
                "Not enough cards after filtering".to_owned(),
            ));
        }

        let mut filtered_faces = false;

        combined_faces = combined_faces
            .into_iter()
            .enumerate()
            .filter_map(|(i, face)| {
                if face_totals[i] < ANSWERS_PER_PROBLEM {
                    eprintln!("Not enough cards with face \"{face}\"");
                    filtered_faces = true;
                    None
                } else {
                    Some(face)
                }
            })
            .collect();

        if filtered_faces {
            question_faces.retain(|face| combined_faces.contains(face));

            if question_faces.is_empty() {
                return Err(FlashrError::DeckMismatch(
                    "Unable to find enough question faces".to_owned(),
                ));
            }

            answer_faces.retain(|face| combined_faces.contains(face));

            if answer_faces.is_empty() {
                return Err(FlashrError::DeckMismatch(
                    "Unable to find enough answer faces".to_owned(),
                ));
            }
        } else {
            break total_cards;
        }
    };

    Ok((
        find_deck_cards(
            decks,
            total_cards,
            &combined_faces,
            &question_faces,
            &answer_faces,
        ),
        question_faces,
        answer_faces,
    ))
}

fn total_faces(
    decks: &[Deck],
    combined_faces: &[String],
    question_faces: &[String],
    answer_faces: &[String],
) -> (Vec<usize>, usize) {
    let mut face_totals: Vec<_> = (0..combined_faces.len()).map(|_| 0).collect();
    let mut total_cards = 0;

    decks.iter().for_each(|deck| {
        let mut has_question = false;
        let mut has_answer = false;

        let deck_faces = {
            let mut buf = Vec::with_capacity(deck.faces.len());
            deck.faces
                .iter()
                .enumerate()
                .filter_map(|(deck_i, deck_face)| {
                    combined_faces
                        .iter()
                        .enumerate()
                        .find_map(|(faces_i, face)| {
                            if face == deck_face {
                                let question_face = question_faces.contains(face);
                                let answer_face = answer_faces.contains(face);

                                has_question = has_question || question_face;
                                has_answer = has_answer || answer_face;

                                Some((deck_i, faces_i, question_face, answer_face))
                            } else {
                                None
                            }
                        })
                })
                .for_each(|pair| buf.push(pair));
            buf
        };

        if deck_faces.len() > 1 && has_question && has_answer {
            total_faces_deck(deck, deck_faces, &mut face_totals, &mut total_cards);
        }
    });

    (face_totals, total_cards)
}

fn total_faces_deck(
    deck: &Deck,
    deck_faces: Vec<(usize, usize, bool, bool)>,
    face_totals: &mut [usize],
    total_cards: &mut usize,
) {
    deck.cards.iter().for_each(|card| {
        let mut has_question = false;
        let mut has_answer = false;
        let mut total_card_faces = 0;

        deck_faces
            .iter()
            .for_each(|(deck_i, _, question_face, answer_face)| {
                if card[*deck_i].is_some() {
                    total_card_faces += 1;
                    has_question = has_question || *question_face;
                    has_answer = has_answer || *answer_face;
                }
            });

        if total_card_faces > 1 && has_question && has_answer {
            deck_faces.iter().for_each(|(deck_i, face_i, _, _)| {
                if card[*deck_i].is_some() {
                    face_totals[*face_i] += 1;
                }
            });
            *total_cards += 1;
        }
    });
}

fn find_deck_cards<'a>(
    decks: &'a [Deck],
    total_cards: usize,
    combined_faces: &[String],
    question_faces: &[String],
    answer_faces: &[String],
) -> Vec<DeckCard<'a>> {
    let mut deck_cards = Vec::with_capacity(total_cards);

    decks.iter().for_each(|deck| {
        let mut has_question = false;
        let mut has_answer = false;

        let deck_faces = {
            let mut buf = Vec::with_capacity(deck.faces.len());
            deck.faces
                .iter()
                .enumerate()
                .filter_map(|(deck_i, deck_face)| {
                    combined_faces
                        .iter()
                        .enumerate()
                        .find_map(|(faces_i, face)| {
                            if face == deck_face {
                                let question_face = question_faces.contains(face);
                                let answer_face = answer_faces.contains(face);

                                has_question = has_question || question_face;
                                has_answer = has_answer || answer_face;

                                Some((deck_i, faces_i, question_face, answer_face))
                            } else {
                                None
                            }
                        })
                })
                .for_each(|pair| buf.push(pair));
            buf
        };

        if deck_faces.len() > 1 && has_question && has_answer {
            find_deck_cards_in_deck(deck, deck_faces, &mut deck_cards);
        }
    });

    deck_cards
}

fn find_deck_cards_in_deck<'a>(
    deck: &'a Deck,
    deck_faces: Vec<(usize, usize, bool, bool)>,
    deck_cards: &mut Vec<DeckCard<'a>>,
) {
    deck.cards.iter().for_each(|card| {
        let mut has_question = false;
        let mut has_answer = false;
        let mut total_card_faces = 0;

        deck_faces
            .iter()
            .for_each(|(deck_i, _, question_face, answer_face)| {
                if card[*deck_i].is_some() {
                    total_card_faces += 1;
                    has_question = has_question || *question_face;
                    has_answer = has_answer || *answer_face;
                }
            });

        if total_card_faces > 1 && has_question && has_answer {
            deck_cards.push(DeckCard::new(deck, card));
        }
    });
}

#[cfg(test)]
mod test {
    use crate::{deck::load_decks, stats::Stats};

    use super::MatchProblemIterator;

    #[test]
    fn ensure_unique_question_answers() {
        let decks = load_decks(vec!["./tests/deck1.json"]).expect("Unable to load test deck");
        let stats = &mut Stats::new("");
        let rng = &mut rand::thread_rng();
        let problems = MatchProblemIterator::new(&decks, stats, None, None, false, rng).unwrap();

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
        let stats = &mut Stats::new("");
        let rng = &mut rand::thread_rng();
        let mut problems =
            MatchProblemIterator::new(&decks, stats, None, None, false, rng).unwrap();

        assert!(problems
            .next()
            .is_some_and(|problem| problem
                .is_err_and(|err| matches!(err, crate::FlashrError::DeckMismatch(_)))));
    }
}
