use std::io;

use io_uring::IoUring;

fn main() -> io::Result<()> {
    let _ring = IoUring::new(256)?;

    Ok(())
}
