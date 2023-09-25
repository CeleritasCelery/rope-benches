// #[macro_use]
extern crate criterion;
use crdt_testdata::*;
use criterion::*;
use rand::prelude::*;
mod rope;
use self::rope::*;
use crop::Rope as CropRope;
use get_size::GetSize;
use jumprope::JumpRope;
use regex::Regex;
use ropey::Rope as RopeyRope;
use std::any::type_name;
use std::{
    borrow::Cow,
    cmp::min,
    fs::File,
    io::{BufReader, Read},
};
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
    let mut count = len as isize;
    let mut string = String::new();
    while count > 0 {
        let chr = std::char::from_u32(rng.gen_range(0x0000..0xD7FF)).unwrap();
        count -= chr.len_utf8() as isize;
        string.push(chr);
    }
    string
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
    }

    fn line_search(&self, re: &regex::Regex) -> usize {
        self.full_search(re)
    }

    fn byte_len(&self) -> usize {
        self.len_bytes()
    }
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
        self.chunks().collect()
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.len_chars()
    }

    fn line_search(&self, re: &regex::Regex) -> usize {
        let mut lines = self.lines();
        let mut offset: usize = 0;
        lines.find(|line| {
            let cow: Cow<str> = line.clone().into();
            let re_match = match cow {
                Cow::Borrowed(x) => re.find(x).map(|x| x.start()),
                Cow::Owned(x) => re.find(x.as_str()).map(|x| x.start()),
            };
            match re_match {
                Some(x) => {
                    offset += x;
                    true
                }
                None => {
                    offset += line.len_bytes();
                    false
                }
            }
        });
        offset
    }

    fn byte_len(&self) -> usize {
        self.len_bytes()
    }
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

    fn line_search(&self, re: &regex::Regex) -> usize {
        let mut lines = self.raw_lines();
        let mut offset: usize = 0;
        lines.find(|line| {
            let chunks = line.chunks();
            let buf: Vec<&str> = chunks.collect();
            let re_match = if buf.len() == 1 {
                re.find(buf[0]).map(|x| x.start())
            } else {
                let string: String = buf.iter().map(|x| *x).collect();
                re.find(string.as_str()).map(|x| x.start())
            };

            match re_match {
                Some(x) => {
                    offset += x;
                    true
                }
                None => {
                    offset += line.byte_len();
                    false
                }
            }
        });
        offset
    }

    fn byte_len(&self) -> usize {
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
    fn get_string(&self) -> Cow<'_, str> {
        self.read(..)
    }

    #[inline(always)]
    fn char_len(&self) -> usize {
        self.len_chars()
    }

    fn line_search(&self, re: &regex::Regex) -> usize {
        match self.read(..) {
            Cow::Borrowed(x) => re.find(x).map(|x| x.start()).unwrap_or_else(|| self.len()),
            Cow::Owned(_) => unreachable!(),
        }
    }

    fn full_search(&self, re: &regex::Regex) -> usize {
        self.line_search(re)
    }

    fn byte_len(&self) -> usize {
        self.len()
    }
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

fn search_linewise<R: Rope + From<String>>(b: &mut Bencher) {
    let filename = format!("{}/data/realworld.txt", env!("CARGO_MANIFEST_DIR"));
    // read the file into a string
    let file = File::open(filename).unwrap();
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    // duplicate contents 40 times
    let contents = contents.repeat(40);

    let r = R::from(contents);

    let re = Regex::new(r"foo(bar|baz)fob").unwrap();
    b.iter(|| {
        black_box(r.line_search(&re));
    });
}

fn search_full<R: Rope + From<String>>(b: &mut Bencher, size: &usize) {
    let filename = format!("{}/data/realworld.txt", env!("CARGO_MANIFEST_DIR"));
    // read the file into a string
    let file = File::open(filename).unwrap();
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    let repeat = size / contents.len();
    let contents = contents.repeat(repeat);
    let len = contents.len();

    let r = R::from(contents);

    let re = Regex::new(r"(?s)foo..fob").unwrap();
    b.iter(|| {
        let idx = r.full_search(&re);
        assert_eq!(idx, len);
        black_box(idx);
    });
}

fn build_string<R: Rope + From<String>>(b: &mut Bencher, size: &usize) {
    let filename = format!("{}/data/realworld.txt", env!("CARGO_MANIFEST_DIR"));
    // read the file into a string
    let file = File::open(filename).unwrap();
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    let repeat = size / contents.len();
    let contents = contents.repeat(repeat);

    let r = R::from(contents);

    b.iter(|| {
        black_box(r.get_string());
    });
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

fn space_overhead<R: From<String> + GetSize>(size: usize) {
    let rng = &mut SmallRng::seed_from_u64(123);
    let string = random_string(rng, size);
    let len = string.len();
    let rope = R::from(string);
    let size = GetSize::get_size(&rope);
    let overhead = size - len;
    let percent_overhead = (overhead as f64 / len as f64) * 100.0;
    println!("{}: {:.2}%", type_name::<R>(), percent_overhead);
}

fn space_overhead_edits<R: Rope + From<String> + GetSize>(size: usize) {
    let test_data = load_named_data("automerge-paper");

    let mut r = R::new();
    for _ in 0..size {
        for txn in &test_data.txns {
            for TestPatch(pos, del, ins) in &txn.patches {
                r.edit_at(*pos, *del, &ins);
            }
        }
    }
    let len = r.byte_len();
    let size = GetSize::get_size(&r);
    let overhead = size - len;
    let percent_overhead = (overhead as f64 / len as f64) * 100.0;
    println!("{}: {:.2}%", type_name::<R>(), percent_overhead);
}

#[allow(unused)]
fn report_space_overhead() {
    let size = usize::pow(2, 20);
    space_overhead::<Buffer>(size);
    space_overhead::<JumpRope>(size);
    space_overhead::<RopeyRope>(size);
    space_overhead::<CropRope>(size);
}

#[allow(unused)]
fn report_space_overhead_edits() {
    let size = 20;
    space_overhead_edits::<Buffer>(size);
    space_overhead_edits::<JumpRope>(size);
    space_overhead_edits::<RopeyRope>(size);
    space_overhead_edits::<CropRope>(size);
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
    group.bench_function("ropey", append_small::<RopeyRope>);
    group.finish();
}

#[allow(unused)]
fn bench_search_linewise(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_linewise");
    group.sample_size(50);

    group.bench_function("buffer", search_linewise::<Buffer>);
    group.bench_function("jumprope", search_linewise::<JumpRope>);
    group.bench_function("ropey", search_linewise::<RopeyRope>);
    group.bench_function("crop", search_linewise::<CropRope>);
    group.finish();
}

#[allow(unused)]
fn bench_search_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_full");
    group.sample_size(50);

    for (size, sample) in &[(10, 100), (15, 100), (20, 50), (25, 10), (30, 10)] {
        group.sample_size(*sample);
        let size = &usize::pow(2, *size);
        let id = BenchmarkId::new("buffer", size);
        group.bench_with_input(id, size, search_full::<Buffer>);
        let id = BenchmarkId::new("jumprope", size);
        group.bench_with_input(id, size, search_full::<JumpRope>);
        let id = BenchmarkId::new("ropey", size);
        group.bench_with_input(id, size, search_full::<RopeyRope>);
        let id = BenchmarkId::new("crop", size);
        group.bench_with_input(id, size, search_full::<CropRope>);
    }
    group.finish();
}

#[allow(unused)]
fn bench_build_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_string");

    for (size, sample) in &[(10, 100), (20, 50), (30, 10)] {
        group.sample_size(*sample);
        let size = &usize::pow(2, *size);
        let id = BenchmarkId::new("buffer", size);
        group.bench_with_input(id, size, build_string::<Buffer>);
        let id = BenchmarkId::new("jumprope", size);
        group.bench_with_input(id, size, build_string::<JumpRope>);
        let id = BenchmarkId::new("ropey", size);
        group.bench_with_input(id, size, build_string::<RopeyRope>);
        let id = BenchmarkId::new("crop", size);
        group.bench_with_input(id, size, build_string::<CropRope>);
    }
    group.finish();
}

#[allow(unused)]
fn bench_ins_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("ins_random");

    group.bench_function("buffer", ins_random::<Buffer>);
    group.bench_function("jumprope", ins_random::<JumpRope>);
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
        x::<CropRope>(&mut group, name, &test_data_bytes);
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_create,
    bench_append_small,
    bench_search_linewise,
    bench_search_full,
    bench_build_string,
    bench_ins_random,
    bench_stable_ins_del,
    realworld
);
criterion_main!(benches);

// fn main() {
//     report_space_overhead()
// }
