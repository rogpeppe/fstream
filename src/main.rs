#![recursion_limit = "500"]

use snafu::{ResultExt, Snafu};

#[tokio::main]
async fn main() {
    // Set up an arbitrary configuration, the equivalent of:
    //
    // 	walk /tmp | filter {mode +d} | print
    //
    // TODO implement a parser to enable setting this
    // up dynamically according to some specified syntax.

    let (send_root1, recv_root1) = fstream::new();
    let walker = tokio::spawn(async { walk::walk("/tmp", send_root1).await.context(ErrWalk) });
    let (send_root2, recv_root2) = fstream::new();
    let filterer = tokio::spawn(async {
        filter::filter(recv_root1, send_root2, |entry, _path| {
            // TODO change filter function to return Result?
            entry.file_type().expect("file type").is_dir()
        })
        .await
        .context(ErrFilter)
    });
    let printer = tokio::spawn(async { print::print(recv_root2).await.context(ErrPrint) });

    if let Err(err) = walker.await.unwrap() {
        println!("walker error: {}", err);
    } else {
        println!("walker ok");
    }
    if let Err(err) = filterer.await.unwrap() {
        println!("filter error: {}", err);
    } else {
        println!("filter ok");
    }
    if let Err(err) = printer.await.unwrap() {
        println!("printer error: {}", err);
    } else {
        println!("printer ok");
    }
}

#[derive(Debug, Snafu)]
enum Error {
    ErrPrint { source: print::Error },
    ErrWalk { source: walk::Error },
    ErrFilter { source: filter::Error },
}

pub mod filter;
pub mod fstream;
pub mod print;
pub mod walk;
