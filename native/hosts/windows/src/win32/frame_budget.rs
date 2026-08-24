//! Release-only frame-budget guard for Startup Lab animation.

use super::{Canvas, REVEAL_INTERVAL_MILLIS, StartupLab, startup_lab, tests::sample_lab};

/// Frames composed in one batch.
///
/// The window covers the end of the mark's reveal, the transition to the
/// settled ambient path, and the first settled frames — the most expensive
/// stretch of the animation.
const FRAMES: usize = 30;

/// Milliseconds of animation between frames in a batch.
const STEP_MILLIS: u64 = 10;

/// Animation position the batch starts from.
const START_MILLIS: u64 = 600;

/// Batches timed, of which the cheapest observation is kept.
///
/// Five is enough for one to land in a quiet slice of the scheduler without
/// making the test slow: a batch costs well under half a second.
const BATCHES: usize = 5;

/// Composes one batch, returning each frame's cost in microseconds.
fn batch(canvas: &mut Canvas, lab: &StartupLab) -> [f64; FRAMES] {
    let mut costs = [0.0; FRAMES];
    for (index, cost) in costs.iter_mut().enumerate() {
        let at = START_MILLIS + index as u64 * STEP_MILLIS;
        let started = std::time::Instant::now();
        startup_lab::draw(canvas, lab, at);
        *cost = started.elapsed().as_nanos() as f64 / 1_000.0;
    }
    costs
}

/// Returns the cheapest cost observed for each frame across [`BATCHES`].
///
/// Frames are kept apart rather than averaged because the animation is not
/// uniform: composing the mark's reveal costs several times what a settled
/// frame costs, and a mean hides which frames are near the interval.
fn cheapest_frames() -> [f64; FRAMES] {
    let lab = sample_lab();
    let mut canvas = Canvas::new(1_240, 900);
    // Warm the glyph and backdrop caches, as the first real frame does.
    startup_lab::draw(&mut canvas, &lab, START_MILLIS);

    let mut best = [f64::INFINITY; FRAMES];
    for _ in 0..BATCHES {
        for (kept, measured) in best.iter_mut().zip(batch(&mut canvas, &lab)) {
            *kept = kept.min(measured);
        }
    }
    best
}

/// The interval a frame has to fit inside, in microseconds.
fn budget_micros() -> f64 {
    f64::from(REVEAL_INTERVAL_MILLIS) * 1_000.0
}

#[test]
fn an_animated_frame_fits_inside_the_timer_interval() {
    let frames = cheapest_frames();
    let mean = frames.iter().sum::<f64>() / FRAMES as f64;
    let budget = budget_micros();
    // Reported on success as well as failure: a number that only appears
    // when the guard trips cannot show the trend that precedes it.
    println!("mean frame {mean:.0} us of a {budget:.0} us budget");
    assert!(
        mean < budget,
        "the cheapest of {BATCHES} batches still averages {mean:.0} us per frame, \
         over the {budget:.0} us budget"
    );
}

#[test]
fn no_single_frame_of_the_reveal_overruns_the_interval() {
    let frames = cheapest_frames();
    let (index, worst) = frames
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .expect("a batch composes at least one frame");
    let at = START_MILLIS + index as u64 * STEP_MILLIS;
    let budget = budget_micros();
    println!("worst frame {worst:.0} us at {at} ms of a {budget:.0} us budget");
    // The mean can sit comfortably inside the interval while one frame
    // overruns it, and it is the single frame that drops, not the mean.
    assert!(
        *worst < budget,
        "the frame at {at} ms costs {worst:.0} us, over the {budget:.0} us budget"
    );
}
