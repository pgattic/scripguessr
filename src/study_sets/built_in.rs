use std::sync::{Arc, OnceLock};

use serde::Deserialize;

use super::{PromptPolicy, StudyGuessScope, StudyPassage, StudySet};
use crate::scriptures::{BookScope, Canon, CanonScope, GameScope};

pub fn built_in_study_sets() -> &'static [Arc<StudySet>] {
    static SETS: OnceLock<Vec<Arc<StudySet>>> = OnceLock::new();
    SETS.get_or_init(|| {
        build_built_in_study_sets()
            .into_iter()
            .map(Arc::new)
            .collect()
    })
}

fn build_built_in_study_sets() -> Vec<StudySet> {
    let old_testament = old_testament_mastery();
    let new_testament = new_testament_mastery();
    let book_of_mormon = book_of_mormon_mastery();
    let doctrine_and_covenants = doctrine_and_covenants_mastery();
    let all = [
        &old_testament,
        &new_testament,
        &book_of_mormon,
        &doctrine_and_covenants,
    ]
    .into_iter()
    .flat_map(|set| set.passages.clone())
    .collect();

    let mut sets = vec![
        StudySet {
            id: "doctrinal-mastery-all".to_string(),
            name: "All Doctrinal Mastery".to_string(),
            passages: all,
            guess_scope: StudyGuessScope::AllStandardWorks,
            prompt_policy: PromptPolicy::WholePassage,
        },
        old_testament,
        new_testament,
        book_of_mormon,
        doctrine_and_covenants,
    ];
    sets.extend(preach_my_gospel_study_sets());
    sets
}

fn preach_my_gospel_study_sets() -> Vec<StudySet> {
    #[derive(Deserialize)]
    struct Data {
        sets: Vec<DataSet>,
    }

    #[derive(Deserialize)]
    struct DataSet {
        id: String,
        name: String,
        passages: Vec<StudyPassage>,
    }

    let data: Data = serde_json::from_str(include_str!(
        "../../assets/data/preach-my-gospel-study-sets.json"
    ))
    .expect("Preach My Gospel study-set data must be valid");
    let mut all_passages = Vec::new();
    let mut chapter_sets = data
        .sets
        .into_iter()
        .map(|set| {
            for passage in &set.passages {
                if !all_passages.contains(passage) {
                    all_passages.push(passage.clone());
                }
            }
            StudySet {
                id: set.id,
                name: set.name,
                passages: set.passages,
                guess_scope: StudyGuessScope::FullCanons,
                prompt_policy: PromptPolicy::Automatic,
            }
        })
        .collect::<Vec<_>>();
    chapter_sets.insert(
        0,
        StudySet {
            id: "preach-my-gospel-scripture-study".to_string(),
            name: "Preach My Gospel Scripture Study".to_string(),
            passages: all_passages,
            guess_scope: StudyGuessScope::AllStandardWorks,
            prompt_policy: PromptPolicy::Automatic,
        },
    );
    chapter_sets
}

fn passage(canon: Canon, book: &str, chapter: u16, start: u16, end: u16) -> StudyPassage {
    StudyPassage {
        canon,
        book: book.to_string(),
        chapter,
        verses: (start..=end).collect(),
    }
}

fn selected_passage(canon: Canon, book: &str, chapter: u16, verses: &[u16]) -> StudyPassage {
    StudyPassage {
        canon,
        book: book.to_string(),
        chapter,
        verses: verses.to_vec(),
    }
}

fn old_testament_mastery() -> StudySet {
    use Canon::{OldTestament as OT, PearlOfGreatPrice as PGP};
    StudySet {
        id: "doctrinal-mastery-old-testament".to_string(),
        name: "Old Testament Doctrinal Mastery".to_string(),
        passages: vec![
            passage(PGP, "Moses", 1, 39, 39),
            passage(PGP, "Moses", 7, 18, 18),
            passage(PGP, "Abraham", 2, 9, 11),
            passage(PGP, "Abraham", 3, 22, 23),
            passage(OT, "Genesis", 1, 26, 27),
            passage(OT, "Genesis", 2, 24, 24),
            passage(OT, "Genesis", 39, 9, 9),
            passage(OT, "Exodus", 20, 3, 17),
            passage(OT, "Joshua", 24, 15, 15),
            passage(OT, "Psalms", 24, 3, 4),
            passage(OT, "Proverbs", 3, 5, 6),
            passage(OT, "Isaiah", 1, 18, 18),
            passage(OT, "Isaiah", 5, 20, 20),
            passage(OT, "Isaiah", 29, 13, 14),
            passage(OT, "Isaiah", 53, 3, 5),
            passage(OT, "Isaiah", 58, 6, 7),
            passage(OT, "Isaiah", 58, 13, 14),
            passage(OT, "Jeremiah", 1, 4, 5),
            passage(OT, "Ezekiel", 3, 16, 17),
            passage(OT, "Ezekiel", 37, 15, 17),
            passage(OT, "Daniel", 2, 44, 45),
            passage(OT, "Amos", 3, 7, 7),
            passage(OT, "Malachi", 3, 8, 10),
            passage(OT, "Malachi", 4, 5, 6),
        ],
        guess_scope: StudyGuessScope::Custom(GameScope {
            canons: vec![
                CanonScope {
                    canon: OT,
                    books: BookScope::All,
                },
                CanonScope {
                    canon: PGP,
                    books: BookScope::Selected(vec!["Moses".to_string(), "Abraham".to_string()]),
                },
            ],
        }),
        prompt_policy: PromptPolicy::WholePassage,
    }
}

fn new_testament_mastery() -> StudySet {
    use Canon::NewTestament as NT;
    StudySet {
        id: "doctrinal-mastery-new-testament".to_string(),
        name: "New Testament Doctrinal Mastery".to_string(),
        passages: vec![
            passage(NT, "Matthew", 5, 14, 16),
            passage(NT, "Matthew", 11, 28, 30),
            passage(NT, "Matthew", 16, 15, 19),
            passage(NT, "Matthew", 22, 36, 39),
            passage(NT, "Luke", 2, 10, 12),
            passage(NT, "Luke", 22, 19, 20),
            passage(NT, "Luke", 24, 36, 39),
            passage(NT, "John", 3, 5, 5),
            passage(NT, "John", 3, 16, 16),
            passage(NT, "John", 7, 17, 17),
            passage(NT, "John", 17, 3, 3),
            passage(NT, "1 Corinthians", 6, 19, 20),
            passage(NT, "1 Corinthians", 11, 11, 11),
            passage(NT, "1 Corinthians", 15, 20, 22),
            passage(NT, "1 Corinthians", 15, 40, 42),
            passage(NT, "Ephesians", 1, 10, 10),
            passage(NT, "Ephesians", 2, 19, 20),
            passage(NT, "2 Thessalonians", 2, 1, 3),
            passage(NT, "2 Timothy", 3, 15, 17),
            passage(NT, "Hebrews", 12, 9, 9),
            passage(NT, "James", 1, 5, 6),
            passage(NT, "James", 2, 17, 18),
            passage(NT, "1 Peter", 4, 6, 6),
            passage(NT, "Revelation", 20, 12, 12),
        ],
        guess_scope: StudyGuessScope::FullCanons,
        prompt_policy: PromptPolicy::WholePassage,
    }
}

fn book_of_mormon_mastery() -> StudySet {
    use Canon::BookOfMormon as BOM;
    StudySet {
        id: "doctrinal-mastery-book-of-mormon".to_string(),
        name: "Book of Mormon Doctrinal Mastery".to_string(),
        passages: vec![
            passage(BOM, "1 Nephi", 3, 7, 7),
            passage(BOM, "2 Nephi", 2, 25, 25),
            passage(BOM, "2 Nephi", 2, 27, 27),
            passage(BOM, "2 Nephi", 26, 33, 33),
            passage(BOM, "2 Nephi", 28, 30, 30),
            passage(BOM, "2 Nephi", 32, 3, 3),
            passage(BOM, "2 Nephi", 32, 8, 9),
            passage(BOM, "Mosiah", 2, 17, 17),
            passage(BOM, "Mosiah", 2, 41, 41),
            passage(BOM, "Mosiah", 3, 19, 19),
            passage(BOM, "Mosiah", 4, 9, 9),
            passage(BOM, "Mosiah", 18, 8, 10),
            passage(BOM, "Alma", 7, 11, 13),
            passage(BOM, "Alma", 34, 9, 10),
            passage(BOM, "Alma", 39, 9, 9),
            passage(BOM, "Alma", 41, 10, 10),
            passage(BOM, "Helaman", 5, 12, 12),
            passage(BOM, "3 Nephi", 11, 10, 11),
            passage(BOM, "3 Nephi", 12, 48, 48),
            passage(BOM, "3 Nephi", 27, 20, 20),
            passage(BOM, "Ether", 12, 6, 6),
            passage(BOM, "Ether", 12, 27, 27),
            passage(BOM, "Moroni", 7, 45, 48),
            passage(BOM, "Moroni", 10, 4, 5),
        ],
        guess_scope: StudyGuessScope::FullCanons,
        prompt_policy: PromptPolicy::WholePassage,
    }
}

fn doctrine_and_covenants_mastery() -> StudySet {
    use Canon::{DoctrineAndCovenants as DC, PearlOfGreatPrice as PGP};
    StudySet {
        id: "doctrinal-mastery-doctrine-and-covenants".to_string(),
        name: "Doctrine and Covenants Doctrinal Mastery".to_string(),
        passages: vec![
            passage(PGP, "Joseph Smith—History", 1, 15, 20),
            passage(DC, "D&C", 1, 30, 30),
            passage(DC, "D&C", 1, 37, 38),
            passage(DC, "D&C", 6, 36, 36),
            passage(DC, "D&C", 8, 2, 3),
            passage(DC, "D&C", 13, 1, 1),
            passage(DC, "D&C", 18, 10, 11),
            passage(DC, "D&C", 18, 15, 16),
            passage(DC, "D&C", 19, 16, 19),
            passage(DC, "D&C", 21, 4, 6),
            passage(DC, "D&C", 29, 10, 11),
            passage(DC, "D&C", 49, 15, 17),
            passage(DC, "D&C", 58, 42, 43),
            passage(DC, "D&C", 64, 9, 11),
            passage(DC, "D&C", 76, 22, 24),
            passage(DC, "D&C", 82, 10, 10),
            passage(DC, "D&C", 84, 20, 22),
            passage(DC, "D&C", 88, 118, 118),
            passage(DC, "D&C", 89, 18, 21),
            passage(DC, "D&C", 107, 8, 8),
            selected_passage(DC, "D&C", 121, &[36, 41, 42]),
            passage(DC, "D&C", 130, 22, 23),
            passage(DC, "D&C", 131, 1, 4),
            passage(DC, "D&C", 135, 3, 3),
        ],
        guess_scope: StudyGuessScope::Custom(GameScope {
            canons: vec![
                CanonScope {
                    canon: DC,
                    books: BookScope::All,
                },
                CanonScope {
                    canon: PGP,
                    books: BookScope::Selected(vec!["Joseph Smith—History".to_string()]),
                },
            ],
        }),
        prompt_policy: PromptPolicy::WholePassage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_of_mormon_mastery_uses_the_full_canon_for_guesses() {
        let set = built_in_study_sets()
            .iter()
            .find(|set| set.id == "doctrinal-mastery-book-of-mormon")
            .unwrap();
        let scope = set.resolved_guess_scope();

        assert!(scope.includes_book(Canon::BookOfMormon, "Jacob"));
        assert!(!scope.includes_book(Canon::NewTestament, "Matthew"));
    }

    #[test]
    fn preach_my_gospel_sets_use_automatic_prompts() {
        let sets = built_in_study_sets();
        let combined = sets
            .iter()
            .find(|set| set.id == "preach-my-gospel-scripture-study")
            .unwrap();

        assert_eq!(combined.passages.len(), 602);
        assert_eq!(combined.prompt_policy, PromptPolicy::Automatic);
        assert!(
            sets.iter()
                .filter(|set| set.id.starts_with("preach-my-gospel-chapter-"))
                .all(|set| !set.passages.is_empty())
        );
    }
}
