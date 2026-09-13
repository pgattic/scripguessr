pub const MAX_SCORE: u32 = 1000;
const POINTS_LOST_PER_CHAPTER: u32 = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Score {
    pub points: u32,
    pub chapter_distance: u32,
}

impl Score {
    pub fn from_chapter_distance(chapter_distance: u32) -> Self {
        let points = if chapter_distance == 0 {
            MAX_SCORE
        } else {
            MAX_SCORE.saturating_sub(chapter_distance.saturating_mul(POINTS_LOST_PER_CHAPTER))
        };

        Self {
            points,
            chapter_distance,
        }
    }

    pub fn distance_label(self) -> String {
        match self.chapter_distance {
            0 => "Exact chapter".to_string(),
            1 => "Off by 1 chapter".to_string(),
            distance => format!("Off by {distance} chapters"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_guess_gets_max_score() {
        assert_eq!(
            Score::from_chapter_distance(0),
            Score {
                points: 1000,
                chapter_distance: 0,
            }
        );
    }

    #[test]
    fn nearby_guess_decays_by_chapter_distance() {
        assert_eq!(Score::from_chapter_distance(1).points, 978);
        assert_eq!(Score::from_chapter_distance(3).points, 934);
    }

    #[test]
    fn far_guess_bottoms_out_at_zero() {
        assert_eq!(Score::from_chapter_distance(200).points, 0);
    }

    #[test]
    fn distance_label_uses_singular_and_plural_text() {
        assert_eq!(
            Score::from_chapter_distance(0).distance_label(),
            "Exact chapter"
        );
        assert_eq!(
            Score::from_chapter_distance(1).distance_label(),
            "Off by 1 chapter"
        );
        assert_eq!(
            Score::from_chapter_distance(2).distance_label(),
            "Off by 2 chapters"
        );
    }
}
