pub mod cmd;
pub mod item;
async fn client_worker() {
    todo!()
}
use indicatif::ProgressBar;
pub async fn progress_bar(pg:ProgressBar,name:&'static str) {
    let mut interval=tokio::time::interval(std::time::Duration::from_millis(100));
    pg.set_message(name);
    for _ in 0..50 {
        interval.tick().await;
        pg.inc(1);
    }
    pg.finish();
}