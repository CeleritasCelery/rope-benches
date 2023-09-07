// #[macro_use]
extern crate criterion;
use criterion::*;

use crdt_testdata::*;

use rand::prelude::*;

mod rope;
use self::rope::*;
use jumprope::*;

use std::cmp::min;

use crop::Rope as CropRope;
use ropey::Rope as RopeyRope;
use text_buffer::Buffer;

const CHARS: &[u8; 83] =
    b" ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()[]{}<>?,./";

// Gross. Find a way to reuse the code from random_unicode_string.
fn random_ascii_string(rng: &mut SmallRng, len: usize) -> String {
    let mut s = String::new();
    for _ in 0..len {
        s.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
    }
    s
}

fn random_string(rng: &mut SmallRng, len: usize) -> String {
    (0..len)
        .map(|_| std::char::from_u32(rng.gen_range(0x0000..0xD7FF)).unwrap())
        .collect()
}

impl Rope for JumpRope {
    const NAME: &'static str = "JumpRope";

    #[inline(always)]
    fn new() -> Self {
        JumpRope::new()
    }

    #[inline(always)]
    fn insert_at(&mut self, pos: usize, contents: &str) {
        self.insert(pos, contents);
    }
    #[inline(always)]
    fn del_at(&mut self, pos: usize, len: usize) {
        self.remove(pos..pos + len);
    }

    #[inline(always)]
    fn edit_at(&mut self, pos: usize, del_len: usize, ins_content: &str) {
        self.replace(pos..pos + del_len, ins_content);
    }

    #[inline(always)]
    fn to_string(&self) -> String {
        ToString::to_string(self)
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.len_chars()
    } // in unicode values
}

impl Rope for JumpRopeBuf {
    const NAME: &'static str = "JumpRopeBuf";

    #[inline(always)]
    fn new() -> Self {
        JumpRopeBuf::new()
    }

    #[inline(always)]
    fn insert_at(&mut self, pos: usize, contents: &str) {
        self.insert(pos, contents);
    }
    #[inline(always)]
    fn del_at(&mut self, pos: usize, len: usize) {
        self.remove(pos..pos + len);
    }

    #[inline(always)]
    fn edit_at(&mut self, pos: usize, del_len: usize, ins_content: &str) {
        if del_len > 0 {
            self.remove(pos..pos + del_len);
        }
        if !ins_content.is_empty() {
            self.insert(pos, ins_content);
        }
    }

    #[inline(always)]
    fn to_string(&self) -> String {
        ToString::to_string(self)
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.len_chars()
    } // in unicode values
}

impl Rope for RopeyRope {
    const NAME: &'static str = "Ropey";

    #[inline(always)]
    fn new() -> Self {
        RopeyRope::new()
    }

    #[inline(always)]
    fn insert_at(&mut self, pos: usize, contents: &str) {
        self.insert(pos, contents);
    }
    #[inline(always)]
    fn del_at(&mut self, pos: usize, len: usize) {
        self.remove(pos..pos + len);
    }

    #[inline(always)]
    fn to_string(&self) -> String {
        unimplemented!()
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.len_chars()
    } // in unicode values
}

impl Rope for CropRope {
    const NAME: &'static str = "Crop";
    const EDITS_USE_BYTE_OFFSETS: bool = true;

    fn new() -> Self {
        Self::new()
    }

    fn insert_at(&mut self, pos: usize, contents: &str) {
        self.insert(pos, contents);
    }

    fn del_at(&mut self, pos: usize, len: usize) {
        self.delete(pos..pos + len)
    }

    fn to_string(&self) -> String {
        ToString::to_string(self)
    }

    fn char_len(&self) -> usize {
        self.byte_len()
    }
}
impl Rope for Buffer {
    const NAME: &'static str = "Buffer";

    #[inline(always)]
    fn new() -> Self {
        Buffer::new()
    }

    #[inline(always)]
    fn insert_at(&mut self, pos: usize, contents: &str) {
        self.set_cursor(pos);
        self.insert(contents);
    }
    #[inline(always)]
    fn del_at(&mut self, pos: usize, len: usize) {
        self.delete_range(pos, pos + len);
    }

    #[inline(always)]
    fn to_string(&self) -> String {
        unimplemented!()
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.len_chars()
    } // in unicode values
}

use crdt_testdata::{load_testing_data, TestData};
use criterion::measurement::WallTime;

fn gen_strings(rng: &mut SmallRng) -> Vec<String> {
    // I wish there was a better syntax for just making an array here.
    let mut strings = Vec::<String>::new();
    for _ in 0..100 {
        let len = rng.gen_range(1..3);
        strings.push(random_ascii_string(rng, len));
    }
    strings
}

fn gen_small_string(rng: &mut SmallRng) -> Vec<String> {
    let mut strings = Vec::<String>::new();
    for _ in 0..1000 {
        let len = rng.gen_range(1..3);
        strings.push(random_string(rng, len));
    }
    strings
}

fn append_small<R: Rope>(b: &mut Bencher) {
    let mut rng = SmallRng::seed_from_u64(123);
    let strings = gen_small_string(&mut rng);

    let mut r = R::new();
    let mut len = 0;
    b.iter(|| {
        for text in &strings {
            r.insert_at(len, text.as_str());
            len += text.chars().count();
        }
    });

    black_box(r.char_len());
}

fn ins_random<R: Rope>(b: &mut Bencher) {
    let mut rng = SmallRng::seed_from_u64(123);
    let strings = gen_strings(&mut rng);

    let mut r = R::new();
    // Len isn't needed, but its here to allow direct comparison with ins_append.
    let mut len = 0;
    b.iter(|| {
        let pos = rng.gen_range(0..len + 1);
        let text = &strings[rng.gen_range(0..strings.len())];
        r.insert_at(pos, text.as_str());
        len += text.chars().count();
    });

    black_box(r.char_len());
    black_box(len);
}

fn create<R: for<'a> From<&'a str>>(b: &mut Bencher) {
    let rng = &mut SmallRng::seed_from_u64(123);
    let string = random_string(rng, usize::pow(2, 20));
    let init = string.as_str();
    b.iter(|| {
        black_box(R::from(init));
    });
}

fn stable_ins_del<R: Rope + From<String>>(b: &mut Bencher, target_length: &u64) {
    let target_length = *target_length as usize;
    let mut rng = SmallRng::seed_from_u64(123);

    // I wish there was a better syntax for just making an array here.
    let strings = gen_strings(&mut rng);

    let mut r = R::from(random_string(&mut rng, target_length));
    let mut len = target_length;

    b.iter(|| {
        if len <= target_length {
            // Insert
            let pos = rng.gen_range(0..len + 1);
            let text = &strings[rng.gen_range(0..strings.len())];
            r.insert_at(pos, text.as_str());
            len += text.chars().count();
        } else {
            // Delete
            let pos = rng.gen_range(0..len);
            let dlen = min(rng.gen_range(0..10), len - pos);
            len -= dlen;

            r.del_at(pos, dlen);
        }
    });

    // Return something based on the computation to avoid it being optimized
    // out. Although right now the compiler isn't smart enough for that
    // anyway.
    // r.len()
    black_box(r.char_len());
}

#[allow(unused)]
fn bench_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("create");
    group.bench_function("buffer", create::<Buffer>);
    group.bench_function("jumprope", create::<JumpRope>);
    group.bench_function("ropey", create::<RopeyRope>);
    group.bench_function("crop", create::<CropRope>);
    group.finish();
}

#[allow(unused)]
fn bench_append_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("append_small");

    group.bench_function("buffer", append_small::<Buffer>);
    group.bench_function("jumprope", append_small::<JumpRope>);
    group.bench_function("jumprope-buf", append_small::<JumpRopeBuf>);
    group.bench_function("ropey", append_small::<RopeyRope>);
    group.finish();
}

#[allow(unused)]
fn bench_ins_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("ins_random");

    group.bench_function("buffer", ins_random::<Buffer>);
    group.bench_function("jumprope", ins_random::<JumpRope>);
    group.bench_function("jumprope-buf", ins_random::<JumpRopeBuf>);
    group.bench_function("ropey", ins_random::<RopeyRope>);
    group.bench_function("crop", ins_random::<CropRope>);
    group.finish();
}

#[allow(unused)]
fn bench_stable_ins_del(c: &mut Criterion) {
    let mut group = c.benchmark_group("stable_ins_del");

    for size in [1000, 100000, 10000000].iter() {
        group.throughput(Throughput::Elements(*size));
        group.bench_with_input(
            BenchmarkId::new("buffer", size),
            size,
            stable_ins_del::<Buffer>,
        );
        group.bench_with_input(
            BenchmarkId::new("jumprope", size),
            size,
            stable_ins_del::<JumpRope>,
        );
        group.bench_with_input(
            BenchmarkId::new("ropey", size),
            size,
            stable_ins_del::<RopeyRope>,
        );
        group.bench_with_input(
            BenchmarkId::new("crop", size),
            size,
            stable_ins_del::<CropRope>,
        );
    }
    group.finish();
}

fn load_named_data(name: &str) -> TestData {
    let filename = format!(
        "{}/../benchmark_data/{}.json.gz",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    load_testing_data(&filename)
}

const DATASETS: &[&str] = &[
    "automerge-paper",
    "rustcode",
    "sveltecomponent",
    "seph-blog1",
];

fn realworld(c: &mut Criterion) {
    for name in DATASETS {
        let mut group = c.benchmark_group("realworld");
        let test_data_chars = load_named_data(name);
        group.throughput(Throughput::Elements(test_data_chars.len() as u64));
        let test_data_bytes = test_data_chars.chars_to_bytes();

        fn x<R: Rope>(group: &mut BenchmarkGroup<WallTime>, name: &str, test_data: &TestData) {
            assert_eq!(R::EDITS_USE_BYTE_OFFSETS, test_data.using_byte_positions);

            group.bench_function(BenchmarkId::new(R::NAME, name), |b| {
                b.iter(|| {
                    let mut r = R::new();
                    for txn in &test_data.txns {
                        for TestPatch(pos, del, ins) in &txn.patches {
                            r.edit_at(*pos, *del, ins);
                        }
                    }
                    assert_eq!(r.char_len(), test_data.end_content.len());
                    black_box(r.char_len());
                })
            });
        }

        x::<Buffer>(&mut group, name, &test_data_chars);
        x::<RopeyRope>(&mut group, name, &test_data_chars);
        x::<JumpRope>(&mut group, name, &test_data_chars);
        x::<JumpRopeBuf>(&mut group, name, &test_data_chars);
        x::<CropRope>(&mut group, name, &test_data_bytes);

        // This takes a long time to run.
        // x::<String>(&mut group, name, &test_data);

        group.finish();
    }
}

criterion_group!(
    benches,
    bench_create,
    bench_append_small,
    bench_ins_random,
    bench_stable_ins_del,
    realworld
);
// criterion_group!(benches, bench_all);
criterion_main!(benches);
