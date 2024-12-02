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
                curve: envelope::EnvelopeCurve::Exponential { tightness: -7.0, slope_offset: 1.0 }
            },
        ],
        75,
        50.0,
    ) else {
        panic!();
    };

    let envelope = envelope.right_end_completion(envelope::EnvelopeEndCompletion::HoldSlope);

    for i in 0..=90 as i64 {
        println!(
            "{}",
            "-".repeat(envelope.value(i as f64) as usize) + "*"
        );
    }
}
