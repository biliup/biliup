


pub mod error;
pub mod extractor;
pub mod flv_parser;
pub mod flv_writer;
pub mod hls;
pub mod httpflv;
pub mod util;

// fn retry<O, E: std::fmt::Display>(mut f: impl FnMut() -> Result<O, E>) -> Result<O, E> {
//     let mut retries = 0;
//     let mut wait = 1;
//     loop {
//         match f() {
//             Err(e) if retries < 3 => {
//                 retries += 1;
//                 println!(
//                     "Retry attempt #{}. Sleeping {wait}s before the next attempt. {e}",
//                     retries,
//                 );
//                 sleep(Duration::from_secs(wait));
//                 wait *= 2;
//             }
//             res => break res,
//         }
//     }
// }
