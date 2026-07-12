//! Probe: time the Mandelbrot LET / REDUCE / HSTACK formula end-to-end
//! through `NativeOxfmlHostSession` to see where the seconds go. Not a CI
//! test — gated behind `--ignored` so a normal `cargo test` doesn't
//! pay the multi-second cost. Run with:
//!
//! ```
//! cargo test -p dnaonecalc-host --test mandelbrot_perf_probe \
//!   -- --ignored --nocapture
//! ```

use dnaonecalc_host::adapters::oxfml::{NativeOxfmlHostSession, OxfmlHostSession};
use dnaonecalc_host::app::case_lifecycle::new_formula_space;
use dnaonecalc_host::services::live_edit::apply_live_editor_input;
use dnaonecalc_host::state::OneCalcHostState;
use dnaonecalc_host::ui::editor::commands::{EditorInputEvent, EditorInputKind};

fn fresh_state() -> OneCalcHostState {
    let mut state = OneCalcHostState::default();
    let _ = new_formula_space(&mut state);
    state
}

fn type_formula(bridge: &dyn OxfmlHostSession, state: &mut OneCalcHostState, text: &str) {
    let caret = text.chars().count();
    apply_live_editor_input(
        bridge,
        state,
        EditorInputEvent {
            text: text.to_string(),
            selection_start: Some(caret),
            selection_end: Some(caret),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some(text.to_string()),
        },
    )
    .expect("bridge runs");
}

const MANDEL: &str = r#"=LET(
  rows, 100,
  cols, 60,
  maxIter, 30,
  cx, -0.5,
  cy, 0,
  zoom, 1.2,
  width, 3 / zoom,
  height, 2.4 / zoom,
  palette, " .:-=+*#%@",
  rowSeq, SEQUENCE(rows, 1, 0, 1),
  colSeq, SEQUENCE(1, cols, 0, 1),
  x0, cx - width/2 + (colSeq / (cols - 1)) * width,
  y0, cy - height/2 + (rowSeq / (rows - 1)) * height,
  mandel, LAMBDA(a, b,
    REDUCE(
      HSTACK(0, 0, 0),
      SEQUENCE(maxIter),
      LAMBDA(state, k,
        LET(
          x, INDEX(state, 1, 1),
          y, INDEX(state, 1, 2),
          n, INDEX(state, 1, 3),
          escaped, (x*x + y*y) > 4,
          IF(escaped,
             state,
             HSTACK(x*x - y*y + a, 2*x*y + b, n + 1)
          )
        )
      )
    )
  ),
  iters, MAKEARRAY(rows, cols, LAMBDA(r, c,
    INDEX(mandel(INDEX(x0, 1, c), INDEX(y0, r, 1)), 1, 3)
  )),
  charIdx, IF(iters = maxIter, 1, 1 + INT(iters / maxIter * (LEN(palette) - 1))),
  MID(palette, charIdx, 1)
)"#;

#[test]
#[ignore = "performance probe — multi-second; run with --ignored"]
fn time_mandelbrot_full_size() {
    let bridge = NativeOxfmlHostSession::default();
    let mut state = fresh_state();
    let start = std::time::Instant::now();
    type_formula(&bridge, &mut state, MANDEL);
    let elapsed = start.elapsed();
    println!("Mandelbrot 100x60x30 end-to-end: {elapsed:?}");
}

#[test]
#[ignore = "performance probe — measures shrink dimensions"]
fn time_mandelbrot_shrunk() {
    // Same shape, smaller dims — show how cost scales.
    let bridge = NativeOxfmlHostSession::default();
    for (rows, cols, max_iter) in [
        (5usize, 5, 5),
        (10, 10, 10),
        (10, 10, 30),
        (20, 20, 30),
        (40, 40, 30),
        (50, 30, 30),
        (100, 60, 30),
    ] {
        let mut state = fresh_state();
        let formula = MANDEL
            .replacen("rows, 100,", &format!("rows, {rows},"), 1)
            .replacen("cols, 60,", &format!("cols, {cols},"), 1)
            .replacen("maxIter, 30,", &format!("maxIter, {max_iter},"), 1);
        let start = std::time::Instant::now();
        type_formula(&bridge, &mut state, &formula);
        let elapsed = start.elapsed();
        let cells = rows * cols;
        let inner_iters = cells * max_iter;
        let per_iter = elapsed.as_micros() as f64 / inner_iters as f64;
        println!(
            "rows={rows} cols={cols} maxIter={max_iter} cells={cells} inner_iters={inner_iters} elapsed={elapsed:?} per_inner_iter={per_iter:.2}us",
        );
    }
}
