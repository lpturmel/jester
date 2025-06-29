use jester::App;

fn main() {
    tracing_subscriber::fmt::init();

    App::new("cool game".to_string()).run().ok();
}
