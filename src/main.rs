use crdt_testdata::*;
use criterion::*;
mod rope;
use self::rope::*;
use crop::Rope as Crop;
use get_size::GetSize;
use jumprope::JumpRope;
use regex::Regex;
use ropey::Rope as Ropey;
use std::any::type_name;
use std::{
    borrow::Cow,
    fs::File,
    io::{BufReader, Read},
};
use text_buffer::Buffer;

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

impl Rope for Ropey {
    const NAME: &'static str = "Ropey";

    #[inline(always)]
    fn new() -> Self {
        Ropey::new()
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
            let cow: Cow<str> = (*line).into();
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

    fn line_search_cursor(&self, re: &regex_cursor::engines::meta::Regex) -> usize {
        use regex_cursor::{Input, RopeyCursor};
        let input = Input::new(RopeyCursor::new(self.slice(..)));
        re.find(input).map(|m| m.start()).unwrap_or_else(|| self.byte_len())
    }

    fn byte_len(&self) -> usize {
        self.len_bytes()
    }
}

impl Rope for Crop {
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
                let string: String = buf.iter().copied().collect();
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

    #[inline]
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

use criterion::measurement::WallTime;

fn gen_realworld_text(size: usize) -> String {
    let filename = format!("{}/data/realworld.txt", env!("CARGO_MANIFEST_DIR"));
    // read the file into a string
    let file = File::open(filename).unwrap();
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    let repeat = size / contents.len();
    let mut string = contents.repeat(repeat);
    string.push_str(&contents[..size % contents.len()]);
    string
}

fn append<R: Rope + for<'a> From<&'a str>>(b: &mut Bencher) {
    let mut r = R::new();
    let target = usize::pow(2, 20);
    let text = "ropes";
    let count = target / text.len();
    let mut len = 0;
    b.iter(|| {
        for _ in 0..count {
            r.insert_at(len, text);
            len += text.len();
        }
        len -= text.len();
        for _ in 0..count {
            r.del_at(0, text.len());
        }
    });
}

fn is_even(x: usize) -> bool {
    x % 2 == 0
}

fn mc_smart<R: Rope + for<'a> From<&'a str>>(
    b: &mut Bencher,
    params: &(usize, usize, usize, usize),
) {
    let size = params.0;
    let cursors = params.1;
    let step = params.2;
    let width = params.3;
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

fn mc_naive<R: Rope + for<'a> From<&'a str>>(
    b: &mut Bencher,
    params: &(usize, usize, usize, usize),
) {
    let size = params.0;
    let cursors = params.1;
    let step = params.2;
    let width = params.3;
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
    let contents = gen_realworld_text(usize::pow(2, 20));

    let r = R::from(contents);

    let re = Regex::new(r"foo(bar|baz)fob").unwrap();
    b.iter(|| {
        black_box(r.line_search(&re));
    });
}

fn search_cursor<R: Rope + for<'a> From<&'a str>>(b: &mut Bencher, text: &str) {
    let len = text.len();
    let r = R::from(text);
    let re = regex_cursor::engines::meta::Regex::new(r"foo(bar|baz)fob").unwrap();
    b.iter(|| {
        let idx = r.line_search_cursor(&re);
        assert_eq!(idx, len);
        black_box(idx);
    });
}

fn search_full<R: Rope + for<'a> From<&'a str>>(b: &mut Bencher, text: &str) {
    let len = text.len();
    let container = R::from(text);
    let re = Regex::new(r"foo(bar|baz)fob").unwrap();
    b.iter(|| {
        let idx = container.full_search(&re);
        assert_eq!(idx, len);
        black_box(idx);
    });
}

fn move_gap<R: Rope + for<'a> From<&'a str>>(b: &mut Bencher, text: &str) {
    let len = text.len();
    let mut container = R::from(text);
    let mut forward = true;
    b.iter(|| {
        if forward {
            container.insert_at(len / 2, "b");
            container.del_at(len / 2, 1);
        } else {
            container.insert_at(0, "b");
            container.del_at(0, 1);
        }
        forward = !forward;
    });
}

fn build_string<R: Rope + From<String>>(b: &mut Bencher, size: &usize) {
    let contents = gen_realworld_text(*size);
    let r = R::from(contents);

    b.iter(|| {
        black_box(r.get_string());
    });
}

fn build_from_edits<R: Rope>(size: usize) -> R {
    let test_data = load_named_data("automerge-paper");
    let len = test_data.end_content.len();

    let mut r = R::new();
    for _ in 0..(size / len) {
        for txn in &test_data.txns {
            for TestPatch(pos, del, ins) in &txn.patches {
                r.edit_at(*pos, *del, ins);
            }
        }
    }
    r
}

fn space_overhead<R: for<'a> From<&'a str> + GetSize>(size: usize) {
    let string = gen_realworld_text(size);
    let len = string.len();
    let rope = R::from(&*string);
    let size = GetSize::get_size(&rope);
    let overhead = size - len;
    let percent_overhead = (overhead as f64 / len as f64) * 100.0;
    println!("{}: {:.2}%", type_name::<R>(), percent_overhead);
}

fn space_overhead_edits<R: Rope + GetSize>(size: usize) {
    let r: R = build_from_edits(size);
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
    // space_overhead::<Ropey>(size);
    space_overhead::<Crop>(size);
}

#[allow(unused)]
fn report_space_overhead_edits() {
    let size = usize::pow(2, 20);
    space_overhead_edits::<Buffer>(size);
    space_overhead_edits::<JumpRope>(size);
    // space_overhead_edits::<Ropey>(size);
    space_overhead_edits::<Crop>(size);
}

fn bench_create(c: &mut Criterion) {
    let mut group = c.benchmark_group("from_string");
    group.sample_size(10);

    let size = usize::pow(2, 30);
    let string = "à".repeat(size / 2);
    assert_eq!(string.len(), size);

    group.bench_function("clone", |b| b.iter(|| string.clone()));
    group.bench_function("buffer", |b| b.iter(|| Buffer::from(string.clone())));
    group.bench_function("crop", |b| b.iter(|| Crop::from(string.clone())));
    group.bench_function("jumprope", |b| b.iter(|| JumpRope::from(string.clone())));
    group.bench_function("ropey", |b| b.iter(|| Ropey::from(string.clone())));
    group.finish();

    let mut group = c.benchmark_group("from_str");
    group.bench_function("buffer", |b| b.iter(|| Buffer::from(&*string)));
    group.bench_function("crop", |b| b.iter(|| Crop::from(&*string)));
    group.bench_function("jumprope", |b| b.iter(|| JumpRope::from(&*string)));
    group.bench_function("ropey", |b| b.iter(|| Ropey::from(&*string)));
    group.finish();
}

fn bench_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("save");

    let size = usize::pow(2, 30);
    let string = "à".repeat(size / 2);
    assert_eq!(string.len(), size);

    let x = Buffer::from(string.clone());
    group.bench_function("buffer", |b| b.iter(|| ToString::to_string(&x)));
    let x = Crop::from(string.clone());
    group.bench_function("crop", |b| b.iter(|| ToString::to_string(&x)));
    let x = JumpRope::from(string.clone());
    group.bench_function("jumprope", |b| b.iter(|| JumpRope::to_string(&x)));
    let x = Ropey::from(string.clone());
    group.bench_function("ropey", |b| b.iter(|| ToString::to_string(&x)));
    group.finish();
}

fn bench_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("append");

    group.bench_function("buffer", append::<Buffer>);
    group.bench_function("crop", append::<Crop>);
    group.bench_function("jumprope", append::<JumpRope>);
    group.bench_function("ropey", append::<Ropey>);
    group.finish();
}

fn bench_mc_smart(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_smart");
    use BenchmarkId as id;

    for step in [
        10, 50, 100, 250, 500, 1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000,
    ] {
        let cursors = 1000;
        let width = 10;
        let size = 10000 * 1000;
        let params = &(size, cursors, step, width);
        let d = &format!("cursors_{cursors}/step_{step}");

        group.bench_function(id::new("naive", d), |b| mc_naive::<Buffer>(b, params));
        group.bench_function(id::new("smart", d), |b| mc_smart::<Buffer>(b, params));
    }

    group.finish();
}

fn bench_mc_cursors(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_cursor_count");
    use BenchmarkId as id;

    let max = 10000;
    let step = 100;
    let width = 10;
    let size = max * step;
    for cursors in [10, 100, 250, 500, 1000, 2000, 5000, 7000, max] {
        let params = &(size, cursors, step, width);
        group.bench_function(id::new("buffer", cursors), |b| {
            mc_smart::<Buffer>(b, params)
        });
        group.bench_function(id::new("crop", cursors), |b| mc_smart::<Crop>(b, params));
        group.bench_function(id::new("jumprope", cursors), |b| {
            mc_smart::<JumpRope>(b, params)
        });
        group.bench_function(id::new("ropey", cursors), |b| mc_smart::<Ropey>(b, params));
    }

    group.finish();
}

fn bench_mc_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_cursor_size");
    use BenchmarkId as id;

    let max = 6000;
    let cursors = 100;
    let width = 10;
    let size = max * cursors;
    for step in [
        10, 100, 250, 500, 750, 1000, 1250, 1500, 2000, 2500, 3000, 4000, 4250, 4500, 5000, max,
    ] {
        let params = &(size, cursors, step, width);
        group.bench_function(id::new("buffer", step), |b| mc_smart::<Buffer>(b, params));
        group.bench_function(id::new("crop", step), |b| mc_smart::<Crop>(b, params));
        group.bench_function(id::new("jumprope", step), |b| {
            mc_smart::<JumpRope>(b, params)
        });
        group.bench_function(id::new("ropey", step), |b| mc_smart::<Ropey>(b, params));
    }

    group.finish();
}

fn bench_search_linewise(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_linewise");
    group.sample_size(50);

    group.bench_function("buffer", search_linewise::<Buffer>);
    group.bench_function("jumprope", search_linewise::<JumpRope>);
    group.bench_function("ropey", search_linewise::<Ropey>);
    group.bench_function("crop", search_linewise::<Crop>);
    group.finish();
}

fn bench_search_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_full");
    use BenchmarkId as id;
    let step = usize::pow(2, 27);
    let small = usize::pow(2, 20);
    for (size, sample) in [
        (small, 100),
        (step, 100),
        (step * 2, 75),
        (step * 3, 60),
        (step * 4, 60),
        (step * 5, 50),
        (step * 6, 50),
        (step * 7, 50),
        (step * 8, 50),
    ] {
        let base = gen_realworld_text(size);
        let text = base.as_str();
        group.sample_size(sample);
        group.bench_function(id::new("move_gap", size), |b| move_gap::<Buffer>(b, text));
        group.bench_function(id::new("buffer", size), |b| search_full::<Buffer>(b, text));
        group.bench_function(id::new("crop", size), |b| search_full::<Crop>(b, text));
        group.bench_function(id::new("jumprope", size), |b| {
            search_full::<JumpRope>(b, text)
        });
        group.bench_function(id::new("ropey", size), |b| search_full::<Ropey>(b, text));
        group.bench_function(id::new("ropey_cursor", size), |b| search_cursor::<Ropey>(b, text));
    }
    group.finish();
}

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
        group.bench_with_input(id, size, build_string::<Ropey>);
        let id = BenchmarkId::new("crop", size);
        group.bench_with_input(id, size, build_string::<Crop>);
    }
    group.finish();
}

fn load_named_data(name: &str) -> TestData {
    let filename = format!(
        "{}/benchmark_data/{name}.json.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    load_testing_data(&filename)
}

fn load_named_ascii_data(name: &str) -> TestData {
    let filename = format!(
        "{}/benchmark_data/ascii_only/{name}.json.gz",
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

        // x::<Buffer>(&mut group, name, &test_data);
        x::<JumpRope>(&mut group, name, &test_data);
        x::<Ropey>(&mut group, name, &test_data);
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
        x::<Crop>(&mut group, name, &test_data);
        x::<JumpRope>(&mut group, name, &test_data);
        x::<Ropey>(&mut group, name, &test_data);
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_create,
    bench_save,
    bench_append,
    bench_mc_smart,
    bench_mc_cursors,
    bench_mc_size,
    bench_search_linewise,
    bench_search_full,
    bench_build_string,
    realworld_unicode,
    realworld_ascii,
);
criterion_main!(benches);

// fn main() {
//     report_space_overhead();
//     report_space_overhead_edits();
// }
