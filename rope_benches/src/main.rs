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

fn append<R: Rope + for<'a> From<&'a str>>(b: &mut Bencher) {
    let init = "a".repeat(1024);
    let mut r = R::from(&*init);
    let target = usize::pow(2, 20) - init.len();
    let text = "ropes";
    let count = target / text.len();
    let mut len = init.len();
    b.iter(|| {
        for _ in 0..count {
            r.insert_at(len, text);
            len += text.len();
        }
    });
}

fn is_even(x: usize) -> bool {
    x % 2 == 0
}

fn multiple_cursors_smart<R: Rope + for<'a> From<&'a str>>(
    b: &mut Bencher,
    params: &(usize, usize, usize, usize),
) {
    let size: usize = params.0;
    let cursors: usize = params.1;
    let step: usize = params.2;
    let width: usize = params.3;
    let text = "b";
    let l = text.len();
    let init = "a".repeat(size);
    let mut container = R::from(&*init);
    b.iter(|| {
        let orig_len = container.byte_len();
        for mc in 0..width {
            // every cursor will insert `width` characters
            // check if mc is odd

            for i in 0..cursors {
                let idx = if is_even(mc) {
                    (i * (step + ((mc + 1) * l))) + mc * l
                } else {
                    let i = cursors - 1 - i;
                    (i * (step + (mc * l))) + mc * l
                };
                container.insert_at(idx, text);
            }
        }
        if is_even(width) {
            // if even, last cursor was odd. so delete forwards
            let idx = (width - 1) * l;
            container.del_at(idx, cursors * width * l);
        } else {
            // if odd, last cursor was even. so delete backwards
            let idx = ((cursors - 1) * (step + (width * l))) + (width - 1) * l;
            let len = cursors * width * l;
            container.del_at(idx - len, len);
        }
        assert_eq!(container.byte_len(), orig_len);
    });
}

fn multiple_cursors_impl<R: Rope + for<'a> From<&'a str>>(
    b: &mut Bencher,
    params: &(usize, usize, usize, usize),
) {
    let size: usize = params.0;
    let cursors: usize = params.1;
    let step: usize = params.2;
    let width: usize = params.3;
    let text = "b";
    let l = text.len();
    let init = "a".repeat(size);
    let mut container = R::from(&*init);
    b.iter(|| {
        let orig_len = container.byte_len();
        for mc in 0..width {
            // every cursor will insert `width` characters
            for i in 0..cursors {
                let idx = (i * (step + ((mc + 1) * l))) + mc * l;
                container.insert_at(idx, text);
            }
        }
        let idx = ((cursors - 1) * (step + (width * l))) + (width - 1) * l;
        let del_len = cursors * width * l;
        container.del_at(idx - del_len, del_len);
        assert_eq!(container.byte_len(), orig_len);
    });
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

fn bench_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("from_string");
    group.sample_size(10);

    let size = usize::pow(2, 30);
    let string = "à".repeat(size / 2);
    assert_eq!(string.len(), size);

    group.bench_function("clone", |b| b.iter(|| string.clone()));
    group.bench_function("buffer", |b| b.iter(|| Buffer::from(string.clone())));
    group.bench_function("crop", |b| b.iter(|| CropRope::from(string.clone())));
    group.bench_function("jumprope", |b| b.iter(|| JumpRope::from(string.clone())));
    group.bench_function("ropey", |b| b.iter(|| RopeyRope::from(string.clone())));
    group.finish();

    let mut group = c.benchmark_group("from_str");
    group.bench_function("buffer", |b| b.iter(|| Buffer::from(&*string)));
    group.bench_function("crop", |b| b.iter(|| CropRope::from(&*string)));
    group.bench_function("jumprope", |b| b.iter(|| JumpRope::from(&*string)));
    group.bench_function("ropey", |b| b.iter(|| RopeyRope::from(&*string)));
    group.finish();
}

fn bench_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("save");

    let size = usize::pow(2, 20);
    let string = "à".repeat(size / 2);
    assert_eq!(string.len(), size);

    let x = Buffer::from(string.clone());
    group.bench_function("buffer", |b| b.iter(|| ToString::to_string(&x)));
    let x = CropRope::from(string.clone());
    group.bench_function("crop", |b| b.iter(|| ToString::to_string(&x)));
    let x = JumpRope::from(string.clone());
    group.bench_function("jumprope", |b| b.iter(|| JumpRope::to_string(&x)));
    let x = RopeyRope::from(string.clone());
    group.bench_function("ropey", |b| b.iter(|| ToString::to_string(&x)));
    group.finish();
}

#[allow(unused)]
fn bench_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("append");

    group.bench_function("buffer", append::<Buffer>);
    group.bench_function("crop", append::<CropRope>);
    group.bench_function("jumprope", append::<JumpRope>);
    group.bench_function("ropey", append::<RopeyRope>);
    group.finish();
}

fn bench_multiple_cursors_smart(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_cursors_smart");
    use BenchmarkId as id;

    for step in [
        10, 50, 100, 250, 500, 1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000,
    ] {
        // size, cursors, width
        let cursors = 1000;
        let width = 10;
        let size = 10000 * 1000;
        let params = &(size, cursors, step, width);
        let d = &format!("cursors_{cursors}/step_{step}");

        group.bench_function(id::new("naive", d), |b| {
            multiple_cursors_impl::<Buffer>(b, params)
        });
        group.bench_function(id::new("smart", d), |b| {
            multiple_cursors_smart::<Buffer>(b, params)
        });
    }

    group.finish();
}

fn bench_multiple_cursors(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_cursors");
    use BenchmarkId as id;

    for (pow, sample) in &[
        (15, 100),
        (16, 90),
        (17, 80),
        (18, 70),
        (19, 60),
        (20, 50),
        (21, 20),
        (22, 10),
        (23, 10),
    ] {
        // size, cursors, width
        group.sample_size(*sample);
        let size = usize::pow(2, *pow);
        let cursors = 1000;
        let step = 1000;
        let width = 20;
        if cursors * step >= size {
            continue;
        }
        let params = &(size, cursors, step, width);
        let d = &format!("2^{pow}/{size}/{cursors}/{step}");

        // group.bench_function(id::new("buffer", d), |b| {
        //     multiple_cursors_impl::<Buffer>(b, params)
        // });
        group.bench_function(id::new("buffer", d), |b| {
            multiple_cursors_smart::<Buffer>(b, params)
        });
        group.bench_function(id::new("jumprope", d), |b| {
            multiple_cursors_smart::<JumpRope>(b, params)
        });
    }

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
        "{}/../benchmark_data/{name}.json.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    load_testing_data(&filename)
}

fn load_named_ascii_data(name: &str) -> TestData {
    let filename = format!(
        "{}/../benchmark_data/ascii_only/{name}.json.gz",
        env!("CARGO_MANIFEST_DIR"),
    );
    load_testing_data(&filename)
}

const DATASETS: &[&str] = &[
    "automerge-paper",
    "rustcode",
    "sveltecomponent",
    "seph-blog1",
    "friendsforever_flat",
];

fn realworld_unicode(c: &mut Criterion) {
    for name in DATASETS {
        let mut group = c.benchmark_group("realworld_unicode");
        let test_data = load_named_data(name);

        fn x<R: Rope>(group: &mut BenchmarkGroup<WallTime>, name: &str, test_data: &TestData) {
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

        x::<Buffer>(&mut group, name, &test_data);
        x::<JumpRope>(&mut group, name, &test_data);
        x::<RopeyRope>(&mut group, name, &test_data);
        // doesn't support unicode indexing
        // x::<Crop>(&mut group, name, &test_data);
        group.finish();
    }
}

fn realworld_ascii(c: &mut Criterion) {
    for name in DATASETS {
        let mut group = c.benchmark_group("realworld_ascii");
        let test_data = load_named_ascii_data(name);

        fn x<R: Rope>(group: &mut BenchmarkGroup<WallTime>, name: &str, test_data: &TestData) {
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

        x::<Buffer>(&mut group, name, &test_data);
        x::<CropRope>(&mut group, name, &test_data);
        x::<JumpRope>(&mut group, name, &test_data);
        x::<RopeyRope>(&mut group, name, &test_data);
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_create,
    bench_save,
    bench_append,
    bench_multiple_cursors_smart,
    bench_multiple_cursors,
    bench_search_linewise,
    bench_search_full,
    bench_build_string,
    bench_ins_random,
    bench_stable_ins_del,
    realworld_unicode,
    realworld_ascii,
);
criterion_main!(benches);

// fn main() {
//     report_space_overhead()
// }
