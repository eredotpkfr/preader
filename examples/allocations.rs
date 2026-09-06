use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use preader::{Config, IteratorBuild, IteratorRead, PReader, Result};

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);

        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const SMALL: usize = 20_000;
const LARGE: usize = 200_000;

fn write_lines(path: &Path, lines: usize) {
    let mut file = File::create(path).unwrap();

    for index in 0..lines {
        writeln!(
            file,
            "line {index:06} lorem ipsum dolor sit amet consectetur"
        )
        .unwrap();
    }

    file.sync_all().unwrap();
}

fn measure(label: &str, mut run: impl FnMut() -> usize) -> (usize, usize) {
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let items = run();
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;

    println!("  {label:<26} {items:>9} item -> {allocations:>3} tahsis");

    (items, allocations)
}

fn main() -> Result<()> {
    let root = PathBuf::from(std::env::args().nth(1).expect("usage: allocations <dir>"));

    fs::create_dir_all(&root)?;

    let small = root.join("small.log");
    let large = root.join("large.log");

    write_lines(&small, SMALL);
    write_lines(&large, LARGE);

    let reader = PReader::from(Config {
        state_dir: root.join("states"),
        ..Config::default()
    });

    // The same shape at two very different item counts: if the allocation count
    // does not move, nothing is allocated per item.
    for (kind, label) in [
        ("bytes", "bytes"),
        ("chunks", "chunks(size=64)"),
        ("lines", "lines"),
        ("delimiter", "delimiter(b' ')"),
    ] {
        println!("{label}:");

        let mut counts = Vec::new();

        for (scale, path) in [("kucuk", &small), ("buyuk", &large)] {
            let run = |path: &PathBuf| -> usize {
                let mut items = 0;

                match kind {
                    "bytes" => {
                        let mut it = reader.bytes(path).build().unwrap();
                        while it.read().unwrap().is_some() {
                            items += 1;
                        }
                    }
                    "chunks" => {
                        let mut it = reader.chunks(path).size(64).build().unwrap();
                        while let Some(chunk) = it.read().unwrap() {
                            items += chunk.len().min(1);
                        }
                    }
                    "lines" => {
                        let mut it = reader.lines(path).build().unwrap();
                        while let Some(line) = it.read().unwrap() {
                            items += line.len().min(1);
                        }
                    }
                    _ => {
                        let mut it = reader.delimiter(path).character(b' ').build().unwrap();
                        while let Some(field) = it.read().unwrap() {
                            items += field.len().min(1);
                        }
                    }
                }

                items
            };

            counts.push(measure(scale, || run(path)));
        }

        let (small_items, small_allocations) = counts[0];
        let (large_items, large_allocations) = counts[1];

        assert!(
            large_items > small_items * 5,
            "the two runs must differ by an order of magnitude"
        );
        assert_eq!(
            small_allocations, large_allocations,
            "{label}: allocation count grew with the item count"
        );

        println!(
            "  -> item sayisi {}x arttı, tahsis SABİT\n",
            large_items / small_items.max(1)
        );
    }

    // the owning adapter trades one allocation per item for std's Iterator
    println!("items() adaptoru (std Iterator, sahipli item):");

    for (label, owning) in [("read()", false), ("for-loop", true)] {
        let before = ALLOCATIONS.load(Ordering::Relaxed);
        let mut items = 0;

        if owning {
            for line in reader.lines(&small).build()? {
                items += line?.len().min(1);
            }
        } else {
            let mut it = reader.lines(&small).build()?;

            while let Some(line) = it.read()? {
                items += line.len().min(1);
            }
        }

        let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;

        println!("  lines {label:<9} {items:>7} item -> {allocations:>7} tahsis");
    }

    println!();
    println!("her sekilde read() tahsis sayisi item sayisindan BAGIMSIZ");

    Ok(())
}
