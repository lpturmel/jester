#[derive(Default, Debug, Clone, Copy)]
pub struct FpsStats {
    frame_count: u32,
    acc_time: f32,
    pub fps: f32,
    pub frame_ms: f32,
}

impl FpsStats {
    pub fn tick(&mut self, dt: f32) {
        self.frame_count += 1;
        self.acc_time += dt;

        if self.acc_time >= 1.0 {
            self.fps = self.frame_count as f32 / self.acc_time;
            self.frame_ms = 1_000.0 / self.fps; // ms
            self.frame_count = 0;
            self.acc_time = 0.0;
        }
    }
}
