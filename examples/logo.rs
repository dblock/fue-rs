//! Generates `assets/logo.svg`, a unicorn whose eye is an `@`, a nod to fue's original name, Finding Unicorn Engineers.
//!
//! Run `cargo run --example logo` after changing it; `cargo test` checks that the committed logo matches.

use std::fs;
use std::path::Path;

/// Background.
const NIGHT: &str = "#24104a";
/// The unicorn's head.
const WHITE: &str = "#f8f4ff";

/// The mane gradient, pink to violet to cyan.
const MANE: [(&str, &str); 3] = [("0", "#ff6ec7"), ("0.5", "#b388ff"), ("1", "#7afcff")];
/// The horn gradient, gold to pink.
const HORN: [(&str, &str); 2] = [("0", "#ffd36e"), ("1", "#ff6ec7")];

/// The head in profile, facing right.
const HEAD: &str = "M150 470 C118 380 136 250 228 162 L220 92 L268 146 C302 150 332 176 354 212 L412 300 C432 332 422 370 386 374 L350 368 C326 364 306 350 290 340 C280 384 290 432 312 470 Z";
/// Strands of mane behind the head and down the neck.
const MANE_STRANDS: [&str; 2] = [
    "M228 166 C170 200 140 260 150 330 C120 300 110 250 130 210 C100 230 90 270 96 300 C70 250 90 180 150 150 C130 150 115 160 100 172 C130 120 190 110 232 132 Z",
    "M150 330 C140 390 150 430 170 470 L150 470 C120 420 118 370 150 330 Z",
];
const HORN_SHAPE: &str = "M286 152 L322 168 L398 44 Z";
const HORN_SPIRAL: &str = "M300 150 L326 150 M316 124 L340 124 M332 98 L354 99";
/// The eye, an `@` drawn as strokes so it does not depend on fonts.
const EYE: &str = "M329 222 V244 C329 256 347 256 347 238 A31 31 0 1 0 333 265";
/// Stars: x, y, radius, and color.
const STARS: [(u32, u32, u32, &str); 3] = [
    (440, 150, 7, "#7afcff"),
    (80, 96, 6, "#ff6ec7"),
    (446, 420, 5, "#b388ff"),
];

fn gradient(id: &str, stops: &[(&str, &str)]) -> String {
    let stops: String = stops
        .iter()
        .map(|(offset, color)| format!(r#"<stop offset="{offset}" stop-color="{color}"/>"#))
        .collect();
    format!(r#"<linearGradient id="{id}" x1="0" y1="1" x2="1" y2="0">{stops}</linearGradient>"#)
}

/// The logo as SVG.
fn svg() -> String {
    let mut svg = vec![
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" role="img" aria-label="fue">"#.to_string(),
        format!("<defs>{}{}</defs>", gradient("mane", &MANE), gradient("horn", &HORN)),
        format!(r#"<rect width="512" height="512" rx="96" fill="{NIGHT}"/>"#),
    ];
    svg.extend(MANE_STRANDS.iter().enumerate().map(|(index, strand)| {
        let opacity = if index == 0 { "" } else { r#" opacity="0.85""# };
        format!(r#"<path d="{strand}" fill="url(#mane)"{opacity}/>"#)
    }));
    svg.push(format!(r#"<path d="{HEAD}" fill="{WHITE}"/>"#));
    svg.push(format!(r#"<path d="{HORN_SHAPE}" fill="url(#horn)"/>"#));
    svg.push(format!(
        r#"<path d="{HORN_SPIRAL}" stroke="{NIGHT}" stroke-width="6" stroke-linecap="round"/>"#
    ));
    svg.push(format!(
        r#"<g fill="none" stroke="{NIGHT}" stroke-width="9" stroke-linecap="round"><circle cx="316" cy="238" r="13"/><path d="{EYE}"/></g>"#
    ));
    svg.extend(STARS.iter().map(|(x, y, radius, color)| {
        format!(r#"<circle cx="{x}" cy="{y}" r="{radius}" fill="{color}"/>"#)
    }));
    svg.push("</svg>".to_string());
    svg.join("\n") + "\n"
}

fn path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/logo.svg")
}

fn main() -> std::io::Result<()> {
    fs::write(path(), svg())?;
    println!("Wrote {}.", path().display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_logo_is_up_to_date() {
        let committed = fs::read_to_string(path()).expect("assets/logo.svg exists");
        assert!(
            committed == svg(),
            "assets/logo.svg is out of date. Run `cargo run --example logo`."
        );
    }
}
