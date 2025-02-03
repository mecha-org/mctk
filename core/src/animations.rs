use std::hash::Hash;

pub trait EasingFunction {
    fn ease(t: f64) -> f64;
}

#[derive(Copy, Clone, Debug)]
pub enum AnimationRepeat {
    Once,
    Loop,
    PingPong,
}

#[derive(Copy, Clone, Debug)]
pub struct Animation<F: EasingFunction> {
    start_time: std::time::Instant,
    duration: std::time::Duration,
    repeat: AnimationRepeat,
    _phantom: std::marker::PhantomData<F>,
}

impl<F: EasingFunction> Default for Animation<F> {
    fn default() -> Self {
        Self::new(std::time::Duration::from_secs(1), AnimationRepeat::Once)
    }
}

impl<F: EasingFunction> Hash for Animation<F> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start_time.hash(state);
        self.duration.hash(state);
        (((u32::MAX as f64) * self.get_value()) as u32).hash(state);
    }
}

impl<F: EasingFunction> Animation<F> {
    pub fn new(duration: std::time::Duration, repeat: AnimationRepeat) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            duration,
            repeat,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn reset(&mut self) {
        self.start_time = std::time::Instant::now();
    }

    pub fn get_value(&self) -> f64 {
        let t = self.start_time.elapsed().as_secs_f64() / self.duration.as_secs_f64();
        match self.repeat {
            AnimationRepeat::Once => {
                if t >= 1.0 {
                    1.0
                } else {
                    F::ease(t)
                }
            }
            AnimationRepeat::Loop => F::ease(t % 1.0),
            AnimationRepeat::PingPong => {
                let t = t % 2.0;
                if t >= 1.0 {
                    F::ease(2.0 - t)
                } else {
                    F::ease(t)
                }
            }
        }
    }
}

pub mod easing_functions {
    use super::EasingFunction;

    pub struct Linear;
    impl EasingFunction for Linear {
        fn ease(t: f64) -> f64 {
            t
        }
    }

    pub struct Quadratic;
    impl EasingFunction for Quadratic {
        fn ease(t: f64) -> f64 {
            t * t
        }
    }

    pub struct Cubic;
    impl EasingFunction for Cubic {
        fn ease(t: f64) -> f64 {
            t * t * t
        }
    }

    pub struct EaseOutQuadratic;
    impl EasingFunction for EaseOutQuadratic {
        fn ease(t: f64) -> f64 {
            -t * (t - 2.0)
        }
    }

    pub struct EaseInQuadratic;
    impl EasingFunction for EaseInQuadratic {
        fn ease(t: f64) -> f64 {
            t * t
        }
    }

    pub struct EaseInOutQuadratic;
    impl EasingFunction for EaseInOutQuadratic {
        fn ease(t: f64) -> f64 {
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
    }
}
