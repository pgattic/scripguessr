use crate::api::{AccountDataResponse, UserResponse};
use crate::stats::{ReviewItem, Stats};
use crate::study_sets::{StudyPassage, StudySet};

/// The signed-in player and everything stored for them on the server.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Account {
    pub user: Option<UserResponse>,
    /// Whether the session check has finished, so the UI can tell guests from loading.
    pub loaded: bool,
    pub stats: Stats,
    pub review_items: Vec<ReviewItem>,
    pub study_sets: Vec<StudySet>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReviewToggle {
    Added(ReviewItem),
    Removed(StudyPassage),
}

impl Account {
    pub fn apply_user(&mut self, user: Option<UserResponse>) {
        if user.is_none() {
            *self = Self::default();
        }
        self.user = user;
        self.loaded = true;
    }

    pub fn apply_data(&mut self, data: AccountDataResponse) {
        self.stats = data.stats;
        self.review_items = data.review_items;
        self.study_sets = data.custom_study_sets;
    }

    pub fn is_marked(&self, passage: &StudyPassage) -> bool {
        self.review_items
            .iter()
            .any(|item| item.passage == *passage)
    }

    pub fn toggle_review(&mut self, item: ReviewItem) -> ReviewToggle {
        if self.remove_review(&item.passage) {
            ReviewToggle::Removed(item.passage)
        } else {
            self.review_items.push(item.clone());
            ReviewToggle::Added(item)
        }
    }

    pub fn remove_review(&mut self, passage: &StudyPassage) -> bool {
        let original_len = self.review_items.len();
        self.review_items.retain(|item| item.passage != *passage);
        self.review_items.len() != original_len
    }

    pub fn edit_study_set(
        &mut self,
        id: &str,
        edit: impl FnOnce(&mut StudySet),
    ) -> Option<StudySet> {
        let set = self.study_sets.iter_mut().find(|set| set.id == id)?;
        edit(set);
        Some(set.clone())
    }

    pub fn upsert_study_set(&mut self, set: StudySet) {
        match self
            .study_sets
            .iter_mut()
            .find(|existing| existing.id == set.id)
        {
            Some(existing) => *existing = set,
            None => self.study_sets.push(set),
        }
    }

    pub fn delete_study_set(&mut self, id: &str) {
        self.study_sets.retain(|set| set.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scriptures::Canon;

    fn item(book: &str) -> ReviewItem {
        ReviewItem {
            passage: StudyPassage {
                canon: Canon::BookOfMormon,
                book: book.to_string(),
                chapter: 32,
                verses: vec![21],
            },
            text: "And now as I said concerning faith".to_string(),
            score: 612,
        }
    }

    #[test]
    fn review_items_toggle_by_passage() {
        let mut account = Account::default();
        let item = item("Alma");

        assert_eq!(
            account.toggle_review(item.clone()),
            ReviewToggle::Added(item.clone())
        );
        assert!(account.is_marked(&item.passage));

        assert_eq!(
            account.toggle_review(item.clone()),
            ReviewToggle::Removed(item.passage.clone())
        );
        assert!(!account.is_marked(&item.passage));
        assert!(account.review_items.is_empty());
    }

    #[test]
    fn signing_out_clears_account_data() {
        let mut account = Account::default();
        account.review_items.push(item("Alma"));

        account.apply_user(None);

        assert!(account.loaded);
        assert!(account.review_items.is_empty());
    }
}
