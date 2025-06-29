use jester::App;

fn main() {
    tracing_subscriber::fmt::init();

    let app = App::new("cool game".to_string());
    let _ = app.run();
}
