pub fn pan(sample: f32, panning: f32) -> (f32, f32) {
    let left = sample * (1.0 - panning.abs());
    let right = sample * (1.0 - left);
    (left, right)
}
