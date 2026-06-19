use std::process::Output;

pub(crate) fn output_text(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    text
}

fn build_frequency_attempts(preferred: u32) -> Vec<u32> {
    let mut values = Vec::new();
    for value in [
        preferred,
        8000_000,
        240_000,
        180_000,
        120_000,
        100_000,
        80_000,
        40_000,
        20_000,
        10_000,
    ] {
        if value > 0 && !values.contains(&value) {
            values.push(value);
        }
    }
    values
}

pub(crate) fn build_backend_frequency_attempts(backend: &str, preferred: u32) -> Vec<u32> {
    if backend.eq_ignore_ascii_case("pyocd") {
        let mut values = Vec::new();
        for value in [
            100_000,
            80_000,
            120_000,
            180_000,
            240_000,
            500_000,
            1_000_000,
            preferred,
        ] {
            if value > 0 && !values.contains(&value) {
                values.push(value);
            }
        }
        return values;
    }
    build_frequency_attempts(preferred)
}
