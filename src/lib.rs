#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Peak {
    Low,
    High,
}

pub struct PeaksDetector {
    threshold: f64,
    influence: f64,
    window: Vec<f64>,
}

impl PeaksDetector {
    pub fn new(lag: usize, threshold: f64, influence: f64) -> PeaksDetector {
        PeaksDetector {
            threshold,
            influence,
            // The window is initialized with a capacity of lag since it is meant to contain
            // lookback values/rolling window data
            window: Vec::with_capacity(lag),
        }
    }

    /// Detects peaks in the signal using a z-score method. This method is also how the primary way to insert data into our rolling
    /// window, regardless of whether it is a peak or not
    pub fn z_score_signal(&mut self, value: f64) -> Option<Peak> {
        // If the window is not full, we just push the value and return None as it is clear there
        // is no complete window to analyze for peaks
        if self.window.len() < self.window.capacity() {
            self.window.push(value);

            None
        // If the window is full, we check if the new value is a peak. We check if the last value exists, and that the mean_stats
        // can be calculated. If so, we pop the first value in the window to make space for the new value, check if the new value
        // is a peak, and push the new value to the window. Finally, we return the peak if it exists.
        } else if let (Some((mean, stddev)), Some(&window_last)) =
            (self.stats(), self.window.last())
        {
            // We pop the first value in the window to make space for the new value
            self.window.remove(0);

            if (value - mean).abs() > (self.threshold * stddev) {
                // When we detect that a peak exists, we apply the influence factor to the new value so as to not
                // overreact to the new value. This is done by applying a weighted average to the new value and the
                // last value in the window
                let next_value = (value * self.influence) + ((1. - self.influence) * window_last);

                self.window.push(next_value);

                Some(if value > mean { Peak::High } else { Peak::Low })
            } else {
                // If the new value is not a peak, we just push it to the window and return None
                self.window.push(value);
                None
            }
        } else {
            None
        }
    }

    /// Returns the mean and standard deviation of the values in the window
    pub fn stats(&self) -> Option<(f64, f64)> {
        if self.window.is_empty() {
            None
        } else {
            let window_len = self.window.len() as f64;

            let mean = self.window.iter().fold(0., |a, v| a + v) / window_len;
            let sq_sum = self
                .window
                .iter()
                .fold(0., |a, v| a + ((v - mean) * (v - mean)));

            let stddev = (sq_sum / window_len).sqrt();

            Some((mean, stddev))
        }
    }
}

pub struct PeaksIter<I, F> {
    source: I,
    signal: F,
    detector: PeaksDetector,
}

pub trait PeaksFilter<I>
where
    I: Iterator,
{
    fn peaks<F>(self, detector: PeaksDetector, signal: F) -> PeaksIter<I, F>
    where
        F: FnMut(&I::Item) -> f64;
}

impl<I> PeaksFilter<I> for I
where
    I: Iterator,
{
    fn peaks<F>(self, detector: PeaksDetector, signal: F) -> PeaksIter<I, F>
    where
        F: FnMut(&I::Item) -> f64,
    {
        PeaksIter {
            source: self,
            signal,
            detector,
        }
    }
}

impl<I, F> Iterator for PeaksIter<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> f64,
{
    type Item = (I::Item, Peak);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.source.next() {
            let value = (self.signal)(&item);
            if let Some(peak) = self.detector.z_score_signal(value) {
                return Some((item, peak));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{Peak, PeaksDetector, PeaksFilter};

    #[test]
    fn sample_data() {
        let input = vec![
            1.0, 1.0, 1.1, 1.0, 0.9, 1.0, 1.0, 1.1, 1.0, 0.9, 1.0, 1.1, 1.0, 1.0, 0.9, 1.0, 1.0,
            1.1, 1.0, 1.0, 1.0, 1.0, 1.1, 0.9, 1.0, 1.1, 1.0, 1.0, 0.9, 1.0, 1.1, 1.0, 1.0, 1.1,
            1.0, 0.8, 0.9, 1.0, 1.2, 0.9, 1.0, 1.0, 1.1, 1.2, 1.0, 1.5, 1.0, 3.0, 2.0, 5.0, 3.0,
            2.0, 1.0, 1.0, 1.0, 0.9, 1.0, 1.0, 3.0, 2.6, 4.0, 3.0, 3.2, 2.0, 1.0, 1.0, 0.8, 4.0,
            4.0, 2.0, 2.5, 1.0, 1.0, 1.0,
        ];
        let output: Vec<_> = input
            .into_iter()
            .enumerate()
            .peaks(PeaksDetector::new(30, 5.0, 0.0), |e| e.1)
            .map(|((i, _), p)| (i, p))
            .collect();
        assert_eq!(
            output,
            vec![
                (45, Peak::High),
                (47, Peak::High),
                (48, Peak::High),
                (49, Peak::High),
                (50, Peak::High),
                (51, Peak::High),
                (58, Peak::High),
                (59, Peak::High),
                (60, Peak::High),
                (61, Peak::High),
                (62, Peak::High),
                (63, Peak::High),
                (67, Peak::High),
                (68, Peak::High),
                (69, Peak::High),
                (70, Peak::High),
            ]
        );
    }
}
