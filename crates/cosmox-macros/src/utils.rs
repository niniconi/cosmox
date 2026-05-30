#[allow(dead_code)]
pub fn count_format_args_in_string(format_string: &str) -> Result<usize, String> {
    let mut max_implicit_idx: Option<usize> = None;
    let mut explicit_indices: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();

    let mut chars = format_string.chars().peekable();
    let mut in_placeholder = false;
    let mut current_idx_str = String::new();

    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
            } else {
                in_placeholder = true;
                current_idx_str.clear();
            }
        } else if c == '}' {
            if chars.peek() == Some(&'}') {
                chars.next();
            } else {
                if !in_placeholder {
                    return Err("Mismatched closing brace `}`".to_string());
                }

                if current_idx_str.is_empty() {
                    if max_implicit_idx.is_none() {
                        max_implicit_idx = Some(0);
                    } else {
                        max_implicit_idx = max_implicit_idx.map(|x| x + 1);
                    }
                } else {
                    let parts: Vec<&str> = current_idx_str.split(':').collect();
                    let index_part = parts[0];

                    if let Ok(idx) = index_part.parse::<usize>() {
                        explicit_indices.insert(idx);
                    } else {
                        return Err(format!(
                            "Unsupported or invalid placeholder: {{{current_idx_str}}}",
                        ));
                    }
                }
                in_placeholder = false;
            }
        } else if in_placeholder {
            current_idx_str.push(c);
        }
    }

    if in_placeholder {
        return Err("Unmatched opening brace `{`".to_string());
    }

    let mut total_args = 0;
    if let Some(max_idx) = max_implicit_idx {
        total_args = max_idx + 1;
    }

    for &idx in &explicit_indices {
        if idx >= total_args {
            total_args = idx + 1;
        }
    }

    Ok(total_args)
}
