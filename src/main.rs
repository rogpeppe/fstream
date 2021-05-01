#![recursion_limit="500"]

use snafu::{Snafu, ResultExt};

#[tokio::main]
async fn main() {
	let (tx_root, rx_root) = fstream::new();
	let walker = tokio::spawn(async {
		walk::walk("/home/rogpeppe/tmp", tx_root).await.context(ErrWalk)
	});
	let printer = tokio::spawn(async {
		print::print(rx_root).await.context(ErrPrint)
	});
	if let Err(err) = walker.await.unwrap() {
		println!("walker error: {}", err);
	} else {
		println!("walker ok");
	}
	if let Err(err) = printer.await.unwrap() {
		println!("printer error: {}", err);
	} else {
		println!("printer ok");
	}
}

#[derive(Debug, Snafu)]
enum Error {
	ErrPrint{
		source: print::Error,
	},
	ErrWalk{
		source: walk::Error,
	},
}

pub mod fstream;
pub mod walk;
pub mod print;
