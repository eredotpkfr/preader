use preader::IteratorOptions;

const STARTS: [u64; 4] = [0, 1, 3, 7];
const ENDS: [u64; 3] = [2, 5, u64::MAX];
const SKIPS: [u64; 3] = [0, 1, 3];
const LIMITS: [u64; 3] = [1, 2, u64::MAX];
const SIZES: [usize; 4] = [0, 1, 3, 64];
const FLAGS: [bool; 2] = [false, true];

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub skip_empty: bool,
    pub keep: bool,
    pub align: bool,
}

pub fn windows() -> impl Iterator<Item = IteratorOptions> {
    STARTS.into_iter().flat_map(|start| {
        ENDS.into_iter().flat_map(move |end| {
            SKIPS.into_iter().flat_map(move |skip| {
                LIMITS.into_iter().map(move |limit| IteratorOptions {
                    start,
                    end,
                    skip,
                    limit,
                })
            })
        })
    })
}

pub fn shapes() -> impl Iterator<Item = Shape> {
    FLAGS.into_iter().flat_map(|keep| {
        FLAGS.into_iter().flat_map(move |skip_empty| {
            FLAGS.into_iter().map(move |align| Shape {
                keep,
                skip_empty,
                align,
            })
        })
    })
}

pub fn sizings() -> impl Iterator<Item = (usize, bool)> {
    SIZES
        .into_iter()
        .flat_map(|size| FLAGS.into_iter().map(move |drop_partial| (size, drop_partial)))
}
