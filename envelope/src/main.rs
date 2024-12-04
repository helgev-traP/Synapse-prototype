fn main() {
    // test interpolation
    let Ok(envelope) = envelope::Envelope::new_vec(
        vec![
            envelope::EnvelopeContent {
                frame: 0,
                value: 0.0,
                curve: envelope::EnvelopeCurve::Exponential { tightness: 0.0, slope_offset: 0.0 }
            },
            envelope::EnvelopeContent {
                frame: 25,
                value: 100.0,
                curve: envelope::EnvelopeCurve::Exponential { tightness: -6.0, slope_offset: 0.0 }
            },
            envelope::EnvelopeContent {
                frame: 50,
                value: 0.0,
                curve: envelope::EnvelopeCurve::Exponential { tightness: -200.0, slope_offset: 1.0 }
            },
        ],
        100,
        100.0,
    ) else {
        panic!();
    };

    let envelope = envelope.right_end_completion(envelope::EnvelopeEndCompletion::HoldSlope);

    for i in 0..=90 as i64 {
        println!(
            "frame: {:>4} |slope:{:>25} |{}",
            i,
            envelope.slope(i as f64),
            "-".repeat(envelope.value(i as f64) as usize) + "*"
        );
    }
}
