use std::io::BufWriter;

use freyja_engine::protocol::Protocol;

/// Stack size for the main protocol/search thread (256 MB).
/// The 14x14 board with 4-player Max^n + qsearch creates deep recursion,
/// especially at depth 4+ in midgame positions with many captures.
/// Each frame in the recursive search is large (ArrayVec<Move, 256>, Score4, etc.).
const STACK_SIZE: usize = 256 * 1024 * 1024;

fn main() {
    // Tracing goes to stderr — stdout is exclusively for protocol messages.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    // Spawn the protocol handler on a thread with a larger stack.
    let builder = std::thread::Builder::new()
        .name("freyja-main".into())
        .stack_size(STACK_SIZE);

    let handler = builder
        .spawn(|| {
            let stdin = std::io::stdin().lock();
            let stdout = BufWriter::new(std::io::stdout().lock());

            let mut protocol = Protocol::new(stdout);
            protocol.run(stdin);
        })
        .expect("failed to spawn main thread");

    handler.join().expect("main thread panicked");
}
