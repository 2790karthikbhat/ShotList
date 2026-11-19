// benches/thumbnail_bench.rs
// Benchmark the thumbnail decode + resize pipeline.
// Run with: cargo bench --bench thumbnail_bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use image::{DynamicImage, ImageBuffer, Rgb};

fn bench_focus_score(c: &mut Criterion) {
    // Synthesise a 4000×3000 "image" (equivalent to a 12MP photo).
    let img = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(4000, 3000, Rgb([128u8, 128, 128])));

    let mut group = c.benchmark_group("focus_score");
    group.throughput(Throughput::Elements(1));
    group.bench_function("laplacian_4000x3000", |b| {
        b.iter(|| shotlist::ai::local::laplacian_focus_score(black_box(&img)))
    });
    group.finish();
}

criterion_group!(benches, bench_focus_score);
criterion_main!(benches);
