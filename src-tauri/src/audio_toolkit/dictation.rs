//! Rule-based dictation cleanup: spoken punctuation and sentence capitalization.
//!
//! Standard library only, so the tests can run with a bare `rustc --test`.

enum Command {
    Punctuation(char),
    Break(&'static str),
}

const REPLACEABLE_PUNCTUATION: &[char] = &[',', '.', ';', ':', '!', '?'];
const OPENING_QUOTES: &[char] = &['"', '\'', '(', '[', '“', '‘'];
const CLOSING_QUOTES: &[char] = &['"', '\'', ')', ']', '”', '’'];

/// A spoken command right after one of these is the ordinary word ("the comma").
const DETERMINERS: &[&str] = &[
    "a", "an", "the", "this", "that", "these", "those", "my", "your", "our", "their", "his", "her",
    "its", "each", "every", "any", "another", "per", "one", "first", "second", "third", "last",
    "next", "same", "previous", "current",
];
const PERIOD_NOUNS: &[&str] = &[
    "billing", "warranty", "grace", "trial", "notice", "pay", "payment", "rental", "waiting",
    "cooling", "review", "comment", "retention", "performance", "contract", "reporting", "bid",
    "time", "day", "week", "month", "year", "lien", "budget", "accounting", "construction",
    "closeout", "maintenance", "lookback", "probation", "probationary", "holding", "transition",
    "study", "test",
];
const PERIOD_FOLLOWERS: &[&str] = &[
    "of", "ended", "ends", "expires", "expired", "starts", "started", "begins", "began", "runs",
    "is", "was",
];
const LINE_VERBS: &[&str] = &[
    "add", "added", "adding", "install", "installed", "installing", "run", "ran", "running",
    "pull", "pulled", "need", "needs", "needed", "lay", "laid", "want", "wants", "have", "has",
];
const LINE_FOLLOWERS: &[&str] = &[
    "of", "item", "items", "to", "for", "from", "at", "in", "on", "with", "is", "was", "cost",
];
const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "st", "sr", "jr", "vs", "etc", "inc", "ltd", "co", "corp", "approx",
    "dept", "est", "ave", "blvd",
];
const PRONOUN_I: &[&str] = &["i", "i'm", "i'll", "i've", "i'd", "i’m", "i’ll", "i’ve", "i’d"];

/// Converts spoken punctuation to symbols and capitalizes sentence starts.
///
/// Words are only ever capitalized, never lowercased, so names and acronyms the
/// model got right stay as they are.
pub fn format_dictation(text: &str, spoken_punctuation: bool, capitalization: bool) -> String {
    if !spoken_punctuation && !capitalization {
        return text.to_string();
    }

    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out = String::new();
    let mut at_sentence_start = true;
    let mut i = 0;

    while i < tokens.len() {
        let command = if spoken_punctuation {
            parse_command(&tokens, i)
        } else {
            None
        };
        if let Some((command, used)) = command {
            match command {
                Command::Break(separator) => {
                    out.push_str(separator);
                    at_sentence_start = true;
                    i += used;
                    continue;
                }
                Command::Punctuation(symbol) => {
                    let has_word_before = !out.is_empty() && !out.ends_with('\n');
                    if has_word_before || used == tokens.len() {
                        out.truncate(out.trim_end_matches(REPLACEABLE_PUNCTUATION).len());
                        out.push(symbol);
                        at_sentence_start = matches!(symbol, '.' | '?' | '!');
                        i += used;
                        continue;
                    }
                }
            }
        }

        if !out.is_empty() && !out.ends_with('\n') {
            out.push(' ');
        }
        if capitalization {
            let word = fix_pronoun_i(tokens[i]);
            if at_sentence_start {
                out.push_str(&capitalize_word(&word));
            } else {
                out.push_str(&word);
            }
        } else {
            out.push_str(tokens[i]);
        }
        at_sentence_start = ends_sentence(tokens[i]);
        i += 1;
    }

    out
}

/// Returns the command starting at `tokens[i]` and how many tokens it spans.
fn parse_command(tokens: &[&str], i: usize) -> Option<(Command, usize)> {
    let first = bare_word(tokens[i]);
    let second = tokens
        .get(i + 1)
        .filter(|_| ends_in_word(tokens[i]))
        .map(|token| bare_word(token))
        .unwrap_or_default();

    let (command, used) = match (first.as_str(), second.as_str()) {
        ("question", "mark") => (Command::Punctuation('?'), 2),
        ("exclamation", "point" | "mark") => (Command::Punctuation('!'), 2),
        ("full", "stop") => (Command::Punctuation('.'), 2),
        ("new", "line") => (Command::Break("\n"), 2),
        ("new", "paragraph") => (Command::Break("\n\n"), 2),
        ("comma", _) => (Command::Punctuation(','), 1),
        ("period", _) => (Command::Punctuation('.'), 1),
        ("colon", _) => (Command::Punctuation(':'), 1),
        ("semicolon", _) => (Command::Punctuation(';'), 1),
        _ => return None,
    };

    if is_literal_use(&first, tokens, i, used) {
        None
    } else {
        Some((command, used))
    }
}

/// Decides from the neighboring words whether a command word is just a word.
/// Punctuation next to the command means the model heard a clause boundary, so
/// a neighbor across that boundary is ignored.
fn is_literal_use(first: &str, tokens: &[&str], i: usize, used: usize) -> bool {
    let prev = i
        .checked_sub(1)
        .map(|p| tokens[p])
        .filter(|token| ends_in_word(token))
        .map(bare_word)
        .unwrap_or_default();
    let next = tokens
        .get(i + used)
        .filter(|_| ends_in_word(tokens[i + used - 1]))
        .map(|token| bare_word(token))
        .unwrap_or_default();
    let (prev, next) = (prev.as_str(), next.as_str());

    if DETERMINERS.contains(&prev) {
        return true;
    }
    match first {
        "period" => {
            PERIOD_NOUNS.contains(&prev) || prev.contains('-') || PERIOD_FOLLOWERS.contains(&next)
        }
        "comma" => matches!(prev, "oxford" | "serial") || matches!(next, "separated" | "delimited"),
        "new" => LINE_VERBS.contains(&prev) || LINE_FOLLOWERS.contains(&next),
        _ => false,
    }
}

fn bare_word(token: &str) -> String {
    token
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn ends_in_word(token: &str) -> bool {
    token.ends_with(char::is_alphanumeric)
}

fn ends_sentence(token: &str) -> bool {
    let word = token.trim_end_matches(CLOSING_QUOTES);
    if word.ends_with('?') || word.ends_with('!') {
        return true;
    }
    if !word.ends_with('.') || word.ends_with("...") {
        return false;
    }
    // "U.S." and "a.m." have inner dots; they rarely end a sentence.
    let stem = word.trim_end_matches('.');
    !stem.contains('.') && !ABBREVIATIONS.contains(&bare_word(stem).as_str())
}

fn fix_pronoun_i(token: &str) -> String {
    if PRONOUN_I.contains(&bare_word(token).as_str()) {
        token.replacen('i', "I", 1)
    } else {
        token.to_string()
    }
}

/// Capitalizes the first letter unless the word has its own casing or is an
/// address ("iPhone", "example.com", "me@example.com").
fn capitalize_word(token: &str) -> String {
    let start = token.len() - token.trim_start_matches(OPENING_QUOTES).len();
    let (opening, rest) = token.split_at(start);

    let stem = rest.trim_end_matches(|c: char| !c.is_alphanumeric());
    let is_address = stem.contains('@') || stem.contains('/') || stem.contains('.');
    if is_address || rest.chars().any(char::is_uppercase) {
        return token.to_string();
    }

    let mut chars = rest.chars();
    match chars.next() {
        Some(first) if first.is_alphabetic() => {
            format!("{}{}{}", opening, first.to_uppercase(), chars.as_str())
        }
        _ => token.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::format_dictation;

    #[track_caller]
    fn check(input: &str, expected: &str) {
        assert_eq!(format_dictation(input, true, true), expected);
    }

    #[test]
    fn spoken_punctuation_can_run_alone() {
        assert_eq!(
            format_dictation("hello comma i said period then stop", true, false),
            "hello, i said. then stop"
        );
    }

    #[test]
    fn capitalization_can_run_alone() {
        assert_eq!(
            format_dictation("hello comma i said. then stop", false, true),
            "Hello comma I said. Then stop"
        );
    }

    #[test]
    fn both_off_leaves_text_unchanged() {
        assert_eq!(
            format_dictation("hello comma  i said period", false, false),
            "hello comma  i said period"
        );
    }

    #[test]
    fn converts_spoken_punctuation() {
        check(
            "hello comma world period how are you question mark",
            "Hello, world. How are you?",
        );
        check("wow exclamation point that rocks", "Wow! That rocks");
        check("wow exclamation mark", "Wow!");
        check("note colon buy milk", "Note: buy milk");
        check("first part semicolon second part", "First part; second part");
        check("all done full stop", "All done.");
    }

    #[test]
    fn replaces_punctuation_the_model_added() {
        check("Done, period.", "Done.");
        check("Done. Period. Next item", "Done. Next item");
        check("What time is it, question mark?", "What time is it?");
        check("Hello, comma, world", "Hello, world");
    }

    #[test]
    fn converts_line_breaks() {
        check("first line new line second line", "First line\nSecond line");
        check("Hello. New line. How are you", "Hello.\nHow are you");
        check("one thing new paragraph another", "One thing\n\nAnother");
        check("new line", "\n");
    }

    #[test]
    fn keeps_ordinary_uses_of_period() {
        check("The billing period ended.", "The billing period ended.");
        check("the warranty period", "The warranty period");
        check("a 30-day period", "A 30-day period");
        check("the period of performance", "The period of performance");
        check("it was a long period of time", "It was a long period of time");
        check("that period was rough", "That period was rough");
    }

    #[test]
    fn keeps_ordinary_uses_of_other_commands() {
        check("the comma is wrong", "The comma is wrong");
        check("use the oxford comma", "Use the oxford comma");
        check("comma separated values", "Comma separated values");
        check("he put a question mark there", "He put a question mark there");
        check("we need a new line of credit", "We need a new line of credit");
        check("run new line to the panel", "Run new line to the panel");
        check("add a new paragraph", "Add a new paragraph");
    }

    #[test]
    fn lone_command_becomes_its_symbol() {
        check("period", ".");
        check("Question mark.", "?");
    }

    #[test]
    fn capitalizes_sentence_starts() {
        check("it costs 5 dollars. then stop", "It costs 5 dollars. Then stop");
        check("really? yes! okay", "Really? Yes! Okay");
        check("\"hello there\" he said", "\"Hello there\" he said");
    }

    #[test]
    fn does_not_capitalize_after_abbreviations() {
        check("Dr. smith is here", "Dr. smith is here");
        check("the U.S. army", "The U.S. army");
        check("meet at 5 p.m. tomorrow", "Meet at 5 p.m. tomorrow");
        check("wait... then go", "Wait... then go");
    }

    #[test]
    fn leaves_special_words_alone() {
        check("iPhone is great", "iPhone is great");
        check("example.com is down", "example.com is down");
        check("me@example.com wrote", "me@example.com wrote");
        check("5 dollars", "5 dollars");
        check("NASA called", "NASA called");
    }

    #[test]
    fn never_lowercases() {
        check("Chris and the RVP Team", "Chris and the RVP Team");
    }

    #[test]
    fn capitalizes_pronoun_i() {
        check("i think i'm done and i'll go", "I think I'm done and I'll go");
        check("so i said no", "So I said no");
        check("i.e. this one", "i.e. this one");
    }

    #[test]
    fn handles_empty_input() {
        check("", "");
        check("   ", "");
    }
}
