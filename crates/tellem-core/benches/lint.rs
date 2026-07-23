use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use tellem_core::{Engine, Pack};

fn bench_lint(c: &mut Criterion) {
    let engine = Engine::from_packs(&[Pack::parse(tellem_core::BASE_PACK).unwrap()]).unwrap();
    let para = "It's worth noting that our meticulous team leverages a myriad of \
                cutting-edge frameworks to showcase seamless, transformative results. \
                The parser handles UTF-8 input and the tests pass on CI without any \
                manual steps, which keeps the release process short and boring. ";
    let text = para.repeat(4000); // ~1.2 MB of prose
    let mut g = c.benchmark_group("lint");
    g.throughput(Throughput::Bytes(text.len() as u64));
    g.bench_function("base-pack-1mb", |b| {
        b.iter(|| engine.lint(std::hint::black_box(&text)))
    });
    g.finish();
}

criterion_group!(benches, bench_lint);
criterion_main!(benches);
