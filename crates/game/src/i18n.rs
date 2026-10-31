//! Interface languages. Every string the player reads lives here.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Russian,
    English,
    Uzbek,
}

impl Language {
    pub const ALL: [Language; 3] = [Language::Russian, Language::English, Language::Uzbek];

    /// The label on the language button.
    pub fn code(self) -> &'static str {
        match self {
            Language::Russian => "RU",
            Language::English => "EN",
            Language::Uzbek => "UZ",
        }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Language::Russian => &RUSSIAN,
            Language::English => &ENGLISH,
            Language::Uzbek => &UZBEK,
        }
    }
}

pub struct Strings {
    pub new_game: &'static str,
    pub resign: &'static str,
    pub draw: &'static str,
    pub level_title: &'static str,
    pub playing_white: &'static str,
    pub playing_black: &'static str,
    pub bot_level: &'static str,
    pub your_side: &'static str,
    pub language: &'static str,
    pub white: &'static str,
    pub black: &'static str,
    pub cancel: &'static str,
    pub start: &'static str,
    pub your_move: &'static str,
    pub bot_thinking: &'static str,
    pub check_suffix: &'static str,
    pub choose_piece: &'static str,
    pub you_won: &'static str,
    pub bot_won: &'static str,
    pub checkmate: &'static str,
    pub resigned: &'static str,
    pub stalemate: &'static str,
    pub insufficient_material: &'static str,
    pub fifty_moves: &'static str,
    pub repetition: &'static str,
    pub draw_agreed: &'static str,
    pub draw_declined: &'static str,
    pub brilliant: &'static str,
}

impl Strings {
    pub fn all(&self) -> [&'static str; 28] {
        [
            self.new_game,
            self.resign,
            self.draw,
            self.level_title,
            self.playing_white,
            self.playing_black,
            self.bot_level,
            self.your_side,
            self.language,
            self.white,
            self.black,
            self.cancel,
            self.start,
            self.your_move,
            self.bot_thinking,
            self.check_suffix,
            self.choose_piece,
            self.you_won,
            self.bot_won,
            self.checkmate,
            self.resigned,
            self.stalemate,
            self.insufficient_material,
            self.fifty_moves,
            self.repetition,
            self.draw_agreed,
            self.draw_declined,
            self.brilliant,
        ]
    }
}

static RUSSIAN: Strings = Strings {
    new_game: "Новая партия",
    resign: "Сдаться",
    draw: "Ничья",
    level_title: "уровень",
    playing_white: "Вы играете белыми",
    playing_black: "Вы играете чёрными",
    bot_level: "Уровень бота",
    your_side: "Ваш цвет",
    language: "Язык",
    white: "Белые",
    black: "Чёрные",
    cancel: "Отмена",
    start: "Начать",
    your_move: "Ваш ход",
    bot_thinking: "Бот думает",
    check_suffix: " - шах!",
    choose_piece: "Выберите фигуру",
    you_won: "вы выиграли",
    bot_won: "бот выиграл",
    checkmate: "Мат",
    resigned: "Сдача",
    stalemate: "Ничья - пат",
    insufficient_material: "Ничья - мало фигур",
    fifty_moves: "Ничья - правило 50 ходов",
    repetition: "Ничья - повторение",
    draw_agreed: "Ничья по соглашению",
    draw_declined: "Ничья отклонена",
    brilliant: "Блестящий ход!",
};

static ENGLISH: Strings = Strings {
    new_game: "New game",
    resign: "Resign",
    draw: "Draw",
    level_title: "level",
    playing_white: "You play White",
    playing_black: "You play Black",
    bot_level: "Bot level",
    your_side: "Your side",
    language: "Language",
    white: "White",
    black: "Black",
    cancel: "Cancel",
    start: "Start",
    your_move: "Your move",
    bot_thinking: "Bot is thinking",
    check_suffix: " - check!",
    choose_piece: "Choose a piece",
    you_won: "you won",
    bot_won: "bot won",
    checkmate: "Checkmate",
    resigned: "Resigned",
    stalemate: "Draw - stalemate",
    insufficient_material: "Draw - too few pieces",
    fifty_moves: "Draw - fifty-move rule",
    repetition: "Draw - repetition",
    draw_agreed: "Draw agreed",
    draw_declined: "Draw declined",
    brilliant: "Brilliant move!",
};

static UZBEK: Strings = Strings {
    new_game: "Yangi o‘yin",
    resign: "Taslim",
    draw: "Durang",
    level_title: "daraja",
    playing_white: "Siz oqlarda o‘ynaysiz",
    playing_black: "Siz qoralarda o‘ynaysiz",
    bot_level: "Bot darajasi",
    your_side: "Sizning rangingiz",
    language: "Til",
    white: "Oqlar",
    black: "Qoralar",
    cancel: "Bekor",
    start: "Boshlash",
    your_move: "Sizning yurishingiz",
    bot_thinking: "Bot o‘ylamoqda",
    check_suffix: " - shoh!",
    choose_piece: "Figurani tanlang",
    you_won: "siz yutdingiz",
    bot_won: "bot yutdi",
    checkmate: "Mot",
    resigned: "Taslim",
    stalemate: "Durang - pat",
    insufficient_material: "Durang - figuralar kam",
    fifty_moves: "Durang - 50 yurish qoidasi",
    repetition: "Durang - takrorlanish",
    draw_agreed: "Kelishilgan durang",
    draw_declined: "Durang rad etildi",
    brilliant: "Ajoyib yurish!",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{FONT_METRICS, FontMetrics};

    #[test]
    fn the_font_has_every_character_in_every_language() {
        let metrics = FontMetrics::parse(FONT_METRICS);
        for language in Language::ALL {
            for text in language.strings().all() {
                for char in text.chars() {
                    assert!(metrics.glyphs.contains_key(&char), "{char} in {text}");
                }
            }
        }
    }

    #[test]
    fn button_labels_fit_their_buttons() {
        let metrics = FontMetrics::parse(FONT_METRICS);
        for language in Language::ALL {
            let strings = language.strings();
            for (_, label, rect) in crate::layout::panel_buttons(strings) {
                assert!(metrics.measure(label) + 2.0 <= rect.w, "{label}");
            }
            for (_, label, rect) in crate::layout::side_buttons(strings) {
                assert!(metrics.measure(label) + 2.0 <= rect.w, "{label}");
            }
        }
    }

    #[test]
    fn status_lines_fit_the_panel() {
        let metrics = FontMetrics::parse(FONT_METRICS);
        for language in Language::ALL {
            let s = language.strings();
            for text in [
                s.stalemate,
                s.insufficient_material,
                s.fifty_moves,
                s.repetition,
            ] {
                assert!(
                    metrics.measure(text) <= crate::layout::PANEL_WIDTH,
                    "{text}"
                );
            }
            let longest = format!("{}{}", s.bot_thinking, s.check_suffix);
            assert!(metrics.measure(&longest) <= crate::layout::PANEL_WIDTH);
        }
    }
}
