use std::time::Duration;

pub(super) fn percentage(count: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }

    count as f64 * 100.0 / total as f64
}

pub(super) fn completion_label(completed: bool) -> &'static str {
    if completed { "예" } else { "아니오" }
}

pub(super) fn per_sec(count: usize, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs == 0.0 {
        return 0.0;
    }

    count as f64 / secs
}

pub(super) fn push_row(output: &mut String, label: &str, value: &str) {
    const LABEL_WIDTH: usize = 19;

    let padding = LABEL_WIDTH.saturating_sub(display_width(label)).max(1);
    output.push_str("  ");
    output.push_str(label);
    output.push_str(&" ".repeat(padding));
    output.push_str(value);
    output.push('\n');
}

fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|ch| if ch.is_ascii() { 1 } else { 2 })
        .sum()
}

pub(super) fn format_count(value: usize) -> String {
    let raw = value.to_string();
    let mut formatted = String::with_capacity(raw.len() + raw.len() / 3);

    for (index, ch) in raw.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }

    formatted.chars().rev().collect()
}

pub(super) fn format_duration_display(duration: Duration) -> String {
    let secs = duration.as_secs_f64();

    if secs >= 1.0 {
        format!("{secs:.3}s")
    } else {
        format!("{:.3}ms", secs * 1_000.0)
    }
}

pub(super) fn format_micros(us: u64) -> String {
    if us >= 1_000_000 {
        format!("{:.3}s", us as f64 / 1_000_000.0)
    } else if us >= 1_000 {
        format!("{:.3}ms", us as f64 / 1_000.0)
    } else {
        format!("{us}µs")
    }
}

pub(super) fn format_rate(value: f64) -> String {
    let raw = format!("{value:.2}");
    let Some((whole, fraction)) = raw.split_once('.') else {
        return raw;
    };

    let whole = whole
        .parse::<usize>()
        .map_or_else(|_| whole.to_string(), format_count);

    format!("{whole}.{fraction}")
}
