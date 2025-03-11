#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsData {
    pub x_position: f32,
    pub y_position: f32,
    pub rotation: f32,
    pub x_velocity: f64,
    pub y_velocity: f64,
}

impl Default for PhysicsData {
    fn default() -> Self {
        Self {
            x_position: 0.0,
            y_position: 0.0,
            rotation: 0.0,
            x_velocity: 0.0,
            y_velocity: 0.0,
        }
    }
}
