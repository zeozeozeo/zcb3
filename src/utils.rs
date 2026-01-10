#[cfg(target_arch = "wasm32")]
pub fn setup_wasm() {
    // Set up better panic messages
    console_error_panic_hook::set_once();

    // Initialize logger for browser console
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");
}

// Cross-platform timer
pub struct Timer {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    start: f64,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            start: web_sys::window().unwrap().performance().unwrap().now(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let now = web_sys::window().unwrap().performance().unwrap().now();
            std::time::Duration::from_secs_f64((now - self.start) / 1000.0)
        }
    }
}
