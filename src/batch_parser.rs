pub struct BatchParser {
    pub batches: Vec<String>,
}

impl BatchParser {
    pub fn parse(sql: &str) -> Self {
        let mut batches: Vec<String> = vec![];
        let mut current_batch = String::new();
        let mut last_char = '\n'; // Initialize with a non-relevant character
        let mut string_skipping = false;
        let mut comment_skipping = false;
        let mut go_detected = false;

        for (i, ch) in sql.chars().enumerate() {
            if ch == '\'' && !comment_skipping {
                string_skipping = !string_skipping;
            }

            if !string_skipping {
                if last_char == '/' && ch == '*' {
                    comment_skipping = true;
                }

                if last_char == '*' && ch == '/' {
                    comment_skipping = false;
                }
            }

            if !string_skipping && !comment_skipping {
                if last_char.is_whitespace() && ch == 'G' {
                    go_detected = true; // Potential start of "GO"
                } else if go_detected
                    && ch == 'O'
                    && sql.chars().nth(i + 1).map_or(true, |c| c.is_whitespace())
                {
                    // Confirmed "GO" with whitespaces around, split batch
                    batches.push(current_batch.clone().trim_end_matches('G').to_owned());
                    current_batch.clear();
                    go_detected = false;
                    continue;
                } else {
                    go_detected = false; // Reset detection flag if conditions not met
                }
            }

            current_batch.push(ch);
            last_char = ch;
        }

        // Add the remaining batch if any
        if !current_batch.is_empty() {
            batches.push(current_batch);
        }

        Self { batches }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn positive() {
        let parser = BatchParser::parse(
            "SELECT * FROM Some
			GO
			SELECT * FROM Some",
        );

        assert_eq!(2, parser.batches.len());
        assert!(!parser.batches[0].ends_with("G"));
        let first_line = "SELECT * FROM Some
			";
        assert_eq!(first_line, parser.batches[0]);
        let second_line = "
			SELECT * FROM Some";
        assert_eq!(second_line, parser.batches[1]);
    }

    #[test]
    fn negative_string() {
        let content = "SELECT '
		GO
		' FROM Translation";
        let parser = BatchParser::parse(content);

        assert_eq!(1, parser.batches.len());
        assert_eq!(content, parser.batches[0]);
    }

    #[test]
    fn negative_comment() {
        let content = "SELECT * /*
		GO
		*/ FROM Translation";
        let parser = BatchParser::parse(content);

        assert_eq!(1, parser.batches.len());
        assert_eq!(content, parser.batches[0]);
    }

    #[test]
    fn negative_text() {
        let content = "SELECT * FROM GOals";
        let parser = BatchParser::parse(content);

        assert_eq!(1, parser.batches.len());
        assert_eq!(content, parser.batches[0]);
    }

    #[test]
    fn complex_string() {
        let parser = BatchParser::parse(
            "SELECT '
		GO
		' FROM Translation
		GO
		SELECT '
		GO
		' FROM Translation",
        );

        assert_eq!(2, parser.batches.len());
    }

    #[test]
    fn complexer_string() {
        let content = std::fs::read_to_string("./.tests/parsing/test1.sql").unwrap();

        let parser = BatchParser::parse(&content);

        assert_eq!(2, parser.batches.len());
    }

    #[test]
    fn complexerer_string() {
        let content = std::fs::read_to_string("./.tests/parsing/test2.sql").unwrap();

        let parser = BatchParser::parse(&content);

        assert_eq!(1, parser.batches.len());
    }
}
