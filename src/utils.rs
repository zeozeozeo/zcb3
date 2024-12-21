use std::future::Future;

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F: Future<Output = ()> + 'static + Send>(f: F) {
    tokio::spawn(f);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
