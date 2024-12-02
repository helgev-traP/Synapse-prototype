use core::f64;

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

impl EnvelopeContent {
    pub fn value(&self, frame: f64, next_keyframe: FrameCount, next_value: f64) -> f64 {
        let self_keyframe = self.frame as f64;
        let next_keyframe = next_keyframe as f64;
        let next_value = next_value as f64;

        match &self.curve {
            EnvelopeCurve::Const(v) => *v,
            EnvelopeCurve::Exponential {
                tightness,
                slope_offset,
            } => {
                if *tightness != 0.0 {
                    // exponential interpolation
                    (next_value - slope_offset * (next_keyframe - self_keyframe) - self.value)
                        / (f64::consts::E.powf(*tightness) - 1.0)
                        * (f64::consts::E.powf(
                            tightness * (frame - self_keyframe) / (next_keyframe - self_keyframe),
                        ) - 1.0)
                        + self.value
                        + slope_offset * (frame - self_keyframe)
                } else {
                    // linear interpolation
                    self.value * (next_keyframe - frame) / (next_keyframe - self_keyframe)
                        + next_value * (frame - self_keyframe) / (next_keyframe - self_keyframe)
                }
            }
        }
    }

    pub fn slope(&self, frame: f64, next_keyframe: FrameCount, next_value: f64) -> f64 {
        let self_keyframe = self.frame as f64;
        let next_keyframe = next_keyframe as f64;
        let next_value = next_value as f64;

        match &self.curve {
            EnvelopeCurve::Const(_) => 0.0,
            EnvelopeCurve::Exponential {
                tightness,
                slope_offset,
            } => {
                if *tightness != 0.0 {
                    // exponential interpolation
                    tightness / (next_keyframe - self_keyframe)
                        * (next_value
                            - slope_offset * (next_keyframe - self_keyframe)
                            - self.value)
                        / (f64::consts::E.powf(*tightness) - 1.0)
                        * (f64::consts::E.powf(
                            tightness * (frame - self_keyframe) / (next_keyframe - self_keyframe),
                        ))
                        + slope_offset
                } else {
                    // linear interpolation
                    (next_value - self.value) / (next_keyframe - self_keyframe)
                }
            }
        }
    }
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
    // keyframes are guaranteed to be sorted by frame.
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
            curves: vec![EnvelopeContent {
                frame: 0,
                value: 0.0,
                curve: EnvelopeCurve::Exponential {
                    tightness: 0.0,
                    slope_offset: 1.0,
                },
            }],
            right_end_frame: 1,
            right_end_value: 1.0,
            right_end_completion: EnvelopeEndCompletion::HoldSlope,
            left_end_completion: EnvelopeEndCompletion::HoldSlope,
        }
    }

    pub fn new_vec(
        v: Vec<EnvelopeContent>,
        right_end_frame: FrameCount,
        right_end_value: f64,
    ) -> Result<Self, Vec<EnvelopeContent>> {
        // check frame is sorted
        for i in 0..v.len() - 1 {
            if v[i].frame >= v[i + 1].frame {
                return Err(v);
            }
        }
        if v[v.len() - 1].frame > right_end_frame {
            return Err(v);
        }

        Ok(Envelope {
            curves: v,
            right_end_frame,
            right_end_value,
            right_end_completion: EnvelopeEndCompletion::HoldValue,
            left_end_completion: EnvelopeEndCompletion::HoldValue,
        })
    }

    pub fn right_end_completion(mut self, completion: EnvelopeEndCompletion) -> Self {
        self.right_end_completion = completion;
        self
    }

    pub fn left_end_completion(mut self, completion: EnvelopeEndCompletion) -> Self {
        self.left_end_completion = completion;
        self
    }

    pub fn value(&self, frame: f64) -> f64 {
        // find EnvelopeContent that contains frame
        // no curves
        if self.curves.is_empty() {
            return self.right_end_value;
        }

        let carves_len = self.curves.len();

        if frame < self.curves[0].frame as f64 {
            // left end completion
            match self.left_end_completion {
                EnvelopeEndCompletion::Specific(v) => v,
                EnvelopeEndCompletion::HoldValue => self.curves[0].value,
                EnvelopeEndCompletion::HoldSlope => {
                    let slope = if carves_len == 1 {
                        self.curves[0].slope(
                            self.curves[0].frame as f64,
                            self.right_end_frame,
                            self.right_end_value,
                        )
                    } else {
                        self.curves[0].slope(
                            self.curves[0].frame as f64,
                            self.curves[1].frame,
                            self.curves[1].value,
                        )
                    };

                    slope * (frame - self.curves[0].frame as f64) + self.curves[0].value
                }
                EnvelopeEndCompletion::HoldCurve => {
                    if carves_len == 1 {
                        self.curves[0].value(frame, self.right_end_frame, self.right_end_value)
                    } else {
                        self.curves[0].value(frame, self.curves[1].frame, self.curves[1].value)
                    }
                }
            }
        } else if (self.right_end_frame as f64) < frame {
            // right end completion
            match self.right_end_completion {
                EnvelopeEndCompletion::Specific(v) => v,
                EnvelopeEndCompletion::HoldValue => self.right_end_value,
                EnvelopeEndCompletion::HoldSlope => {
                    let slope = self.curves[carves_len - 1].slope(
                        self.right_end_frame as f64,
                        self.right_end_frame,
                        self.right_end_value,
                    );

                    slope * (frame - self.right_end_frame as f64) + self.right_end_value
                }
                EnvelopeEndCompletion::HoldCurve => self.curves[carves_len - 1].value(
                    frame,
                    self.right_end_frame,
                    self.right_end_value,
                ),
            }
        } else {
            // find EnvelopeContent that contains frame
            // binary search
            let mut left = 0;
            let mut right = carves_len - 1;
            while left < right {
                let mid = (left + right) / 2 + 1;
                if (self.curves[mid].frame as f64) < frame {
                    left = mid;
                } else {
                    right = mid - 1;
                }
            }
            let index = left;

            // left is the index of EnvelopeContent that contains frame
            if index == carves_len - 1 {
                self.curves[index].value(frame, self.right_end_frame, self.right_end_value)
            } else {
                self.curves[index].value(
                    frame,
                    self.curves[index + 1].frame,
                    self.curves[index + 1].value,
                )
            }
        }
    }
}
