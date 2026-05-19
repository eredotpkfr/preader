use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read},
    path::Path,
};

const DEFAULT_CHUNK_SIZE: usize = 5;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: preader <file_path>");

    println!("== read_byte_by_byte ==");
    read_byte_by_byte(&path).unwrap();

    println!("\n== read_chunk_by_chunk (chunk_size = 5) ==");
    read_chunk_by_chunk(&path, DEFAULT_CHUNK_SIZE).unwrap();

    println!("\n== read_line_by_line ==");
    read_line_by_line(&path).unwrap();
}

fn print_progress(percent: f64, bytes_read: u64, total_bytes: u64, chunk: &[u8]) {
    let text = String::from_utf8_lossy(chunk);
    println!("[{percent:>6.2}%] {bytes_read}/{total_bytes} bytes ({text:?})");
}

fn read_byte_by_byte(path: impl AsRef<Path>) -> io::Result<()> {
    let file = File::open(path)?;
    let total_bytes = file.metadata()?.len();

    let reader = BufReader::new(file);
    let mut bytes_read: u64 = 0;
    let mut next_milestone: u64 = 1;

    for byte in reader.bytes() {
        let byte = byte?;
        bytes_read += 1;

        if bytes_read >= next_milestone {
            let percent_int = (bytes_read * 100) / total_bytes;
            let percent = bytes_read as f64 * 100.0 / total_bytes as f64;
            print_progress(percent, bytes_read, total_bytes, &[byte]);
            next_milestone = ((percent_int + 1) * total_bytes).div_ceil(100);
        }
    }

    println!("Done. Read {bytes_read} bytes total.");
    Ok(())
}

fn read_chunk_by_chunk(path: impl AsRef<Path>, chunk_size: usize) -> io::Result<()> {
    let mut file = File::open(path)?;
    let total_bytes = file.metadata()?.len();

    let mut buffer = vec![0u8; chunk_size];
    let mut bytes_read: u64 = 0;
    let mut next_milestone: u64 = 1;

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes_read += n as u64;

        if bytes_read >= next_milestone {
            let percent_int = (bytes_read * 100) / total_bytes;
            let percent = bytes_read as f64 * 100.0 / total_bytes as f64;
            print_progress(percent, bytes_read, total_bytes, &buffer[..n]);
            next_milestone = ((percent_int + 1) * total_bytes).div_ceil(100);
        }
    }

    println!("Done. Read {bytes_read} bytes total.");
    Ok(())
}

fn read_line_by_line(path: impl AsRef<Path>) -> io::Result<()> {
    let file = File::open(path)?;
    let total_bytes = file.metadata()?.len();

    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    let mut bytes_read: u64 = 0;
    let mut next_milestone: u64 = 1;

    loop {
        buffer.clear();
        let n = reader.read_until(b'\n', &mut buffer)?;
        if n == 0 {
            break;
        }
        bytes_read += n as u64;

        if bytes_read >= next_milestone {
            let percent_int = (bytes_read * 100) / total_bytes;
            let percent = bytes_read as f64 * 100.0 / total_bytes as f64;
            print_progress(percent, bytes_read, total_bytes, &buffer);
            next_milestone = ((percent_int + 1) * total_bytes).div_ceil(100);
        }
    }

    println!("Done. Read {bytes_read} bytes total.");
    Ok(())
}
