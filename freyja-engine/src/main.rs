use std::io::BufWriter;

use freyja_engine::protocol::Protocol;

fn main() {
    // Tracing goes to stderr — stdout is exclusively for protocol messages.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let stdin = std::io::stdin().lock();
    let stdout = BufWriter::new(std::io::stdout().lock());

    let mut protocol = Protocol::new(stdout);
    protocol.run(stdin);
}
