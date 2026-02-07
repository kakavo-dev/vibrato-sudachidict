pub fn merge_scientific_notation_tokens<S: AsRef<str>>(tokens: &[S]) -> Vec<String> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut i = 0;

    while i < tokens.len() {
        if i + 2 < tokens.len() {
            let lhs = tokens[i].as_ref();
            let sign = tokens[i + 1].as_ref();
            let rhs = tokens[i + 2].as_ref();

            if is_scientific_lhs(lhs) && (sign == "+" || sign == "-") && is_ascii_digits(rhs) {
                merged.push(format!("{lhs}{sign}{rhs}"));
                i += 3;
                continue;
            }
        }

        merged.push(tokens[i].as_ref().to_string());
        i += 1;
    }

    merged
}

fn is_scientific_lhs(token: &str) -> bool {
    if token.len() < 2 {
        return false;
    }

    let mut chars = token.chars();
    let last = chars.next_back();
    matches!(last, Some('e' | 'E')) && is_ascii_digits(chars.as_str())
}

fn is_ascii_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_only_scientific_notation_pattern() {
        let input = ["1e", "-", "3", "x", "2E", "+", "10"];
        let actual = merge_scientific_notation_tokens(&input);
        assert_eq!(actual, vec!["1e-3", "x", "2E+10"]);
    }

    #[test]
    fn does_not_merge_non_scientific_patterns() {
        let input = ["1", "-", "3", "k8s", "abc"];
        let actual = merge_scientific_notation_tokens(&input);
        assert_eq!(actual, vec!["1", "-", "3", "k8s", "abc"]);
    }

    #[test]
    fn ignores_invalid_left_or_right_side() {
        let input = ["e", "-", "3", "1e", "-", "x"];
        let actual = merge_scientific_notation_tokens(&input);
        assert_eq!(actual, vec!["e", "-", "3", "1e", "-", "x"]);
    }
}
