pub type FrameCount = i64;

// envelope

pub enum EnvelopeCurve {
    Const(f64),
    Exponential { tightness: f64, slope_offset: f64 },
}

pub struct EnvelopeContent {
    /// frame od keyframe
    pub frame: FrameCount,
    /// value of keyframe
    pub value: f64,
    /// completion curve between next keyframe
    pub curve: EnvelopeCurve,
}

pub enum EnvelopeEndCompletion {
    Specific(f64),
    HoldValue,
    HoldSlope,
    HoldCurve,
}

/// # Envelope
/// Input envelope.
pub struct Envelope {
    // keyframes and values
    curves: Vec<EnvelopeContent>,
    right_end_frame: FrameCount,
    right_end_value: f64,

    // end completion
    right_end_completion: EnvelopeEndCompletion,
    left_end_completion: EnvelopeEndCompletion,
}

impl Envelope {
    /// create new envelope that behave like y={value} for all x.
    pub fn new_value(value: f64) -> Self {
        Envelope {
            curves: vec![],
            right_end_frame: 0,
            right_end_value: value,
            right_end_completion: EnvelopeEndCompletion::HoldValue,
            left_end_completion: EnvelopeEndCompletion::HoldValue,
        }
    }

    /// create new envelope that behave like y=x for all x.
    pub fn new_pass_through() -> Self {
        Envelope {
            curves: vec![
                EnvelopeContent {
                    frame: 0,
                    value: 0.0,
                    curve: EnvelopeCurve::Exponential { tightness: 0.0, slope_offset: 1.0 }
                },
            ],
            right_end_frame: 1,
            right_end_value: 1.0,
            right_end_completion: EnvelopeEndCompletion::HoldSlope,
            left_end_completion: EnvelopeEndCompletion::HoldSlope,
        }
    }

    pub fn value(&self, frame: FrameCount) -> f64 {
        // todo
        frame as f64
    }
}