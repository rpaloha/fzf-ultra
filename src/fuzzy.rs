#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Match {
    pub index: usize,
    pub score: i64,
}

pub fn score(query: &str, candidate: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let query: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    let chars: Vec<char> = candidate.chars().collect();
    let mut query_index = 0;
    let mut total = 0_i64;
    let mut previous = None;

    for (index, original) in chars.iter().enumerate() {
        if query_index == query.len() {
            break;
        }
        let current = original.to_lowercase().next().unwrap_or(*original);
        if current != query[query_index] {
            continue;
        }

        let boundary = index == 0 || !chars[index - 1].is_alphanumeric();
        total += 10 + if boundary { 9 } else { 0 };
        if let Some(previous_index) = previous {
            if index == previous_index + 1 {
                total += 14;
            } else {
                total -= (index - previous_index - 1) as i64;
            }
        } else {
            total -= (index as i64).min(20);
        }
        previous = Some(index);
        query_index += 1;
    }

    (query_index == query.len()).then_some(total)
}

pub fn matches(query: &str, candidates: &[String], no_sort: bool) -> Vec<Match> {
    let mut found: Vec<Match> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            score(query, candidate).map(|score| Match { index, score })
        })
        .collect();
    if !no_sort {
        found.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.index.cmp(&b.index)));
    }
    found
}
